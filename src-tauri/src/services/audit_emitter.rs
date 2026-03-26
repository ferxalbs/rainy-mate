use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FleetAuditEvent {
    pub action_type: String,
    pub outcome: String,
    pub agent_id: Option<String>,
    pub tool_name: Option<String>,
    pub airlock_level: Option<u8>,
    pub payload_json: Option<String>,
}

#[derive(Clone, Default)]
pub struct AuditEmitter {
    queue: Arc<Mutex<Vec<FleetAuditEvent>>>,
}

impl AuditEmitter {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn enqueue(&self, event: FleetAuditEvent) {
        let mut lock = self.queue.lock().await;
        lock.push(event);
    }

    pub async fn flush(
        &self,
        atm_client: &crate::services::atm_client::ATMClient,
        node_id: &str,
    ) -> Result<usize, String> {
        let events = {
            let mut lock = self.queue.lock().await;
            if lock.is_empty() {
                return Ok(0);
            }
            let drained = lock.clone();
            lock.clear();
            drained
        };

        match atm_client
            .send_fleet_audit_events(node_id.to_string(), events.clone())
            .await
        {
            Ok(written) => Ok(written),
            Err(e) => {
                let mut lock = self.queue.lock().await;
                let mut restored = events;
                restored.append(&mut *lock);
                *lock = restored;
                Err(e)
            }
        }
    }
}
