use crate::ai::keychain::KeychainManager; // Assuming existence
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use ed25519_dalek::{Signer, SigningKey, Verifier};
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};

const IDENTITY_KEY_ID: &str = "rainy_agent_identity_v1";

#[derive(Debug, thiserror::Error)]
pub enum SecurityError {
    #[error("Keychain error: {0}")]
    KeychainError(String),
}

pub struct SecurityService {
    keychain: KeychainManager,
}

impl SecurityService {
    pub fn new() -> Self {
        Self {
            keychain: KeychainManager::new(),
        }
    }

    /// Gets or creates the device identity key
    /// In ed25519-dalek v2, SigningKey contains the secret and can derive the public key.
    pub fn get_identity_key(&self) -> Result<SigningKey, SecurityError> {
        // Try to load from keychain
        if let Ok(Some(secret_bytes_b64)) = self.keychain.get_key(IDENTITY_KEY_ID) {
            if let Ok(secret_bytes) = BASE64.decode(&secret_bytes_b64) {
                // Expecting 32 bytes for the secret seed
                if let Ok(array) = <[u8; 32]>::try_from(secret_bytes.as_slice()) {
                    return Ok(SigningKey::from_bytes(&array));
                }
            }
        }

        // Generate new if not found
        let mut csprng = OsRng {};
        let signing_key = SigningKey::generate(&mut csprng);

        let secret_bytes = signing_key.to_bytes(); // Returns [u8; 32]
        let b64 = BASE64.encode(secret_bytes);

        self.keychain
            .store_key(IDENTITY_KEY_ID, &b64)
            .map_err(|e| SecurityError::KeychainError(e.to_string()))?;

        Ok(signing_key)
    }

    pub fn sign_content(&self, content: &str) -> Result<String, SecurityError> {
        let key = self.get_identity_key()?;
        let signature = key.sign(content.as_bytes());
        Ok(BASE64.encode(signature.to_bytes()))
    }

    pub fn get_public_key_string(&self) -> Result<String, SecurityError> {
        let key = self.get_identity_key()?;
        let verifying_key = key.verifying_key();
        Ok(BASE64.encode(verifying_key.to_bytes()))
    }

    pub fn hash_capabilities(skills_json: &serde_json::Value) -> String {
        let json_str = serde_json::to_string(skills_json).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(json_str);
        hex::encode(hasher.finalize())
    }
}
