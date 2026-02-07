use crate::services::atm_client::ATMClient;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;
use tokio::time::sleep;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message;

#[derive(Clone)]
pub struct CloudBridge {
    atm_client: Arc<ATMClient>,
    app_handle: AppHandle,
    is_connected: Arc<Mutex<bool>>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", content = "payload")]
enum ServerMessage {
    #[serde(rename = "HEARTBEAT_ACK")]
    HeartbeatAck,
    #[serde(rename = "DEPLOY_AGENT")]
    DeployAgent {
        spec_id: String,
        download_url: String,
    },
    #[serde(rename = "ERROR")]
    Error { message: String },
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", content = "payload")]
enum ClientMessage {
    #[serde(rename = "AUTH")]
    Auth { api_key: String },
    #[serde(rename = "HEARTBEAT")]
    Heartbeat,
}

impl CloudBridge {
    pub fn new(atm_client: Arc<ATMClient>, app_handle: AppHandle) -> Self {
        Self {
            atm_client,
            app_handle,
            is_connected: Arc::new(Mutex::new(false)),
        }
    }

    pub fn start(&self) {
        let bridge = self.clone();
        tokio::spawn(async move {
            bridge.run_loop().await;
        });
    }

    async fn run_loop(&self) {
        loop {
            // 1. Wait for credentials
            if !self.atm_client.has_credentials().await {
                sleep(Duration::from_secs(5)).await;
                continue;
            }

            // 2. Connect
            if let Err(e) = self.connect().await {
                eprintln!("[CloudBridge] Connection failed: {}. Retrying in 10s...", e);
                sleep(Duration::from_secs(10)).await;
            }
        }
    }

    async fn connect(&self) -> Result<(), String> {
        let state = self.atm_client.get_state().await; // Assumes accessor
        let api_key = state.api_key.ok_or("No API Key")?;

        // Convert HTTP URL to WS URL
        let ws_url = state.base_url.replace("http", "ws") + "/ws";

        println!("[CloudBridge] Connecting to {}...", ws_url);

        let (ws_stream, _) = connect_async(&ws_url)
            .await
            .map_err(|e| format!("WS Connect error: {}", e))?;

        println!("[CloudBridge] Connected!");
        *self.is_connected.lock().await = true;

        // Split stream
        let (mut write, mut read) = ws_stream.split();

        // Authenticate
        let auth_msg = serde_json::to_string(&ClientMessage::Auth {
            api_key: api_key.clone(),
        })
        .map_err(|e| e.to_string())?;
        write
            .send(Message::Text(auth_msg.into()))
            .await
            .map_err(|e| e.to_string())?;

        // Heartbeat Loop
        let write_clone = Arc::new(Mutex::new(write));
        let write_heartbeat = write_clone.clone();

        let heartbeat_task = tokio::spawn(async move {
            loop {
                sleep(Duration::from_secs(30)).await;
                let msg = serde_json::to_string(&ClientMessage::Heartbeat).unwrap();
                let mut w = write_heartbeat.lock().await;
                if let Err(e) = w.send(Message::Text(msg.into())).await {
                    eprintln!("[CloudBridge] Heartbeat failed: {}", e);
                    break;
                }
            }
        });

        // Read Loop
        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    self.handle_message(&text.to_string()).await;
                }
                Ok(Message::Close(_)) => {
                    println!("[CloudBridge] Connection closed by server");
                    break;
                }
                Err(e) => {
                    eprintln!("[CloudBridge] Stream error: {}", e);
                    break;
                }
                _ => {}
            }
        }

        *self.is_connected.lock().await = false;
        heartbeat_task.abort();
        Ok(())
    }

    async fn handle_message(&self, text: &str) {
        match serde_json::from_str::<ServerMessage>(text) {
            Ok(msg) => match msg {
                ServerMessage::HeartbeatAck => {
                    // console_debug!("Heartbeat ACK");
                }
                ServerMessage::DeployAgent {
                    spec_id,
                    download_url,
                } => {
                    println!("[CloudBridge] Received deploy instruction for {}", spec_id);
                    let _ = self.app_handle.emit(
                        "cloud:deploy-request",
                        serde_json::json!({
                            "specId": spec_id,
                            "downloadUrl": download_url
                        }),
                    );
                }
                ServerMessage::Error { message } => {
                    eprintln!("[CloudBridge] Server Error: {}", message);
                }
            },
            Err(e) => eprintln!("[CloudBridge] Parse error: {} | Body: {}", e, text),
        }
    }
}
