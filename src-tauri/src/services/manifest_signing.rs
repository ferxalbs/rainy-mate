/// Manifest Signing — HMAC-SHA256 for SkillManifest arrays
///
/// Produces hex digests compatible with ATM's `verifySkillsManifestSignature()`.
/// The canonical form is: JSON.stringify with recursively sorted object keys.
use crate::models::neural::SkillManifest;
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

// ──────────────────────────────────────────────────────────────────────────
// Canonical JSON (recursive key-sort to match TS `stableSortValue`)
// ──────────────────────────────────────────────────────────────────────────

/// Recursively sort all object keys in a `serde_json::Value` tree.
/// Arrays preserve element order; only object keys are sorted.
fn stable_sort_value(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut sorted = serde_json::Map::new();
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            for key in keys {
                sorted.insert(key.clone(), stable_sort_value(&map[key]));
            }
            serde_json::Value::Object(sorted)
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(stable_sort_value).collect())
        }
        other => other.clone(),
    }
}

/// Produce canonical JSON identical to ATM's `canonicalize()`.
fn canonicalize(value: &serde_json::Value) -> String {
    let sorted = stable_sort_value(value);
    serde_json::to_string(&sorted).unwrap_or_default()
}

// ──────────────────────────────────────────────────────────────────────────
// HMAC-SHA256
// ──────────────────────────────────────────────────────────────────────────

fn compute_hmac_sha256_hex(secret: &str, payload: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(payload.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

// ──────────────────────────────────────────────────────────────────────────
// Public API
// ──────────────────────────────────────────────────────────────────────────

/// Sign a skills manifest array with HMAC-SHA256.
/// Returns a hex digest compatible with ATM's `verifySkillsManifestSignature()`.
///
/// The secret MUST be the platform key (same value used in the `Authorization: Bearer` header).
pub fn sign_skills_manifest(manifests: &[SkillManifest], secret: &str) -> String {
    let json_value = serde_json::to_value(manifests).unwrap_or(serde_json::Value::Array(vec![]));
    let payload = canonicalize(&json_value);
    compute_hmac_sha256_hex(secret, &payload)
}

// ──────────────────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::neural::{ParameterSchema, SkillMethod};
    use std::collections::HashMap;
    use uuid::Uuid;

    fn test_manifest() -> SkillManifest {
        let mut params = HashMap::new();
        params.insert(
            "path".to_string(),
            ParameterSchema {
                param_type: "string".to_string(),
                required: Some(true),
                description: Some("File path".to_string()),
            },
        );

        SkillManifest {
            name: "filesystem".to_string(),
            version: "1.0.0".to_string(),
            methods: vec![SkillMethod {
                name: "read_file".to_string(),
                description: "Read a file".to_string(),
                airlock_level: crate::models::neural::AirlockLevel::Safe,
                parameters: params,
            }],
        }
    }

    #[test]
    fn sign_produces_hex_digest() {
        let digest = sign_skills_manifest(&[test_manifest()], "test-secret");
        // Hex string of 32 bytes = 64 hex chars
        assert_eq!(digest.len(), 64);
        assert!(digest.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn sign_is_deterministic() {
        let a = sign_skills_manifest(&[test_manifest()], "key");
        let b = sign_skills_manifest(&[test_manifest()], "key");
        assert_eq!(a, b);
    }

    #[test]
    fn different_keys_produce_different_digests() {
        let a = sign_skills_manifest(&[test_manifest()], "key-a");
        let b = sign_skills_manifest(&[test_manifest()], "key-b");
        assert_ne!(a, b);
    }

    #[test]
    fn empty_manifests_still_signs() {
        let test_secret = Uuid::new_v4().to_string();
        let digest = sign_skills_manifest(&[], &test_secret);
        assert_eq!(digest.len(), 64);
    }

    #[test]
    fn canonicalize_sorts_keys_recursively() {
        let json: serde_json::Value = serde_json::json!({
            "z": 1,
            "a": { "c": 3, "b": 2 }
        });
        let canonical = canonicalize(&json);
        assert_eq!(canonical, r#"{"a":{"b":2,"c":3},"z":1}"#);
    }
}
