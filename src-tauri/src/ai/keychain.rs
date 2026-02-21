// Rainy Cowork - macOS Keychain Integration
// Secure storage for API keys using security-framework

#[cfg(target_os = "macos")]
use security_framework::passwords::{
    delete_generic_password, get_generic_password, set_generic_password,
};

#[cfg(target_os = "macos")]
const SERVICE_NAME: &str = "com.enosislabs.rainycowork";

/// Manager for secure API key storage via macOS Keychain
pub struct KeychainManager;

impl KeychainManager {
    /// Creates a new KeychainManager handle.
    ///
    /// # Examples
    ///
    /// ```
    /// let mgr = KeychainManager::new();
    /// ```
    pub fn new() -> Self {
        Self
    }

    /// Store the given API key in the macOS Keychain under a provider-scoped account.
    ///
    /// The key is stored under the account name `api_key_<provider>` and will overwrite any
    /// existing key for that provider.
    ///
    /// # Parameters
    ///
    /// - `provider`: identifier for the API provider; used to construct the account name.
    /// - `api_key`: the API key bytes to store (UTF-8 string is expected).
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, `Err(String)` with a descriptive message on failure.
    ///
    /// # Examples
    ///
    /// ```
    /// let manager = KeychainManager::new();
    /// let provider = "example";
    /// let api_key = "secret-token";
    /// manager.store_key(provider, api_key).expect("failed to store key");
    /// ```
    #[cfg(target_os = "macos")]
    pub fn store_key(&self, provider: &str, api_key: &str) -> Result<(), String> {
        let account = format!("api_key_{}", provider);

        // Try to delete existing key first (in case of update)
        let _ = delete_generic_password(SERVICE_NAME, &account);

        set_generic_password(SERVICE_NAME, &account, api_key.as_bytes())
            .map_err(|e| format!("Failed to store API key: {}", e))
    }

    /// Retrieves the API key for the given provider from the Keychain.
    ///
    /// On success returns `Ok(Some(key))` when an entry exists or `Ok(None)` when no entry is found.
    /// Returns `Err` with a descriptive message for other failures.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// let mgr = KeychainManager::new();
    /// match mgr.get_key("example_provider") {
    ///     Ok(Some(key)) => println!("retrieved key: {}", key),
    ///     Ok(None) => println!("no key stored for provider"),
    ///     Err(err) => eprintln!("error retrieving key: {}", err),
    /// }
    /// ```
    #[cfg(target_os = "macos")]
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

    /// Deletes the stored API key for the given provider from the macOS Keychain.
    ///
    /// This method removes the Keychain item with an account name formed as `api_key_<provider>`.
    /// If the item does not exist, the call is treated as successful (no error is returned).
    ///
    /// # Returns
    ///
    /// `Ok(())` if the key was deleted or did not exist, `Err(String)` with a descriptive message on other failures.
    ///
    /// # Examples
    ///
    /// ```
    /// let mgr = KeychainManager::new();
    /// // Remove any existing key for "example" (succeeds even if no key exists)
    /// mgr.delete_key("example").expect("failed to delete key");
    /// ```
    #[cfg(target_os = "macos")]
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

    // ========================================================================
    // Non-macOS Fallbacks (Stubs)
    // ========================================================================

    /// Placeholder implementation for storing an API key on non-macOS platforms; does not persist the key.
    ///
    /// This implementation is a no-op on platforms other than macOS and only returns success to avoid
    /// breaking application flow. It does not provide secure, persistent storage of the provided key.
    ///
    /// # Examples
    ///
    /// ```
    /// let mgr = crate::ai::keychain::KeychainManager::new();
    /// assert!(mgr.store_key("provider", "secret").is_ok());
    /// ```
    #[cfg(not(target_os = "macos"))]
    pub fn store_key(&self, _provider: &str, _api_key: &str) -> Result<(), String> {
        // TODO: Implement secure storage for Windows/Linux (e.g., using keytar/secret-service)
        // For now, we return Ok to not break the app flow, but data isn't persisted securely.
        eprintln!("WARN: Secure keychain storage not implemented for this OS");
        Ok(())
    }

    /// Indicates that no API key is available on non-macOS platforms.
    ///
    /// The `provider` argument is ignored; secure key storage is not implemented on this platform.
    ///
    /// # Examples
    ///
    /// ```
    /// let mgr = KeychainManager::new();
    /// assert_eq!(mgr.get_key("openai").unwrap(), None);
    /// ```
    #[cfg(not(target_os = "macos"))]
    pub fn get_key(&self, _provider: &str) -> Result<Option<String>, String> {
        Ok(None)
    }

    /// No-op fallback that simulates deleting an API key for the given provider on non-macOS platforms.
    ///
    /// This implementation does not persist or remove any data; it always reports success.
    ///
    /// # Examples
    ///
    /// ```
    /// let mgr = KeychainManager::new();
    /// assert!(mgr.delete_key("example_provider").is_ok());
    /// ```
    ///
    /// # Returns
    ///
    /// `Ok(())` indicating the operation completed successfully.
    #[cfg(not(target_os = "macos"))]
    pub fn delete_key(&self, _provider: &str) -> Result<(), String> {
        Ok(())
    }

    /// Determines whether an API key is stored for the given provider.
    ///
    /// # Examples
    ///
    /// ```
    /// let mgr = KeychainManager::new();
    /// let _exists = mgr.has_key("example_provider");
    /// ```
    ///
    /// # Returns
    ///
    /// `true` if a stored API key exists for `provider`, `false` otherwise.
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
        #[cfg(target_os = "macos")]
        {
            let retrieved = manager.get_key(test_provider).unwrap();
            assert_eq!(retrieved, Some(test_key.to_string()));
        }

        #[cfg(not(target_os = "macos"))]
        {
            let retrieved = manager.get_key(test_provider).unwrap();
            assert_eq!(retrieved, None);
        }

        // Delete
        assert!(manager.delete_key(test_provider).is_ok());

        // Verify deleted
        let after_delete = manager.get_key(test_provider).unwrap();
        assert_eq!(after_delete, None);
    }
}