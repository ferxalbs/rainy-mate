// Rainy Cowork - macOS Keychain Integration
// Secure storage for API keys using security-framework on macOS, and an explicit error on other platforms

#[cfg(all(target_os = "macos", not(test)))]
use security_framework::passwords::{
    delete_generic_password, get_generic_password, set_generic_password,
};

#[cfg(test)]
use std::collections::HashMap;
#[cfg(test)]
use std::sync::{Mutex, OnceLock};

#[cfg(not(test))]
#[allow(dead_code)]
const SERVICE_NAME: &str = "com.enosislabs.rainycowork";

#[cfg(test)]
fn test_store() -> &'static Mutex<HashMap<String, String>> {
    static STORE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Manager for secure API key storage via macOS Keychain or OS keyring
pub struct KeychainManager;

impl KeychainManager {
    pub fn new() -> Self {
        Self
    }

    /// Store an API key in the Keychain/Keyring
    #[allow(unused_variables)]
    pub fn store_key(&self, provider: &str, api_key: &str) -> Result<(), String> {
        let account = format!("api_key_{}", provider);

        #[cfg(test)]
        {
            let mut store = test_store()
                .lock()
                .map_err(|_| "Failed to lock keychain test store".to_string())?;
            store.insert(account, api_key.to_string());
            return Ok(());
        }

        #[cfg(all(target_os = "macos", not(test)))]
        {
            // Try to delete existing key first (in case of update)
            let _ = delete_generic_password(SERVICE_NAME, &account);

            set_generic_password(SERVICE_NAME, &account, api_key.as_bytes())
                .map_err(|e| format!("Failed to store API key: {}", e))
        }

        #[cfg(all(not(target_os = "macos"), not(test)))]
        {
            Err("Secure keychain storage is only supported on macOS currently.".to_string())
        }
    }

    /// Retrieve an API key from the Keychain/Keyring
    #[allow(unused_variables)]
    pub fn get_key(&self, provider: &str) -> Result<Option<String>, String> {
        let account = format!("api_key_{}", provider);

        #[cfg(test)]
        {
            let store = test_store()
                .lock()
                .map_err(|_| "Failed to lock keychain test store".to_string())?;
            return Ok(store.get(&account).cloned());
        }

        #[cfg(all(target_os = "macos", not(test)))]
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

        #[cfg(all(not(target_os = "macos"), not(test)))]
        {
            Ok(None)
        }
    }

    /// Delete an API key from the Keychain/Keyring
    #[allow(unused_variables)]
    pub fn delete_key(&self, provider: &str) -> Result<(), String> {
        let account = format!("api_key_{}", provider);

        #[cfg(test)]
        {
            let mut store = test_store()
                .lock()
                .map_err(|_| "Failed to lock keychain test store".to_string())?;
            store.remove(&account);
            return Ok(());
        }

        #[cfg(all(target_os = "macos", not(test)))]
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

        #[cfg(all(not(target_os = "macos"), not(test)))]
        {
            Ok(())
        }
    }

    /// Check if an API key exists for a provider
    pub fn has_key(&self, provider: &str) -> bool {
        self.get_key(provider).map(|k| k.is_some()).unwrap_or(false)
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
