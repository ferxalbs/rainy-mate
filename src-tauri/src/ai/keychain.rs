// Rainy Cowork - macOS Keychain Integration
// Secure storage for API keys using security-framework

#[cfg(target_os = "macos")]
use security_framework::passwords::{
    delete_generic_password, get_generic_password, set_generic_password,
};
#[cfg(test)]
use std::collections::HashMap;
#[cfg(test)]
use std::sync::{Mutex, OnceLock};

#[cfg(all(not(test), target_os = "macos"))]
const SERVICE_NAME: &str = "com.enosislabs.rainycowork";

#[cfg(test)]
fn test_store() -> &'static Mutex<HashMap<String, String>> {
    static STORE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Manager for secure API key storage via macOS Keychain
pub struct KeychainManager;

impl KeychainManager {
    pub fn new() -> Self {
        Self
    }

    /// Store an API key in the Keychain
    pub fn store_key(&self, _provider: &str, _api_key: &str) -> Result<(), String> {
        #[cfg(any(test, target_os = "macos"))]
        let account = format!("api_key_{}", _provider);

        #[cfg(test)]
        {
            let mut store = test_store()
                .lock()
                .map_err(|_| "Failed to lock keychain test store".to_string())?;
            store.insert(account, _api_key.to_string());
            return Ok(());
        }

        #[cfg(all(not(test), target_os = "macos"))]
        {
            // Try to delete existing key first (in case of update)
            let _ = delete_generic_password(SERVICE_NAME, &account);

            set_generic_password(SERVICE_NAME, &account, _api_key.as_bytes())
                .map_err(|e| format!("Failed to store API key: {}", e))
        }

        #[cfg(all(not(test), not(target_os = "macos")))]
        {
            Err("Keychain is only supported on macOS".to_string())
        }
    }

    /// Retrieve an API key from the Keychain
    pub fn get_key(&self, _provider: &str) -> Result<Option<String>, String> {
        #[cfg(any(test, target_os = "macos"))]
        let account = format!("api_key_{}", _provider);

        #[cfg(test)]
        {
            let store = test_store()
                .lock()
                .map_err(|_| "Failed to lock keychain test store".to_string())?;
            return Ok(store.get(&account).cloned());
        }

        #[cfg(all(not(test), target_os = "macos"))]
        {
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

        #[cfg(all(not(test), not(target_os = "macos")))]
        {
            Err("Keychain is only supported on macOS".to_string())
        }
    }

    /// Delete an API key from the Keychain
    pub fn delete_key(&self, _provider: &str) -> Result<(), String> {
        #[cfg(any(test, target_os = "macos"))]
        let account = format!("api_key_{}", _provider);

        #[cfg(test)]
        {
            let mut store = test_store()
                .lock()
                .map_err(|_| "Failed to lock keychain test store".to_string())?;
            store.remove(&account);
            return Ok(());
        }

        #[cfg(all(not(test), target_os = "macos"))]
        {
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

        #[cfg(all(not(test), not(target_os = "macos")))]
        {
            Err("Keychain is only supported on macOS".to_string())
        }
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
