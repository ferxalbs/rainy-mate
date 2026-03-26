use crate::models::neural::ToolAccessPolicy;
use crate::services::settings::{SettingsManager, WorkspaceToolPolicyState};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetPolicyEnvelope {
    pub tool_access_policy: ToolAccessPolicy,
    pub tool_access_policy_version: u64,
    pub tool_access_policy_hash: String,
}

fn hash_policy(policy: &ToolAccessPolicy) -> String {
    let mut allow = policy.allow.clone();
    allow.sort();
    let mut deny = policy.deny.clone();
    deny.sort();
    let canonical = serde_json::json!({
        "enabled": policy.enabled,
        "mode": policy.mode,
        "allow": allow,
        "deny": deny,
    })
    .to_string();
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn apply_fleet_policy(
    workspace_id: &str,
    envelope: &FleetPolicyEnvelope,
) -> Result<(), String> {
    let computed = hash_policy(&envelope.tool_access_policy);
    if computed != envelope.tool_access_policy_hash {
        return Err("Fleet policy hash mismatch".to_string());
    }

    let mut settings = SettingsManager::new();
    let floor = settings.get_tool_policy_floor(workspace_id);
    if envelope.tool_access_policy_version < floor {
        return Err(format!(
            "Rejecting stale fleet policy version {} (latest seen {})",
            envelope.tool_access_policy_version, floor
        ));
    }

    settings
        .set_tool_policy_floor(workspace_id, envelope.tool_access_policy_version)
        .map_err(|e| format!("Failed to persist fleet policy floor: {}", e))?;

    settings
        .set_workspace_tool_policy_state(
            workspace_id,
            WorkspaceToolPolicyState {
                tool_access_policy: envelope.tool_access_policy.clone(),
                tool_access_policy_version: envelope.tool_access_policy_version,
                tool_access_policy_hash: envelope.tool_access_policy_hash.clone(),
            },
        )
        .map_err(|e| format!("Failed to persist fleet policy state: {}", e))?;

    Ok(())
}
