use crate::services::KeychainAccessService;
use serde::{Deserialize, Serialize};

const ATM_OWNER_AUTH_KEYCHAIN_ID: &str = "atm_owner_auth";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ATMOwnerAuthBundle {
    pub platform_key: String,
    pub user_api_key: String,
    pub workspace_id: String,
}

pub async fn load_owner_auth_bundle(
    keychain: &KeychainAccessService,
) -> Result<Option<ATMOwnerAuthBundle>, String> {
    let raw = match keychain
        .get(ATM_OWNER_AUTH_KEYCHAIN_ID)
        .await
        .map_err(|e| e.to_string())?
    {
        Some(raw) => raw,
        None => return Ok(None),
    };

    serde_json::from_str::<ATMOwnerAuthBundle>(&raw)
        .map(Some)
        .map_err(|e| format!("Failed to decode ATM owner auth bundle: {}", e))
}

pub async fn save_owner_auth_bundle(
    keychain: &KeychainAccessService,
    bundle: &ATMOwnerAuthBundle,
) -> Result<(), String> {
    let raw = serde_json::to_string(bundle)
        .map_err(|e| format!("Failed to encode ATM owner auth bundle: {}", e))?;
    keychain
        .set(ATM_OWNER_AUTH_KEYCHAIN_ID, &raw)
        .await
        .map_err(|e| e.to_string())
}

pub async fn clear_owner_auth_bundle(keychain: &KeychainAccessService) -> Result<(), String> {
    keychain
        .delete(ATM_OWNER_AUTH_KEYCHAIN_ID)
        .await
        .map_err(|e| e.to_string())
}
