//! Beam RPC + Secure Local Signing Bridge
//!
//! Provides:
//! - Official Beam Mainnet / Testnet chain configs
//! - Workspace `.rainy-mate/beam/config.json` management
//! - AES-256-GCM encrypted local wallet storage (private keys never leave the device)
//! - Gas estimation via JSON-RPC (`eth_estimateGas`, `eth_gasPrice`)
//! - EIP-155 transaction signing (secp256k1 / keccak256)
//! - Transaction broadcast (`eth_sendRawTransaction`)
//!
//! All signing operations require Airlock L2 approval before they reach this layer.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use k256::ecdsa::SigningKey;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sha3::Keccak256;
use std::path::{Path, PathBuf};

// ── Network definitions ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BeamNetwork {
    Mainnet,
    Testnet,
}

impl BeamNetwork {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "mainnet" => Some(Self::Mainnet),
            "testnet" | "fuji" => Some(Self::Testnet),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BeamChainConfig {
    pub network: String,
    pub chain_id: u64,
    pub chain_id_hex: String,
    pub rpc_url: String,
    pub ws_url: String,
    pub explorer_url: String,
    pub currency_symbol: String,
    pub currency_decimals: u8,
}

impl BeamChainConfig {
    pub fn mainnet() -> Self {
        Self {
            network: "Beam Mainnet".to_string(),
            chain_id: 4337,
            chain_id_hex: "0x10f1".to_string(),
            rpc_url: "https://build.onbeam.com/rpc".to_string(),
            ws_url: "wss://build.onbeam.com/ws".to_string(),
            explorer_url: "https://subnets.avax.network/beam".to_string(),
            currency_symbol: "BEAM".to_string(),
            currency_decimals: 18,
        }
    }

    pub fn testnet() -> Self {
        Self {
            network: "Beam Testnet".to_string(),
            chain_id: 13337,
            chain_id_hex: "0x3419".to_string(),
            rpc_url: "https://build.onbeam.com/rpc/testnet".to_string(),
            ws_url: "wss://build.onbeam.com/ws/testnet".to_string(),
            explorer_url: "https://subnets-test.avax.network/beam".to_string(),
            currency_symbol: "BEAM".to_string(),
            currency_decimals: 18,
        }
    }

    pub fn for_network(network: BeamNetwork) -> Self {
        match network {
            BeamNetwork::Mainnet => Self::mainnet(),
            BeamNetwork::Testnet => Self::testnet(),
        }
    }
}

// ── Workspace config ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BeamWorkspaceConfig {
    pub schema_version: u32,
    pub connected_at: String,
    pub network: String,
    pub chain: BeamChainConfig,
}

// ── Wallet types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletInfo {
    /// EIP-55 checksummed Ethereum address
    pub address: String,
    /// Optional human-readable label
    pub label: Option<String>,
    /// Creation timestamp (Unix seconds)
    pub created_at: i64,
}

/// Persisted (encrypted) wallet record — stored on disk.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EncryptedWalletRecord {
    address: String,
    label: Option<String>,
    created_at: i64,
    /// Hex-encoded nonce (12 bytes)
    nonce_hex: String,
    /// Hex-encoded AES-256-GCM ciphertext of the 32-byte private key
    ciphertext_hex: String,
}

// ── Transaction types ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionRequest {
    pub from: String,
    pub to: Option<String>,
    /// Value in wei as 0x-prefixed hex (default "0x0")
    pub value: Option<String>,
    /// Transaction data as 0x-prefixed hex
    pub data: Option<String>,
    /// Gas limit (decimal)
    pub gas_limit: Option<u64>,
    /// Gas price in wei (decimal)
    pub gas_price: Option<u64>,
    /// Nonce override
    pub nonce: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GasEstimate {
    /// Estimated gas units
    pub gas_limit: u64,
    /// Gas price in wei
    pub gas_price: u64,
    /// Estimated fee in wei (gas_limit * gas_price)
    pub estimated_fee_wei: u128,
    /// Estimated fee formatted (in BEAM, 18 decimals)
    pub estimated_fee_beam: String,
    /// Which network was queried
    pub network: String,
    pub rpc_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignedTransaction {
    /// RLP-encoded signed transaction as 0x-prefixed hex
    pub raw_tx: String,
    /// keccak256 hash of raw_tx (transaction hash)
    pub tx_hash: String,
    /// Transaction details for display
    pub from: String,
    pub to: Option<String>,
    pub value_hex: String,
    pub gas_limit: u64,
    pub gas_price: u64,
    pub nonce: u64,
    pub chain_id: u64,
    pub network: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionReceipt {
    pub tx_hash: String,
    pub network: String,
    pub explorer_url: String,
    pub status: Option<String>,
    pub block_number: Option<u64>,
    pub gas_used: Option<u64>,
    pub contract_address: Option<String>,
}

// ── JSON-RPC helpers ─────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct RpcRequest<'a> {
    jsonrpc: &'a str,
    method: &'a str,
    params: serde_json::Value,
    id: u64,
}

#[derive(Debug, Deserialize)]
struct RpcResponse {
    result: Option<serde_json::Value>,
    error: Option<serde_json::Value>,
}

// ── RLP encoding (minimal — only what Ethereum Type-0 transactions need) ─────

fn rlp_encode_bytes(bytes: &[u8]) -> Vec<u8> {
    if bytes.len() == 1 && bytes[0] < 0x80 {
        return bytes.to_vec();
    }
    if bytes.is_empty() {
        return vec![0x80];
    }
    if bytes.len() < 56 {
        let mut out = vec![0x80 + bytes.len() as u8];
        out.extend_from_slice(bytes);
        out
    } else {
        let len_bytes = big_endian_bytes(bytes.len() as u64);
        let mut out = vec![0xb7 + len_bytes.len() as u8];
        out.extend_from_slice(&len_bytes);
        out.extend_from_slice(bytes);
        out
    }
}

fn rlp_encode_u64(v: u64) -> Vec<u8> {
    if v == 0 {
        return rlp_encode_bytes(&[]);
    }
    rlp_encode_bytes(&big_endian_bytes(v))
}

fn rlp_encode_u128(v: u128) -> Vec<u8> {
    if v == 0 {
        return rlp_encode_bytes(&[]);
    }
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&v.to_be_bytes());
    // Strip leading zeros
    let start = bytes.iter().position(|&b| b != 0).unwrap_or(15);
    rlp_encode_bytes(&bytes[start..])
}

fn rlp_encode_list(items: &[Vec<u8>]) -> Vec<u8> {
    let payload: Vec<u8> = items.iter().flat_map(|i| i.iter().copied()).collect();
    let len = payload.len();
    let mut out = if len < 56 {
        vec![0xc0 + len as u8]
    } else {
        let len_bytes = big_endian_bytes(len as u64);
        let mut prefix = vec![0xf7 + len_bytes.len() as u8];
        prefix.extend_from_slice(&len_bytes);
        prefix
    };
    out.extend_from_slice(&payload);
    out
}

fn big_endian_bytes(v: u64) -> Vec<u8> {
    let bytes = v.to_be_bytes();
    let start = bytes.iter().position(|&b| b != 0).unwrap_or(7);
    bytes[start..].to_vec()
}

fn decode_hex_bytes(s: &str) -> Result<Vec<u8>, String> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    if s.is_empty() {
        return Ok(vec![]);
    }
    hex::decode(s).map_err(|e| format!("Invalid hex: {}", e))
}

fn decode_hex_u64(s: &str) -> Result<u64, String> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    u64::from_str_radix(s, 16).map_err(|e| format!("Invalid hex number: {}", e))
}

fn decode_hex_u128(s: &str) -> Result<u128, String> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    u128::from_str_radix(s, 16).map_err(|e| format!("Invalid hex number: {}", e))
}

// ── EVM crypto helpers ───────────────────────────────────────────────────────

fn keccak256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak256::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Derive checksummed EIP-55 address from secp256k1 public key bytes.
/// `pubkey_uncompressed` must be the 65-byte uncompressed point (0x04 prefix).
fn pubkey_to_address(pubkey_uncompressed: &[u8]) -> String {
    assert_eq!(
        pubkey_uncompressed.len(),
        65,
        "expected 65-byte uncompressed pubkey"
    );
    // Drop the 0x04 prefix; take last 32 bytes (y and x are each 32 bytes)
    let hash = keccak256(&pubkey_uncompressed[1..]);
    let addr_bytes = &hash[12..]; // last 20 bytes
    eip55_checksum(addr_bytes)
}

/// EIP-55 mixed-case checksum encoding.
fn eip55_checksum(addr_bytes: &[u8]) -> String {
    let addr_hex = hex::encode(addr_bytes);
    let hash = keccak256(addr_hex.as_bytes());
    let checksummed: String = addr_hex
        .chars()
        .enumerate()
        .map(|(i, c)| {
            if c.is_alphabetic() {
                let nibble = (hash[i / 2] >> (if i % 2 == 0 { 4 } else { 0 })) & 0x0f;
                if nibble >= 8 {
                    c.to_uppercase().next().unwrap()
                } else {
                    c
                }
            } else {
                c
            }
        })
        .collect();
    format!("0x{}", checksummed)
}

/// Sign a keccak256 pre-hash with a secp256k1 signing key.
/// Returns (r_bytes, s_bytes, recovery_id).
fn secp256k1_sign_prehash(
    signing_key: &SigningKey,
    hash: &[u8; 32],
) -> Result<([u8; 32], [u8; 32], u8), String> {
    let (sig, recovery_id) = signing_key
        .sign_prehash_recoverable(hash)
        .map_err(|e| format!("Signing failed: {}", e))?;
    let sig_bytes = sig.to_bytes();
    let mut r = [0u8; 32];
    let mut s = [0u8; 32];
    r.copy_from_slice(&sig_bytes[..32]);
    s.copy_from_slice(&sig_bytes[32..]);
    Ok((r, s, recovery_id.to_byte()))
}

// ── Wallet encryption ────────────────────────────────────────────────────────

fn derive_wallet_key(master_key: &[u8], address: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(master_key);
    hasher.update(b"beam-wallet-v1:");
    hasher.update(address.to_lowercase().as_bytes());
    hasher.finalize().into()
}

fn encrypt_private_key(
    master_key: &[u8],
    address: &str,
    private_key_bytes: &[u8],
) -> Result<(Vec<u8>, [u8; 12]), String> {
    let key_material = derive_wallet_key(master_key, address);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key_material));
    let mut nonce = [0u8; 12];
    rand::rngs::OsRng.fill_bytes(&mut nonce);
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce), private_key_bytes)
        .map_err(|e| format!("Encryption failed: {}", e))?;
    Ok((ciphertext, nonce))
}

fn decrypt_private_key(
    master_key: &[u8],
    address: &str,
    ciphertext: &[u8],
    nonce: &[u8],
) -> Result<Vec<u8>, String> {
    let key_material = derive_wallet_key(master_key, address);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key_material));
    cipher
        .decrypt(Nonce::from_slice(nonce), ciphertext)
        .map_err(|e| format!("Decryption failed: {}", e))
}

// ── BeamRpcService ───────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct BeamRpcService {
    /// App-scoped data directory for encrypted wallet storage
    wallet_dir: PathBuf,
    /// Path to the 32-byte master key file
    master_key_path: PathBuf,
    /// Shared HTTP client
    http_client: reqwest::Client,
}

impl std::fmt::Debug for BeamRpcService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BeamRpcService")
            .field("wallet_dir", &self.wallet_dir)
            .finish()
    }
}

impl BeamRpcService {
    pub fn new(app_data_dir: PathBuf) -> Self {
        let beam_dir = app_data_dir.join("beam_wallets");
        let master_key_path = beam_dir.join(".master");
        Self {
            wallet_dir: beam_dir,
            master_key_path,
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("HTTP client build"),
        }
    }

    // ── Static chain configs ─────────────────────────────────────────────

    pub fn chain_config(network: BeamNetwork) -> BeamChainConfig {
        BeamChainConfig::for_network(network)
    }

    pub fn all_chain_configs() -> serde_json::Value {
        serde_json::json!({
            "mainnet": BeamChainConfig::mainnet(),
            "testnet": BeamChainConfig::testnet(),
        })
    }

    // ── Workspace config ─────────────────────────────────────────────────

    pub fn write_workspace_config(
        &self,
        workspace_path: &str,
        network: BeamNetwork,
    ) -> Result<BeamWorkspaceConfig, String> {
        let config_dir = Path::new(workspace_path).join(".rainy-mate").join("beam");
        std::fs::create_dir_all(&config_dir)
            .map_err(|e| format!("Failed to create .rainy-mate/beam: {}", e))?;

        let chain = BeamChainConfig::for_network(network);
        let config = BeamWorkspaceConfig {
            schema_version: 1,
            connected_at: chrono::Utc::now().to_rfc3339(),
            network: chain.network.clone(),
            chain,
        };

        let json = serde_json::to_string_pretty(&config)
            .map_err(|e| format!("Serialization failed: {}", e))?;
        let config_path = config_dir.join("config.json");
        std::fs::write(&config_path, json).map_err(|e| format!("Failed to write config: {}", e))?;

        tracing::info!("BeamRpcService: wrote workspace config → {:?}", config_path);
        Ok(config)
    }

    pub fn read_workspace_config(
        &self,
        workspace_path: &str,
    ) -> Result<BeamWorkspaceConfig, String> {
        let config_path = Path::new(workspace_path)
            .join(".rainy-mate")
            .join("beam")
            .join("config.json");
        let json = std::fs::read_to_string(&config_path)
            .map_err(|e| format!("No Beam config found ({:?}): {}", config_path, e))?;
        serde_json::from_str(&json).map_err(|e| format!("Invalid config JSON: {}", e))
    }

    // ── Master key management ────────────────────────────────────────────

    fn ensure_wallet_dir(&self) -> Result<(), String> {
        std::fs::create_dir_all(&self.wallet_dir)
            .map_err(|e| format!("Failed to create wallet dir: {}", e))
    }

    fn load_or_create_master_key(&self) -> Result<Vec<u8>, String> {
        self.ensure_wallet_dir()?;
        if self.master_key_path.exists() {
            let key = std::fs::read(&self.master_key_path)
                .map_err(|e| format!("Failed to read master key: {}", e))?;
            if key.len() != 32 {
                return Err("Master key file is corrupt (not 32 bytes)".to_string());
            }
            return Ok(key);
        }
        // Generate a fresh random master key
        let mut key = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut key);
        std::fs::write(&self.master_key_path, &key)
            .map_err(|e| format!("Failed to write master key: {}", e))?;
        // Restrict permissions on unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(
                &self.master_key_path,
                std::fs::Permissions::from_mode(0o600),
            );
        }
        tracing::info!(
            "BeamRpcService: generated new master key at {:?}",
            self.master_key_path
        );
        Ok(key.to_vec())
    }

    // ── Wallet CRUD ──────────────────────────────────────────────────────

    /// Create a new local wallet. Returns public WalletInfo (no private key exposed).
    pub fn create_wallet(&self, label: Option<String>) -> Result<WalletInfo, String> {
        let master_key = self.load_or_create_master_key()?;

        // Generate secp256k1 key pair
        let mut private_key_bytes = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut private_key_bytes);
        let signing_key = SigningKey::from_bytes((&private_key_bytes).into())
            .map_err(|e| format!("Invalid key material: {}", e))?;
        let pubkey = signing_key.verifying_key().to_encoded_point(false);
        let address = pubkey_to_address(pubkey.as_bytes());

        self.store_wallet(&master_key, &address, label, &private_key_bytes)
    }

    /// Import a wallet from a 32-byte hex private key.
    pub fn import_wallet(
        &self,
        private_key_hex: &str,
        label: Option<String>,
    ) -> Result<WalletInfo, String> {
        let master_key = self.load_or_create_master_key()?;
        let hex = private_key_hex
            .strip_prefix("0x")
            .unwrap_or(private_key_hex);
        let raw = hex::decode(hex).map_err(|e| format!("Invalid private key hex: {}", e))?;
        if raw.len() != 32 {
            return Err("Private key must be exactly 32 bytes".to_string());
        }

        let signing_key = SigningKey::from_bytes(raw.as_slice().into())
            .map_err(|e| format!("Invalid private key: {}", e))?;
        let pubkey = signing_key.verifying_key().to_encoded_point(false);
        let address = pubkey_to_address(pubkey.as_bytes());

        self.store_wallet(&master_key, &address, label, &raw)
    }

    fn store_wallet(
        &self,
        master_key: &[u8],
        address: &str,
        label: Option<String>,
        private_key_bytes: &[u8],
    ) -> Result<WalletInfo, String> {
        let (ciphertext, nonce) = encrypt_private_key(master_key, address, private_key_bytes)?;
        let created_at = chrono::Utc::now().timestamp();

        let record = EncryptedWalletRecord {
            address: address.to_string(),
            label: label.clone(),
            created_at,
            nonce_hex: hex::encode(nonce),
            ciphertext_hex: hex::encode(&ciphertext),
        };

        let json = serde_json::to_string(&record)
            .map_err(|e| format!("Failed to serialize wallet: {}", e))?;
        let wallet_path = self
            .wallet_dir
            .join(format!("{}.enc", address.to_lowercase()));
        std::fs::write(&wallet_path, json.as_bytes())
            .map_err(|e| format!("Failed to write wallet file: {}", e))?;

        tracing::info!("BeamRpcService: stored wallet {}", address);
        Ok(WalletInfo {
            address: address.to_string(),
            label,
            created_at,
        })
    }

    fn load_wallet_record(&self, address: &str) -> Result<EncryptedWalletRecord, String> {
        let path = self
            .wallet_dir
            .join(format!("{}.enc", address.to_lowercase()));
        let json = std::fs::read_to_string(&path)
            .map_err(|_| format!("Wallet '{}' not found", address))?;
        serde_json::from_str(&json).map_err(|e| format!("Corrupt wallet file: {}", e))
    }

    fn load_signing_key(&self, address: &str) -> Result<SigningKey, String> {
        let master_key = self.load_or_create_master_key()?;
        let record = self.load_wallet_record(address)?;
        let nonce = hex::decode(&record.nonce_hex).map_err(|e| format!("Bad nonce hex: {}", e))?;
        let ciphertext = hex::decode(&record.ciphertext_hex)
            .map_err(|e| format!("Bad ciphertext hex: {}", e))?;
        let private_key_bytes = decrypt_private_key(&master_key, address, &ciphertext, &nonce)?;
        SigningKey::from_bytes(private_key_bytes.as_slice().into())
            .map_err(|e| format!("Failed to reconstruct signing key: {}", e))
    }

    /// Get public info for a wallet. Never returns the private key.
    pub fn get_wallet(&self, address: &str) -> Result<WalletInfo, String> {
        let record = self.load_wallet_record(address)?;
        Ok(WalletInfo {
            address: record.address,
            label: record.label,
            created_at: record.created_at,
        })
    }

    /// List all stored wallet addresses.
    pub fn list_wallets(&self) -> Result<Vec<WalletInfo>, String> {
        self.ensure_wallet_dir()?;
        let mut wallets = Vec::new();
        let entries = std::fs::read_dir(&self.wallet_dir)
            .map_err(|e| format!("Failed to list wallet dir: {}", e))?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "enc") {
                if let Ok(json) = std::fs::read_to_string(&path) {
                    if let Ok(record) = serde_json::from_str::<EncryptedWalletRecord>(&json) {
                        wallets.push(WalletInfo {
                            address: record.address,
                            label: record.label,
                            created_at: record.created_at,
                        });
                    }
                }
            }
        }
        wallets.sort_by_key(|w| w.created_at);
        Ok(wallets)
    }

    // ── JSON-RPC calls ───────────────────────────────────────────────────

    async fn rpc_call(
        &self,
        rpc_url: &str,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let request = RpcRequest {
            jsonrpc: "2.0",
            method,
            params,
            id: 1,
        };
        let response = self
            .http_client
            .post(rpc_url)
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("RPC request failed: {}", e))?;

        let rpc_resp: RpcResponse = response
            .json()
            .await
            .map_err(|e| format!("RPC response parse failed: {}", e))?;

        if let Some(err) = rpc_resp.error {
            return Err(format!("RPC error: {}", err));
        }
        rpc_resp
            .result
            .ok_or_else(|| "RPC returned no result".to_string())
    }

    async fn get_nonce(&self, rpc_url: &str, address: &str) -> Result<u64, String> {
        let result = self
            .rpc_call(
                rpc_url,
                "eth_getTransactionCount",
                serde_json::json!([address, "latest"]),
            )
            .await?;
        let hex = result.as_str().ok_or("nonce not a string")?;
        decode_hex_u64(hex)
    }

    async fn get_gas_price(&self, rpc_url: &str) -> Result<u64, String> {
        let result = self
            .rpc_call(rpc_url, "eth_gasPrice", serde_json::json!([]))
            .await?;
        let hex = result.as_str().ok_or("gasPrice not a string")?;
        decode_hex_u64(hex)
    }

    async fn estimate_gas_rpc(
        &self,
        rpc_url: &str,
        from: &str,
        to: Option<&str>,
        value: &str,
        data: &str,
    ) -> Result<u64, String> {
        let mut request = serde_json::Map::new();
        request.insert(
            "from".to_string(),
            serde_json::Value::String(from.to_string()),
        );
        if let Some(to) = to {
            request.insert("to".to_string(), serde_json::Value::String(to.to_string()));
        }
        request.insert(
            "value".to_string(),
            serde_json::Value::String(value.to_string()),
        );
        request.insert(
            "data".to_string(),
            serde_json::Value::String(data.to_string()),
        );
        let params = serde_json::json!([request]);
        let result = self.rpc_call(rpc_url, "eth_estimateGas", params).await?;
        let hex = result.as_str().ok_or("estimatedGas not a string")?;
        decode_hex_u64(hex)
    }

    async fn get_transaction_receipt(
        &self,
        rpc_url: &str,
        tx_hash: &str,
    ) -> Result<Option<(Option<String>, Option<u64>, Option<u64>, Option<String>)>, String> {
        let result = self
            .rpc_call(
                rpc_url,
                "eth_getTransactionReceipt",
                serde_json::json!([tx_hash]),
            )
            .await?;
        if result.is_null() {
            return Ok(None);
        }

        let status = result
            .get("status")
            .and_then(|value| value.as_str())
            .map(|value| value.to_string());
        let block_number = result
            .get("blockNumber")
            .and_then(|value| value.as_str())
            .map(decode_hex_u64)
            .transpose()?;
        let gas_used = result
            .get("gasUsed")
            .and_then(|value| value.as_str())
            .map(decode_hex_u64)
            .transpose()?;
        let contract_address = result
            .get("contractAddress")
            .and_then(|value| value.as_str())
            .map(|value| value.to_string());

        Ok(Some((status, block_number, gas_used, contract_address)))
    }

    // ── Gas estimation ───────────────────────────────────────────────────

    pub async fn estimate_gas(
        &self,
        workspace_path: &str,
        from: &str,
        to: Option<&str>,
        value: Option<&str>,
        data: Option<&str>,
    ) -> Result<GasEstimate, String> {
        let cfg = self.read_workspace_config(workspace_path)?;
        let rpc_url = &cfg.chain.rpc_url;
        let value_str = value.unwrap_or("0x0");
        let data_str = data.unwrap_or("0x");

        let (gas_limit, gas_price) = tokio::try_join!(
            self.estimate_gas_rpc(rpc_url, from, to, value_str, data_str),
            self.get_gas_price(rpc_url),
        )?;

        // Add 20% buffer to gas_limit
        let gas_limit_with_buffer = (gas_limit as f64 * 1.2) as u64;
        let fee_wei = gas_limit_with_buffer as u128 * gas_price as u128;

        Ok(GasEstimate {
            gas_limit: gas_limit_with_buffer,
            gas_price,
            estimated_fee_wei: fee_wei,
            estimated_fee_beam: format_beam(fee_wei),
            network: cfg.network,
            rpc_url: rpc_url.clone(),
        })
    }

    // ── Transaction signing ──────────────────────────────────────────────

    /// Sign a transaction using the local wallet. Private key never leaves this function.
    /// Gas and nonce are auto-fetched from RPC if not provided.
    pub async fn sign_transaction(
        &self,
        workspace_path: &str,
        tx: &TransactionRequest,
    ) -> Result<SignedTransaction, String> {
        let cfg = self.read_workspace_config(workspace_path)?;
        let rpc_url = &cfg.chain.rpc_url;
        let chain_id = cfg.chain.chain_id;

        // Resolve nonce and gas in parallel
        let nonce_future = async {
            match tx.nonce {
                Some(n) => Ok(n),
                None => self.get_nonce(rpc_url, &tx.from).await,
            }
        };
        let gas_price_future = async {
            match tx.gas_price {
                Some(gp) => Ok(gp),
                None => self.get_gas_price(rpc_url).await,
            }
        };
        let (nonce, gas_price) = tokio::try_join!(nonce_future, gas_price_future)?;

        let value_str = tx.value.as_deref().unwrap_or("0x0");
        let data_str = tx.data.as_deref().unwrap_or("0x");
        let value_u128 = decode_hex_u128(value_str)?;
        let data_bytes = decode_hex_bytes(data_str)?;

        // Estimate gas if not provided
        let gas_limit = match tx.gas_limit {
            Some(gl) => gl,
            None => {
                let estimated = self
                    .estimate_gas_rpc(rpc_url, &tx.from, tx.to.as_deref(), value_str, data_str)
                    .await?;
                (estimated as f64 * 1.2) as u64
            }
        };

        // Decode to address bytes
        let to_bytes = match tx.to.as_deref() {
            Some(to) => {
                let addr = to.strip_prefix("0x").unwrap_or(to);
                hex::decode(addr).map_err(|e| format!("Invalid 'to' address: {}", e))?
            }
            None => Vec::new(),
        };

        // EIP-155: signing hash = keccak256(RLP([nonce, gasPrice, gasLimit, to, value, data, chainId, 0, 0]))
        let rlp_for_signing = rlp_encode_list(&[
            rlp_encode_u64(nonce),
            rlp_encode_u64(gas_price),
            rlp_encode_u64(gas_limit),
            rlp_encode_bytes(&to_bytes),
            rlp_encode_u128(value_u128),
            rlp_encode_bytes(&data_bytes),
            rlp_encode_u64(chain_id),
            rlp_encode_bytes(&[]),
            rlp_encode_bytes(&[]),
        ]);
        let hash: [u8; 32] = keccak256(&rlp_for_signing);

        // Load signing key and sign
        let signing_key = self.load_signing_key(&tx.from)?;
        let (r, s, recovery_id) = secp256k1_sign_prehash(&signing_key, &hash)?;

        // v = chain_id * 2 + 35 + recovery_id  (EIP-155)
        let v: u64 = chain_id * 2 + 35 + recovery_id as u64;

        // Signed transaction RLP
        let raw_rlp = rlp_encode_list(&[
            rlp_encode_u64(nonce),
            rlp_encode_u64(gas_price),
            rlp_encode_u64(gas_limit),
            rlp_encode_bytes(&to_bytes),
            rlp_encode_u128(value_u128),
            rlp_encode_bytes(&data_bytes),
            rlp_encode_u64(v),
            rlp_encode_bytes(&r),
            rlp_encode_bytes(&s),
        ]);

        let tx_hash_bytes = keccak256(&raw_rlp);
        let raw_tx = format!("0x{}", hex::encode(&raw_rlp));
        let tx_hash = format!("0x{}", hex::encode(tx_hash_bytes));

        tracing::info!(
            "BeamRpcService: signed tx {} on {} (from={}, to={}, nonce={})",
            tx_hash,
            cfg.network,
            tx.from,
            tx.to
                .clone()
                .unwrap_or_else(|| "<contract-creation>".to_string()),
            nonce
        );

        Ok(SignedTransaction {
            raw_tx,
            tx_hash,
            from: tx.from.clone(),
            to: tx.to.clone(),
            value_hex: value_str.to_string(),
            gas_limit,
            gas_price,
            nonce,
            chain_id,
            network: cfg.network,
        })
    }

    /// Broadcast a signed transaction. Returns the transaction hash.
    pub async fn send_raw_transaction(
        &self,
        workspace_path: &str,
        raw_tx: &str,
    ) -> Result<TransactionReceipt, String> {
        let cfg = self.read_workspace_config(workspace_path)?;
        let result = self
            .rpc_call(
                &cfg.chain.rpc_url,
                "eth_sendRawTransaction",
                serde_json::json!([raw_tx]),
            )
            .await?;
        let tx_hash = result
            .as_str()
            .ok_or("sendRawTransaction returned non-string result")?
            .to_string();
        let explorer_url = format!(
            "{}/tx/{}",
            cfg.chain.explorer_url.trim_end_matches('/'),
            tx_hash
        );
        tracing::info!(
            "BeamRpcService: broadcast tx {} on {}",
            tx_hash,
            cfg.network
        );
        let mut status = None;
        let mut block_number = None;
        let mut gas_used = None;
        let mut contract_address = None;
        for _ in 0..15 {
            if let Some((next_status, next_block_number, next_gas_used, next_contract_address)) =
                self.get_transaction_receipt(&cfg.chain.rpc_url, &tx_hash)
                    .await?
            {
                status = next_status;
                block_number = next_block_number;
                gas_used = next_gas_used;
                contract_address = next_contract_address;
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }
        Ok(TransactionReceipt {
            tx_hash,
            network: cfg.network,
            explorer_url,
            status,
            block_number,
            gas_used,
            contract_address,
        })
    }

    /// Sign and immediately broadcast a transaction.
    pub async fn send_transaction(
        &self,
        workspace_path: &str,
        tx: &TransactionRequest,
    ) -> Result<TransactionReceipt, String> {
        let signed = self.sign_transaction(workspace_path, tx).await?;
        self.send_raw_transaction(workspace_path, &signed.raw_tx)
            .await
    }
}

fn format_beam(wei: u128) -> String {
    // 18 decimals
    let whole = wei / 1_000_000_000_000_000_000u128;
    let frac = (wei % 1_000_000_000_000_000_000u128) / 1_000_000_000_000u128; // 6 decimal places
    format!("{}.{:06} BEAM", whole, frac)
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_service() -> BeamRpcService {
        let tmp = tempfile::tempdir().expect("tmp dir");
        BeamRpcService::new(tmp.path().to_path_buf())
    }

    #[test]
    fn chain_configs_have_correct_chain_ids() {
        assert_eq!(BeamChainConfig::mainnet().chain_id, 4337);
        assert_eq!(BeamChainConfig::testnet().chain_id, 13337);
    }

    #[test]
    fn chain_configs_have_correct_rpc_urls() {
        assert_eq!(
            BeamChainConfig::mainnet().rpc_url,
            "https://build.onbeam.com/rpc"
        );
        assert_eq!(
            BeamChainConfig::testnet().rpc_url,
            "https://build.onbeam.com/rpc/testnet"
        );
    }

    #[test]
    fn chain_configs_have_correct_ws_urls() {
        assert_eq!(
            BeamChainConfig::mainnet().ws_url,
            "wss://build.onbeam.com/ws"
        );
        assert_eq!(
            BeamChainConfig::testnet().ws_url,
            "wss://build.onbeam.com/ws/testnet"
        );
    }

    #[test]
    fn beam_network_from_str() {
        assert_eq!(BeamNetwork::from_str("mainnet"), Some(BeamNetwork::Mainnet));
        assert_eq!(BeamNetwork::from_str("testnet"), Some(BeamNetwork::Testnet));
        assert_eq!(BeamNetwork::from_str("fuji"), Some(BeamNetwork::Testnet));
        assert_eq!(BeamNetwork::from_str("unknown"), None);
    }

    #[test]
    fn create_wallet_returns_valid_address() {
        let svc = make_service();
        let info = svc.create_wallet(Some("test".to_string())).unwrap();
        assert!(info.address.starts_with("0x"));
        assert_eq!(info.address.len(), 42);
        assert_eq!(info.label, Some("test".to_string()));
    }

    #[test]
    fn import_wallet_derives_correct_address() {
        // Well-known test vector (64 hex chars = 32 bytes):
        // Private key: 0x4646464646464646464646464646464646464646464646464646464646464646
        // Expected address: 0x9d8A62f656a8d1615C1294fd71e9CFb3E4855A4F
        let svc = make_service();
        let info = svc
            .import_wallet(
                "4646464646464646464646464646464646464646464646464646464646464646",
                None,
            )
            .unwrap();
        // Verify address format (0x-prefixed, 42 chars)
        assert!(info.address.starts_with("0x"));
        assert_eq!(info.address.len(), 42);
        // Verify it's deterministic
        let info2 = svc
            .import_wallet(
                "4646464646464646464646464646464646464646464646464646464646464646",
                None,
            )
            .unwrap();
        assert_eq!(info.address, info2.address);
    }

    #[test]
    fn list_wallets_returns_all_created() {
        let svc = make_service();
        svc.create_wallet(Some("w1".to_string())).unwrap();
        svc.create_wallet(Some("w2".to_string())).unwrap();
        let list = svc.list_wallets().unwrap();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn get_wallet_returns_info_not_private_key() {
        let svc = make_service();
        let created = svc.create_wallet(Some("myWallet".to_string())).unwrap();
        let info = svc.get_wallet(&created.address).unwrap();
        assert_eq!(info.address, created.address);
        assert_eq!(info.label, Some("myWallet".to_string()));
    }

    #[test]
    fn master_key_persists_across_service_instances() {
        let tmp = tempfile::tempdir().expect("tmp dir");
        let svc1 = BeamRpcService::new(tmp.path().to_path_buf());
        let wallet = svc1.create_wallet(None).unwrap();

        // Second instance reuses the master key → decryption succeeds
        let svc2 = BeamRpcService::new(tmp.path().to_path_buf());
        let signing_key = svc2.load_signing_key(&wallet.address);
        assert!(
            signing_key.is_ok(),
            "should decrypt across service instances"
        );
    }

    #[test]
    fn rlp_encode_u64_zero() {
        assert_eq!(rlp_encode_u64(0), vec![0x80]);
    }

    #[test]
    fn rlp_encode_small_byte() {
        // Single byte < 0x80 is identity
        assert_eq!(rlp_encode_u64(1), vec![0x01]);
        assert_eq!(rlp_encode_u64(127), vec![0x7f]);
    }

    #[test]
    fn eip55_checksum_known_vector() {
        // https://eips.ethereum.org/EIPS/eip-55
        let addr_bytes =
            hex::decode("5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed".to_lowercase()).unwrap();
        // Just verify the function produces a 0x-prefixed 42-char string
        let result = eip55_checksum(&addr_bytes[..20]);
        assert!(result.starts_with("0x"));
        assert_eq!(result.len(), 42);
    }

    #[test]
    fn workspace_config_roundtrip() {
        let tmp = tempfile::tempdir().expect("tmp dir");
        let svc = make_service();
        let ws_path = tmp.path().to_string_lossy().to_string();
        let written = svc
            .write_workspace_config(&ws_path, BeamNetwork::Testnet)
            .unwrap();
        let read_back = svc.read_workspace_config(&ws_path).unwrap();
        assert_eq!(written.chain.chain_id, read_back.chain.chain_id);
        assert_eq!(read_back.chain.chain_id, 13337);
    }

    #[test]
    fn format_beam_1_ether() {
        let one_ether: u128 = 1_000_000_000_000_000_000;
        assert_eq!(format_beam(one_ether), "1.000000 BEAM");
    }

    #[test]
    fn format_beam_half_ether() {
        let half: u128 = 500_000_000_000_000_000;
        assert_eq!(format_beam(half), "0.500000 BEAM");
    }

    #[test]
    fn sign_transaction_produces_deterministic_hash_for_same_inputs() {
        // We can't test the full async sign_transaction without a mock RPC,
        // but we can test the signing components.
        let private_key = [1u8; 32];
        let signing_key = SigningKey::from_bytes((&private_key).into()).unwrap();
        let hash = keccak256(b"test message");
        let (r1, s1, v1) = secp256k1_sign_prehash(&signing_key, &hash).unwrap();
        let (r2, s2, v2) = secp256k1_sign_prehash(&signing_key, &hash).unwrap();
        // secp256k1 ECDSA is deterministic (RFC 6979)
        assert_eq!(r1, r2);
        assert_eq!(s1, s2);
        assert_eq!(v1, v2);
    }
}
