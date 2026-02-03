use crate::models::neural::CommandResult;
use crate::services::airlock::AirlockService;
use crate::services::neural_service::NeuralService;
use crate::services::skill_executor::SkillExecutor;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};
use tokio::time::sleep;

const POLL_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Clone)]
pub struct CommandPoller {
    neural_service: NeuralService,
    skill_executor: Arc<SkillExecutor>,
    is_running: Arc<Mutex<bool>>,
    airlock_service: Arc<RwLock<Option<AirlockService>>>,
}

impl CommandPoller {
    pub fn new(neural_service: NeuralService, skill_executor: Arc<SkillExecutor>) -> Self {
        Self {
            neural_service,
            skill_executor,
            is_running: Arc::new(Mutex::new(false)),
            airlock_service: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn set_airlock_service(&self, service: AirlockService) {
        let mut lock = self.airlock_service.write().await;
        *lock = Some(service);
    }

    pub async fn start(&self) {
        let mut running = self.is_running.lock().await;
        if *running {
            return;
        }
        *running = true;
        drop(running);

        let poller = self.clone();
        tokio::spawn(async move {
            println!("[CommandPoller] Started polling loop via NeuralService");

            while *poller.is_running.lock().await {
                if let Err(e) = poller.poll_and_execute().await {
                    eprintln!("[CommandPoller] Error: {}", e);
                }
                sleep(POLL_INTERVAL).await;
            }

            println!("[CommandPoller] Stopped polling loop");
        });
    }

    #[allow(dead_code)]
    pub async fn stop(&self) {
        let mut running = self.is_running.lock().await;
        *running = false;
    }

    async fn poll_and_execute(&self) -> Result<(), Box<dyn std::error::Error>> {
        // 1. Send heartbeat and get commands via NeuralService
        if !self.neural_service.has_credentials().await {
            return Ok(()); // Silently skip if not authenticated
        }

        // Check if node is registered (has node_id)
        if !self.neural_service.is_registered().await {
            // Node not registered yet - silently skip to avoid log spam
            return Ok(());
        }

        let pending_commands_result = self
            .neural_service
            .heartbeat(crate::models::neural::DesktopNodeStatus::Online)
            .await;

        let commands = match pending_commands_result {
            Ok(cmds) => cmds,
            Err(e) => {
                // Only log if it's not a "Node not registered" error (already handled above)
                if !e.contains("Node not registered") {
                    eprintln!("[CommandPoller] Heartbeat error: {}", e);
                }
                return Ok(()); // Don't propagate as error to avoid log spam
            }
        };

        // 2. Process commands if any
        for command in commands {
            println!("[CommandPoller] Received command: {:?}", command.id);

            // AIRLOCK CHECK
            let allowed = {
                let lock = self.airlock_service.read().await;
                if let Some(airlock) = &*lock {
                    match airlock.check_permission(&command).await {
                        Ok(true) => true,
                        Ok(false) => false,
                        Err(e) => {
                            eprintln!(
                                "[CommandPoller] Airlock error for command {}: {}",
                                command.id, e
                            );
                            false
                        }
                    }
                } else {
                    eprintln!(
                        "[CommandPoller] Airlock service not initialized! Rejecting command {}",
                        command.id
                    );
                    false
                }
            };

            if !allowed {
                println!(
                    "[CommandPoller] Command {} REJECTED by Airlock or User",
                    command.id
                );
                let _ = self
                    .neural_service
                    .complete_command(
                        &command.id,
                        CommandResult {
                            success: false,
                            output: None,
                            error: Some("Rejected by Airlock/User".into()),
                            exit_code: Some(1),
                        },
                    )
                    .await;
                continue;
            }

            // Notify start
            let _ = self.neural_service.start_command(&command.id).await;

            // Execute
            let result = self.skill_executor.execute(&command).await;

            // Report result
            let _ = self
                .neural_service
                .complete_command(&command.id, result)
                .await;
        }

        Ok(())
    }
}
