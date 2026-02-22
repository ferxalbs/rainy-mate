// Rainy Cowork - macOS Keychain Integration
// Secure storage for API keys using security-framework

#[cfg(target_os = "macos")]
use security_framework::passwords::{
    delete_generic_password, get_generic_password, set_generic_password,
};

const SERVICE_NAME: &str = "com.enosislabs.rainycowork";

/// Manager for secure API key storage via macOS Keychain
pub struct KeychainManager;

#[cfg(target_os = "macos")]
impl KeychainManager {
    pub fn new() -> Self {
        Self
    }

    /// Store an API key in the Keychain
    pub fn store_key(&self, provider: &str, api_key: &str) -> Result<(), String> {
        let account = format!("api_key_{}", provider);

        // Try to delete existing key first (in case of update)
        let _ = delete_generic_password(SERVICE_NAME, &account);

        set_generic_password(SERVICE_NAME, &account, api_key.as_bytes())
            .map_err(|e| format!("Failed to store API key: {}", e))
    }

    /// Retrieve an API key from the Keychain
    pub fn get_key(&self, provider: &str) -> Result<Option<String>, String> {
        let account = format!("api_key_{}", provider);

        match get_generic_password(SERVICE_NAME, &account) {
            Ok(bytes) => {
                let key = String::from_utf8(bytes.to_vec())
                    .map_err(|e| format!("Invalid key data: {}", e))?;
                Ok(Some(key))
            }
            Err(e) => {
                let err_str = e.to_string();
                // ItemNotFound is not an error - just means no key stored
                if err_str.contains("ItemNotFound")
                    || err_str.contains("not found")
                    || err_str.contains("could not be found")
                {
                    Ok(None)
                } else {
                    Err(format!("Failed to retrieve API key: {}", e))
                }
            }
        }
    }

    /// Delete an API key from the Keychain
    pub fn delete_key(&self, provider: &str) -> Result<(), String> {
        let account = format!("api_key_{}", provider);

        match delete_generic_password(SERVICE_NAME, &account) {
            Ok(_) => Ok(()),
            Err(e) => {
                let err_str = e.to_string();
                // Ignore "not found" errors
                if err_str.contains("ItemNotFound")
                    || err_str.contains("not found")
                    || err_str.contains("could not be found")
                {
                    Ok(())
                } else {
                    Err(format!("Failed to delete API key: {}", e))
                }
            }
        }
    }

    /// Check if an API key exists for a provider
    pub fn has_key(&self, provider: &str) -> bool {
        self.get_key(provider).map(|k| k.is_some()).unwrap_or(false)
    }
}

#[cfg(not(target_os = "macos"))]
impl KeychainManager {
    pub fn new() -> Self {
        Self
    }

    pub fn store_key(&self, _provider: &str, _api_key: &str) -> Result<(), String> {
        // No-op or error on non-macOS
        // For now, we just return Ok to not break flow, but maybe logging warning is better
        // However, user expects storage. Returning error might be safer.
        // But to pass tests/runtime, let's just log and return error
        Err("Secure storage is only supported on macOS currently.".to_string())
    }

    pub fn get_key(&self, _provider: &str) -> Result<Option<String>, String> {
        Ok(None)
    }

    pub fn delete_key(&self, _provider: &str) -> Result<(), String> {
        Ok(())
    }

    pub fn has_key(&self, _provider: &str) -> bool {
        false
    }
}

impl Default for KeychainManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "macos")]
    fn test_keychain_operations() {
        let manager = KeychainManager::new();
        let test_provider = "test_provider";
        let test_key = "test_api_key_12345";

        // Clean up first
        let _ = manager.delete_key(test_provider);

        // Store
        assert!(manager.store_key(test_provider, test_key).is_ok());

        // Retrieve
        let retrieved = manager.get_key(test_provider).unwrap();
        assert_eq!(retrieved, Some(test_key.to_string()));

        // Delete
        assert!(manager.delete_key(test_provider).is_ok());

        // Verify deleted
        let after_delete = manager.get_key(test_provider).unwrap();
        assert_eq!(after_delete, None);
    }
}
