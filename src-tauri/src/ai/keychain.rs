// Rainy Cowork - macOS Keychain Integration
// Secure storage for API keys using security-framework

#[cfg(target_os = "macos")]
use security_framework::item::{
    ItemAddOptions, ItemAddValue, ItemClass, ItemSearchOptions, Limit, Reference, SearchResult,
};
#[cfg(target_os = "macos")]
use std::borrow::Cow;

const SERVICE_NAME: &str = "com.enosislabs.rainycowork";

/// Manager for secure API key storage
pub struct KeychainManager;

#[cfg(target_os = "macos")]
impl KeychainManager {
    pub fn new() -> Self {
        Self
    }

    /// Store an API key in the Keychain
    pub fn store_key(&self, provider: &str, api_key: &str) -> Result<(), String> {
        let account = format!("api_key_{}", provider);

        // Best effort delete existing key first
        let _ = self.delete_key(provider);

        // Construct using the correct variant for v3.1.0+
        let value = ItemAddValue::Data {
            class: ItemClass::generic_password(),
            data: Cow::Borrowed(api_key.as_bytes()),
        };

        ItemAddOptions::new(value)
            .set_service(SERVICE_NAME)
            .set_account_name(&account)
            .add()
            .map_err(|e| format!("Failed to store API key: {}", e))?;

        Ok(())
    }

    /// Retrieve an API key from the Keychain
    pub fn get_key(&self, provider: &str) -> Result<Option<String>, String> {
        let account = format!("api_key_{}", provider);

        match ItemSearchOptions::new()
            .class(ItemClass::generic_password())
            .service(SERVICE_NAME)
            .account(&account)
            .load_data(true)
            .limit(Limit::One)
            .search()
        {
            Ok(results) => {
                if results.is_empty() {
                    return Ok(None);
                }
                match results.first() {
                    Some(SearchResult::Data(data)) => {
                        let key = String::from_utf8(data.to_vec())
                            .map_err(|e| format!("Invalid key data: {}", e))?;
                        Ok(Some(key))
                    }
                    _ => Ok(None),
                }
            }
            Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("The specified item could not be found") || err_str.contains("not found") {
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

        // Use security-framework-sys directly because high-level delete might be missing or hidden
        use security_framework_sys::item::{SecItemDelete, kSecClass, kSecClassGenericPassword, kSecAttrService, kSecAttrAccount};
        use core_foundation::dictionary::CFDictionary;
        use core_foundation::string::CFString;
        use core_foundation::base::TCFType;

        unsafe {
            let service = CFString::new(SERVICE_NAME);
            let account_cf = CFString::new(&account);

            let mut query_pairs = vec![
                (kSecClass, kSecClassGenericPassword.as_void_ptr()),
                (kSecAttrService, service.as_void_ptr()),
                (kSecAttrAccount, account_cf.as_void_ptr()),
            ];

            let query = CFDictionary::from_CFType_pairs(&query_pairs);

            let status = SecItemDelete(query.as_concrete_TypeRef());

            // errSecSuccess = 0, errSecItemNotFound = -25300
            if status == 0 || status == -25300 {
                Ok(())
            } else {
                Err(format!("Failed to delete API key, status: {}", status))
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
        Ok(())
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
