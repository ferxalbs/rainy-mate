use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

const DEFAULT_REMOTE_GRANT_TTL: Duration = Duration::from_secs(60 * 60 * 8);

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteWorkspaceGrant {
    pub workspace_id: String,
    pub connector_id: String,
    pub session_peer: String,
    pub canonical_path: String,
    pub display_name: String,
    pub granted_at_ms: i64,
    pub last_used_at_ms: i64,
    pub expires_at_ms: i64,
}

#[derive(Debug, Default)]
pub struct RemoteWorkspaceGrantStore {
    grants: Arc<RwLock<HashMap<String, RemoteWorkspaceGrant>>>,
}

impl RemoteWorkspaceGrantStore {
    pub fn new() -> Self {
        Self::default()
    }

    fn key(workspace_id: &str, connector_id: &str, session_peer: &str) -> String {
        format!("{workspace_id}::{connector_id}::{session_peer}")
    }

    fn now_millis() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis() as i64)
            .unwrap_or_default()
    }

    fn compute_expiry(now_ms: i64) -> i64 {
        now_ms + DEFAULT_REMOTE_GRANT_TTL.as_millis() as i64
    }

    pub async fn get_active(
        &self,
        workspace_id: &str,
        connector_id: &str,
        session_peer: &str,
    ) -> Option<RemoteWorkspaceGrant> {
        let key = Self::key(workspace_id, connector_id, session_peer);
        let now_ms = Self::now_millis();
        let mut grants = self.grants.write().await;
        let grant = grants.get(&key).cloned()?;
        if grant.expires_at_ms <= now_ms {
            grants.remove(&key);
            return None;
        }
        Some(grant)
    }

    pub async fn touch(
        &self,
        workspace_id: &str,
        connector_id: &str,
        session_peer: &str,
    ) -> Option<RemoteWorkspaceGrant> {
        let key = Self::key(workspace_id, connector_id, session_peer);
        let now_ms = Self::now_millis();
        let mut grants = self.grants.write().await;
        let grant = grants.get_mut(&key)?;
        if grant.expires_at_ms <= now_ms {
            grants.remove(&key);
            return None;
        }
        grant.last_used_at_ms = now_ms;
        grant.expires_at_ms = Self::compute_expiry(now_ms);
        Some(grant.clone())
    }

    pub async fn insert(
        &self,
        workspace_id: &str,
        connector_id: &str,
        session_peer: &str,
        canonical_path: &str,
    ) -> RemoteWorkspaceGrant {
        let now_ms = Self::now_millis();
        let grant = RemoteWorkspaceGrant {
            workspace_id: workspace_id.to_string(),
            connector_id: connector_id.to_string(),
            session_peer: session_peer.to_string(),
            canonical_path: canonical_path.to_string(),
            display_name: Path::new(canonical_path)
                .file_name()
                .map(|value| value.to_string_lossy().to_string())
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| canonical_path.to_string()),
            granted_at_ms: now_ms,
            last_used_at_ms: now_ms,
            expires_at_ms: Self::compute_expiry(now_ms),
        };
        let key = Self::key(workspace_id, connector_id, session_peer);
        self.grants.write().await.insert(key, grant.clone());
        grant
    }
}
