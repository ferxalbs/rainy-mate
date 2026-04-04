//! Tauri commands for the Beam RPC + Secure Local Signing Bridge.
//!
//! These commands are invokable directly from the frontend (WorkspaceLaunchpad,
//! BeamChainCard, etc.) without going through the agent/ATM pipeline.
//! They complement the agent-accessible "evm" skill tools.

use crate::services::beam_rpc::{
    BeamChainConfig, BeamNetwork, BeamRpcService, BeamWorkspaceConfig, GasEstimate, SignedTransaction,
    TransactionReceipt, TransactionRequest, WalletInfo,
};
use std::sync::Arc;
use tauri::State;

// ── Chain config (static — no service state required) ────────────────────────

/// Returns both Mainnet and Testnet chain configurations.
#[tauri::command]
pub async fn get_beam_chain_configs() -> Result<serde_json::Value, String> {
    Ok(BeamRpcService::all_chain_configs())
}

/// Returns the chain configuration for a specific network.
#[tauri::command]
pub async fn get_beam_chain_config(network: String) -> Result<BeamChainConfig, String> {
    let net = BeamNetwork::from_str(&network)
        .ok_or_else(|| format!("Unknown network '{}'. Use 'mainnet' or 'testnet'.", network))?;
    Ok(BeamRpcService::chain_config(net))
}

// ── Workspace connection ──────────────────────────────────────────────────────

/// Write `.rainy-mate/beam/config.json` for the workspace path and chosen network.
#[tauri::command]
pub async fn connect_beam_workspace(
    workspace_path: String,
    network: String,
    beam_rpc: State<'_, Arc<BeamRpcService>>,
) -> Result<BeamWorkspaceConfig, String> {
    let net = BeamNetwork::from_str(&network)
        .ok_or_else(|| format!("Unknown network '{}'. Use 'mainnet' or 'testnet'.", network))?;
    beam_rpc.write_workspace_config(&workspace_path, net)
}

/// Read the existing `.rainy-mate/beam/config.json` for the workspace.
#[tauri::command]
pub async fn get_beam_workspace_config(
    workspace_path: String,
    beam_rpc: State<'_, Arc<BeamRpcService>>,
) -> Result<BeamWorkspaceConfig, String> {
    beam_rpc.read_workspace_config(&workspace_path)
}

// ── Wallet management ─────────────────────────────────────────────────────────

/// Create a new encrypted local wallet.
#[tauri::command]
pub async fn create_beam_wallet(
    label: Option<String>,
    beam_rpc: State<'_, Arc<BeamRpcService>>,
) -> Result<WalletInfo, String> {
    beam_rpc.create_wallet(label)
}

/// Import a wallet from a 32-byte hex private key.
/// The key is encrypted on-device and never stored in plaintext.
#[tauri::command]
pub async fn import_beam_wallet(
    private_key_hex: String,
    label: Option<String>,
    beam_rpc: State<'_, Arc<BeamRpcService>>,
) -> Result<WalletInfo, String> {
    beam_rpc.import_wallet(&private_key_hex, label)
}

/// Get public info for a locally stored wallet (address + label only).
#[tauri::command]
pub async fn get_beam_wallet(
    address: String,
    beam_rpc: State<'_, Arc<BeamRpcService>>,
) -> Result<WalletInfo, String> {
    beam_rpc.get_wallet(&address)
}

/// List all locally stored wallet addresses and labels.
#[tauri::command]
pub async fn list_beam_wallets(
    beam_rpc: State<'_, Arc<BeamRpcService>>,
) -> Result<Vec<WalletInfo>, String> {
    beam_rpc.list_wallets()
}

// ── Gas estimation ────────────────────────────────────────────────────────────

/// Estimate gas for an EVM transaction on the workspace-connected Beam network.
#[tauri::command]
pub async fn estimate_beam_gas(
    workspace_path: String,
    from: String,
    to: String,
    value: Option<String>,
    data: Option<String>,
    beam_rpc: State<'_, Arc<BeamRpcService>>,
) -> Result<GasEstimate, String> {
    beam_rpc
        .estimate_gas(&workspace_path, &from, &to, value.as_deref(), data.as_deref())
        .await
}

// ── Transaction signing (Secure Local Signing Bridge) ────────────────────────

/// Sign a transaction with a local wallet (L2 gate is enforced by the Airlock before this point
/// when called via the agent. Direct frontend calls should only be used in trusted contexts).
#[tauri::command]
pub async fn sign_beam_transaction(
    workspace_path: String,
    from: String,
    to: String,
    value: Option<String>,
    data: Option<String>,
    gas_limit: Option<u64>,
    gas_price: Option<u64>,
    nonce: Option<u64>,
    beam_rpc: State<'_, Arc<BeamRpcService>>,
) -> Result<SignedTransaction, String> {
    let tx = TransactionRequest {
        from,
        to,
        value,
        data,
        gas_limit,
        gas_price,
        nonce,
    };
    beam_rpc.sign_transaction(&workspace_path, &tx).await
}

/// Sign and broadcast a transaction to the connected Beam network.
#[tauri::command]
pub async fn send_beam_transaction(
    workspace_path: String,
    from: String,
    to: String,
    value: Option<String>,
    data: Option<String>,
    gas_limit: Option<u64>,
    gas_price: Option<u64>,
    nonce: Option<u64>,
    beam_rpc: State<'_, Arc<BeamRpcService>>,
) -> Result<TransactionReceipt, String> {
    let tx = TransactionRequest {
        from,
        to,
        value,
        data,
        gas_limit,
        gas_price,
        nonce,
    };
    beam_rpc.send_transaction(&workspace_path, &tx).await
}
