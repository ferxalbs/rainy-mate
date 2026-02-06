use mac_address::get_mac_address;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct NodeAuthenticator {
    cached_fingerprint: Arc<Mutex<Option<String>>>,
}

impl NodeAuthenticator {
    pub fn new() -> Self {
        Self {
            cached_fingerprint: Arc::new(Mutex::new(None)),
        }
    }

    /// Generates a SHA256 hash of the device's MAC address
    pub async fn get_device_fingerprint(&self) -> Result<String, String> {
        let mut cache = self.cached_fingerprint.lock().await;

        if let Some(fingerprint) = &*cache {
            return Ok(fingerprint.clone());
        }

        match get_mac_address() {
            Ok(Some(mac)) => {
                let mac_string = mac.to_string();
                let mut hasher = Sha256::new();
                hasher.update(mac_string.as_bytes());
                let result = hasher.finalize();
                let fingerprint = hex::encode(result);

                *cache = Some(fingerprint.clone());
                Ok(fingerprint)
            }
            Ok(None) => Err("No MAC address found".to_string()),
            Err(e) => Err(format!("Failed to get MAC address: {}", e)),
        }
    }
}
