use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, Mutex};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tracing::{error, info, warn};
use url::Url;

#[derive(Clone)]
pub struct SocketClient {
    url: String,
    tx: broadcast::Sender<SocketMessage>,
    connected: Arc<Mutex<bool>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocketMessage {
    pub event: String,
    pub payload: serde_json::Value,
}

impl SocketClient {
    pub fn new(url: String) -> Self {
        let (tx, _) = broadcast::channel(100);
        Self {
            url,
            tx,
            connected: Arc::new(Mutex::new(false)),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<SocketMessage> {
        self.tx.subscribe()
    }

    pub async fn connect(&self) {
        let url = Url::parse(&self.url).expect("Invalid WebSocket URL");
        let tx = self.tx.clone();
        let connected = self.connected.clone();
        let url_clone = self.url.clone();

        tokio::spawn(async move {
            loop {
                info!("Attempting to connect to WebSocket: {}", url_clone);
                match connect_async(url.to_string()).await {
                    Ok((ws_stream, _)) => {
                        info!("Connected to WebSocket: {}", url_clone);
                        {
                            let mut lock = connected.lock().await;
                            *lock = true;
                        }

                        let (_write, mut read) = ws_stream.split();

                        // Keep connection alive logic here if needed

                        while let Some(msg) = read.next().await {
                            match msg {
                                Ok(Message::Text(text)) => {
                                    if let Ok(socket_msg) =
                                        serde_json::from_str::<SocketMessage>(&text)
                                    {
                                        if let Err(e) = tx.send(socket_msg) {
                                            warn!("Failed to broadcast message: {}", e);
                                        }
                                    } else {
                                        warn!("Received invalid JSON: {}", text);
                                    }
                                }
                                Ok(Message::Binary(_)) => {
                                    // Handle binary if needed (MessagePack later)
                                }
                                Ok(Message::Close(_)) => {
                                    warn!("WebSocket connection closed");
                                    break;
                                }
                                Err(e) => {
                                    error!("WebSocket error: {}", e);
                                    break;
                                }
                                _ => {}
                            }
                        }

                        {
                            let mut lock = connected.lock().await;
                            *lock = false;
                        }
                        warn!("Disconnected from WebSocket. Reconnecting in 5s...");
                    }
                    Err(e) => {
                        error!("Failed to connect: {}. Retrying in 5s...", e);
                    }
                }
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        });
    }

    // @RESERVED for Bi-directional communication
    #[allow(dead_code)]
    pub async fn send(&self, _message: SocketMessage) -> Result<(), String> {
        // This is a simplified send. In a real scenario, we'd need a channel to the write task.
        // For now, we focus on receiving commands (Cloud -> Desktop).
        // Sending Logic requires a separate mpsc channel to the writer loop.
        // TODO: Implement full bi-directional communication.
        Ok(())
    }
}
