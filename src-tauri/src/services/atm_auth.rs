use crate::ai::keychain::KeychainManager;
use serde::{Deserialize, Serialize};

const ATM_OWNER_AUTH_KEYCHAIN_ID: &str = "atm_owner_auth";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ATMOwnerAuthBundle {
    pub platform_key: String,
    pub user_api_key: String,
    pub workspace_id: String,
}

pub fn load_owner_auth_bundle() -> Result<Option<ATMOwnerAuthBundle>, String> {
    let keychain = KeychainManager::new();
    let raw = match keychain.get_key(ATM_OWNER_AUTH_KEYCHAIN_ID)? {
        Some(raw) => raw,
        None => return Ok(None),
    };

    serde_json::from_str::<ATMOwnerAuthBundle>(&raw)
        .map(Some)
        .map_err(|e| format!("Failed to decode ATM owner auth bundle: {}", e))
}

pub fn save_owner_auth_bundle(bundle: &ATMOwnerAuthBundle) -> Result<(), String> {
    let keychain = KeychainManager::new();
    let raw = serde_json::to_string(bundle)
        .map_err(|e| format!("Failed to encode ATM owner auth bundle: {}", e))?;
    keychain.store_key(ATM_OWNER_AUTH_KEYCHAIN_ID, &raw)
}

pub fn clear_owner_auth_bundle() -> Result<(), String> {
    let keychain = KeychainManager::new();
    keychain.delete_key(ATM_OWNER_AUTH_KEYCHAIN_ID)
}
