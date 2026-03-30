use crate::ai::keychain::KeychainManager;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use tokio::time::sleep;

const TRANSIENT_KEYWORDS: &[&str] = &[
    "interaction not allowed",
    "user interaction is not allowed",
    "temporarily unavailable",
    "timeout",
    "timed out",
    "try again",
    "busy",
    "deadlock",
];
const NOT_FOUND_KEYWORDS: &[&str] = &["itemnotfound", "not found", "could not be found"];
const PERMISSION_KEYWORDS: &[&str] = &["auth", "permission", "denied"];
const MAX_ATTEMPTS: u32 = 4;
const BASE_BACKOFF_MS: u64 = 40;
const MAX_BACKOFF_MS: u64 = 300;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeychainAccessErrorKind {
    TransientBusy,
    PermissionDenied,
    DataCorrupt,
    Other,
}

#[derive(Debug, Clone)]
pub struct KeychainAccessError {
    pub kind: KeychainAccessErrorKind,
    pub message: String,
}

impl std::fmt::Display for KeychainAccessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for KeychainAccessError {}

#[derive(Debug, Clone, Default)]
pub struct StartupCredentialSnapshot {
    pub atm_admin_key: Option<String>,
    pub owner_auth_bundle_raw: Option<String>,
    pub neural_platform_key: Option<String>,
    pub neural_user_api_key: Option<String>,
    pub neural_workspace_id: Option<String>,
    pub provider_keys: HashMap<String, Option<String>>,
}

#[derive(Debug, Clone, Default)]
pub struct KeychainAccessService {
    cache: Arc<Mutex<HashMap<String, Option<String>>>>,
}

type KeychainOpResult<T> = Result<T, KeychainAccessError>;

fn global_gate() -> &'static Mutex<()> {
    static GATE: OnceLock<Mutex<()>> = OnceLock::new();
    GATE.get_or_init(|| Mutex::new(()))
}

impl KeychainAccessService {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn get(&self, key: &str) -> KeychainOpResult<Option<String>> {
        if let Some(cached) = self
            .cache
            .lock()
            .map_err(|_| Self::poisoned())?
            .get(key)
            .cloned()
        {
            return Ok(cached);
        }

        let key_owned = key.to_string();
        let value = self
            .with_retry(&format!("get:{key}"), move || {
                let key = key_owned.clone();
                async move {
                    Self::run_blocking(move || {
                        let manager = KeychainManager::new();
                        manager.get_key(&key).map_err(Self::classify_error)
                    })
                    .await
                }
            })
            .await?;

        self.cache
            .lock()
            .map_err(|_| Self::poisoned())?
            .insert(key.to_string(), value.clone());
        Ok(value)
    }

    pub fn get_blocking(&self, key: &str) -> KeychainOpResult<Option<String>> {
        if let Some(cached) = self
            .cache
            .lock()
            .map_err(|_| Self::poisoned())?
            .get(key)
            .cloned()
        {
            return Ok(cached);
        }

        let value = self.with_retry_blocking(&format!("get:{key}"), || {
            let manager = KeychainManager::new();
            manager.get_key(key).map_err(Self::classify_error)
        })?;

        self.cache
            .lock()
            .map_err(|_| Self::poisoned())?
            .insert(key.to_string(), value.clone());
        Ok(value)
    }

    pub async fn set(&self, key: &str, value: &str) -> KeychainOpResult<()> {
        let key_owned = key.to_string();
        let value_owned = value.to_string();
        self.with_retry(&format!("set:{key}"), move || {
            let key = key_owned.clone();
            let value = value_owned.clone();
            async move {
                Self::run_blocking(move || {
                    let manager = KeychainManager::new();
                    manager
                        .store_key(&key, &value)
                        .map_err(Self::classify_error)
                })
                .await
            }
        })
        .await?;

        self.cache
            .lock()
            .map_err(|_| Self::poisoned())?
            .insert(key.to_string(), Some(value.to_string()));
        Ok(())
    }

    pub fn set_blocking(&self, key: &str, value: &str) -> KeychainOpResult<()> {
        self.with_retry_blocking(&format!("set:{key}"), || {
            let manager = KeychainManager::new();
            manager.store_key(key, value).map_err(Self::classify_error)
        })?;

        self.cache
            .lock()
            .map_err(|_| Self::poisoned())?
            .insert(key.to_string(), Some(value.to_string()));
        Ok(())
    }

    pub async fn delete(&self, key: &str) -> KeychainOpResult<()> {
        let key_owned = key.to_string();
        self.with_retry(&format!("delete:{key}"), move || {
            let key = key_owned.clone();
            async move {
                Self::run_blocking(move || {
                    let manager = KeychainManager::new();
                    manager.delete_key(&key).map_err(Self::classify_error)
                })
                .await
            }
        })
        .await?;

        self.cache
            .lock()
            .map_err(|_| Self::poisoned())?
            .insert(key.to_string(), None);
        Ok(())
    }

    pub async fn get_many(
        &self,
        keys: &[&str],
    ) -> KeychainOpResult<HashMap<String, Option<String>>> {
        let owned = keys
            .iter()
            .map(|key| (*key).to_string())
            .collect::<Vec<_>>();
        let values = self
            .with_retry("get_many", move || {
                let owned = owned.clone();
                async move {
                    Self::run_blocking(move || {
                        let manager = KeychainManager::new();
                        let mut out = HashMap::with_capacity(owned.len());
                        for key in owned {
                            let value = manager.get_key(&key).map_err(Self::classify_error)?;
                            out.insert(key, value);
                        }
                        Ok(out)
                    })
                    .await
                }
            })
            .await?;

        let mut cache = self.cache.lock().map_err(|_| Self::poisoned())?;
        for (key, value) in &values {
            cache.insert(key.clone(), value.clone());
        }
        Ok(values)
    }

    pub fn get_many_blocking(
        &self,
        keys: &[&str],
    ) -> KeychainOpResult<HashMap<String, Option<String>>> {
        let values = self.with_retry_blocking("get_many", || {
            let manager = KeychainManager::new();
            let mut out = HashMap::with_capacity(keys.len());
            for key in keys {
                let value = manager.get_key(key).map_err(Self::classify_error)?;
                out.insert((*key).to_string(), value);
            }
            Ok(out)
        })?;

        let mut cache = self.cache.lock().map_err(|_| Self::poisoned())?;
        for (key, value) in &values {
            cache.insert(key.clone(), value.clone());
        }
        Ok(values)
    }

    pub async fn load_startup_snapshot(&self) -> KeychainOpResult<StartupCredentialSnapshot> {
        let keys = [
            "atm_admin_key",
            "atm_owner_auth",
            "neural_platform_key",
            "neural_user_api_key",
            "neural_workspace_id",
            "rainy_api",
            "rainyapi",
            "gemini",
            "gemini_byok",
        ];
        let values = self.get_many(&keys).await?;

        let mut provider_keys = HashMap::new();
        provider_keys.insert(
            "rainy_api".to_string(),
            values
                .get("rainy_api")
                .cloned()
                .unwrap_or(None)
                .or_else(|| values.get("rainyapi").cloned().unwrap_or(None)),
        );
        provider_keys.insert(
            "gemini".to_string(),
            values
                .get("gemini")
                .cloned()
                .unwrap_or(None)
                .or_else(|| values.get("gemini_byok").cloned().unwrap_or(None)),
        );

        Ok(StartupCredentialSnapshot {
            atm_admin_key: values.get("atm_admin_key").cloned().unwrap_or(None),
            owner_auth_bundle_raw: values.get("atm_owner_auth").cloned().unwrap_or(None),
            neural_platform_key: values.get("neural_platform_key").cloned().unwrap_or(None),
            neural_user_api_key: values.get("neural_user_api_key").cloned().unwrap_or(None),
            neural_workspace_id: values.get("neural_workspace_id").cloned().unwrap_or(None),
            provider_keys,
        })
    }

    async fn with_retry<F, Fut, T>(&self, op_name: &str, mut op: F) -> KeychainOpResult<T>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = KeychainOpResult<T>>,
    {
        let mut attempt = 0;
        loop {
            attempt += 1;
            match op().await {
                Ok(value) => return Ok(value),
                Err(err)
                    if err.kind == KeychainAccessErrorKind::TransientBusy
                        && attempt < MAX_ATTEMPTS =>
                {
                    tracing::warn!(
                        "[KeychainAccess] transient failure for {} on attempt {}: {}",
                        op_name,
                        attempt,
                        err
                    );
                    sleep(Self::retry_delay(attempt)).await;
                }
                Err(err) => return Err(err),
            }
        }
    }

    fn with_retry_blocking<F, T>(&self, op_name: &str, mut op: F) -> KeychainOpResult<T>
    where
        F: FnMut() -> KeychainOpResult<T>,
    {
        let mut attempt = 0;
        loop {
            attempt += 1;
            let result = {
                let _guard = global_gate().lock().map_err(|_| Self::poisoned())?;
                op()
            };

            match result {
                Ok(value) => return Ok(value),
                Err(err)
                    if err.kind == KeychainAccessErrorKind::TransientBusy
                        && attempt < MAX_ATTEMPTS =>
                {
                    tracing::warn!(
                        "[KeychainAccess] transient failure for {} on attempt {}: {}",
                        op_name,
                        attempt,
                        err
                    );
                    std::thread::sleep(Self::retry_delay(attempt));
                }
                Err(err) => return Err(err),
            }
        }
    }

    async fn run_blocking<T, F>(operation: F) -> KeychainOpResult<T>
    where
        T: Send + 'static,
        F: FnOnce() -> KeychainOpResult<T> + Send + 'static,
    {
        tokio::task::spawn_blocking(move || {
            let _guard = global_gate().lock().map_err(|_| Self::poisoned())?;
            operation()
        })
        .await
        .map_err(|e| KeychainAccessError {
            kind: KeychainAccessErrorKind::Other,
            message: format!("Keychain task join error: {}", e),
        })?
    }

    fn retry_delay(attempt: u32) -> Duration {
        let factor = 1u64 << attempt.saturating_sub(1);
        Duration::from_millis((BASE_BACKOFF_MS * factor).min(MAX_BACKOFF_MS))
    }

    fn poisoned() -> KeychainAccessError {
        KeychainAccessError {
            kind: KeychainAccessErrorKind::Other,
            message: "Keychain access mutex poisoned".to_string(),
        }
    }

    fn classify_error(message: String) -> KeychainAccessError {
        let lower = message.to_lowercase();
        let kind = if NOT_FOUND_KEYWORDS.iter().any(|item| lower.contains(item)) {
            KeychainAccessErrorKind::Other
        } else if TRANSIENT_KEYWORDS.iter().any(|item| lower.contains(item)) {
            KeychainAccessErrorKind::TransientBusy
        } else if PERMISSION_KEYWORDS.iter().any(|item| lower.contains(item)) {
            KeychainAccessErrorKind::PermissionDenied
        } else if lower.contains("utf-8") || lower.contains("invalid") {
            KeychainAccessErrorKind::DataCorrupt
        } else {
            KeychainAccessErrorKind::Other
        };
        KeychainAccessError { kind, message }
    }
}

#[cfg(test)]
mod tests {
    use super::KeychainAccessService;

    #[tokio::test]
    async fn keychain_access_roundtrip() {
        let service = KeychainAccessService::new();
        let key = "keychain_access_roundtrip";

        let _ = service.delete(key).await;
        service.set(key, "value-1").await.expect("set");
        assert_eq!(
            service.get(key).await.expect("get"),
            Some("value-1".to_string())
        );

        service.delete(key).await.expect("delete");
        assert_eq!(service.get(key).await.expect("get after delete"), None);
    }

    #[tokio::test]
    async fn startup_snapshot_reads_expected_keys() {
        let service = KeychainAccessService::new();
        let _ = service.delete("atm_admin_key").await;
        let _ = service.delete("rainy_api").await;

        service
            .set("atm_admin_key", "admin-secret")
            .await
            .expect("set atm");
        service
            .set("rainy_api", "ra-test")
            .await
            .expect("set rainy");

        let snapshot = service.load_startup_snapshot().await.expect("snapshot");
        assert_eq!(snapshot.atm_admin_key, Some("admin-secret".to_string()));
        assert_eq!(
            snapshot
                .provider_keys
                .get("rainy_api")
                .cloned()
                .unwrap_or(None),
            Some("ra-test".to_string())
        );
    }

    #[tokio::test]
    async fn concurrent_requests_share_serialized_access_path() {
        let service = KeychainAccessService::new();
        let key = "keychain_access_concurrent";
        let _ = service.delete(key).await;

        let mut tasks = Vec::new();
        for idx in 0..8u8 {
            let svc = service.clone();
            tasks.push(tokio::spawn(async move {
                let value = format!("value-{idx}");
                svc.set(key, &value).await?;
                svc.get(key).await
            }));
        }

        for task in tasks {
            let result = task.await.expect("join").expect("keychain op");
            assert!(result.is_some());
        }
    }
}
