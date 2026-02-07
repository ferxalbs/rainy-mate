use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSignature {
    // Ed25519 signature of the hash (soul + skills + memory config)
    pub signature: String,
    // The public key ID that signed this package
    pub signer_id: String,
    // Hash of the capabilities/skills json - preventing unauthorized skill escalation
    pub capabilities_hash: String,
    // Device ID where this agent was created/signed
    pub origin_device_id: String,
    // Timestamp of signing
    pub signed_at: i64,
}

impl AgentSignature {
    // Methods removed to avoid dead code warnings
}
