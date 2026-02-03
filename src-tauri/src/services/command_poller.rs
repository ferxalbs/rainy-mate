use crate::services::neural_service::NeuralService;
use crate::services::skill_executor::SkillExecutor;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::sleep;

const POLL_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Clone)]
pub struct CommandPoller {
    neural_service: NeuralService,
    skill_executor: Arc<SkillExecutor>,
    is_running: Arc<Mutex<bool>>,
}

impl CommandPoller {
    pub fn new(neural_service: NeuralService, skill_executor: Arc<SkillExecutor>) -> Self {
        Self {
            neural_service,
            skill_executor,
            is_running: Arc::new(Mutex::new(false)),
        }
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
        // NeuralService handles auth and node_id internally

        // We only poll if we are authenticated/registered
        if !self.neural_service.has_credentials().await {
            // Not authenticated yet, skip polling
            return Ok(());
        }

        let pending_commands_result = self
            .neural_service
            .heartbeat(crate::models::neural::DesktopNodeStatus::Online)
            .await;

        let commands = match pending_commands_result {
            Ok(cmds) => cmds,
            Err(e) => {
                // Return error effectively
                return Err(format!("Heartbeat failed: {}", e).into());
            }
        };

        // 2. Process commands if any
        for command in commands {
            println!("[CommandPoller] Received command: {:?}", command.id);

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
