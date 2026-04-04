//! Beam EVM skill executor — dispatches the "evm" skill namespace.
//!
//! All signing operations have already passed Airlock L2 before reaching here.
//! This layer is responsible only for parsing args and calling BeamRpcService.

use super::args::*;
use super::SkillExecutor;
use crate::models::neural::CommandResult;
use crate::services::beam_rpc::{BeamNetwork, TransactionRequest};
use serde_json::Value;

impl SkillExecutor {
    /// Dispatch handler for the "evm" skill namespace.
    pub(super) async fn execute_evm(
        &self,
        workspace_id: String,
        method: &str,
        params: &Option<Value>,
    ) -> CommandResult {
        let beam_lock = self.beam_rpc.read().await;
        let beam = match beam_lock.as_ref() {
            Some(svc) => svc.clone(),
            None => return self.error("BeamRpcService not initialized — restart the application"),
        };
        drop(beam_lock); // release lock before async operations below

        // Resolve the workspace root path (needed for config read/write)
        let workspace_path = match self.workspace_manager.load_workspace(&workspace_id) {
            Ok(ws) => {
                // Use the first allowed path as the workspace root
                ws.allowed_paths.into_iter().next().unwrap_or_default()
            }
            Err(_) => String::new(),
        };

        let params = match params {
            Some(p) => p,
            None => return self.error("Missing parameters"),
        };

        match method {
            // ── beam_rpc_connect ─────────────────────────────────────────
            "beam_rpc_connect" => {
                let args: BeamRpcConnectArgs = match serde_json::from_value(params.clone()) {
                    Ok(a) => a,
                    Err(e) => return self.error(&format!("Invalid parameters: {}", e)),
                };
                if workspace_path.is_empty() {
                    return self.error("Cannot connect: workspace has no allowed paths configured");
                }
                let network = match BeamNetwork::from_str(&args.network) {
                    Some(n) => n,
                    None => {
                        return self.error(&format!(
                            "Unknown network '{}'. Use 'mainnet' or 'testnet'",
                            args.network
                        ))
                    }
                };
                match beam.write_workspace_config(&workspace_path, network) {
                    Ok(cfg) => CommandResult {
                        success: true,
                        output: Some(serde_json::to_string_pretty(&cfg).unwrap_or_default()),
                        error: None,
                        exit_code: Some(0),
                    },
                    Err(e) => self.error(&e),
                }
            }

            // ── beam_create_wallet ───────────────────────────────────────
            "beam_create_wallet" => {
                let args: BeamCreateWalletArgs = match serde_json::from_value(params.clone()) {
                    Ok(a) => a,
                    Err(e) => return self.error(&format!("Invalid parameters: {}", e)),
                };
                match beam.create_wallet(args.label) {
                    Ok(info) => CommandResult {
                        success: true,
                        output: Some(serde_json::to_string(&info).unwrap_or_default()),
                        error: None,
                        exit_code: Some(0),
                    },
                    Err(e) => self.error(&e),
                }
            }

            // ── beam_import_wallet ───────────────────────────────────────
            "beam_import_wallet" => {
                let args: BeamImportWalletArgs = match serde_json::from_value(params.clone()) {
                    Ok(a) => a,
                    Err(e) => return self.error(&format!("Invalid parameters: {}", e)),
                };
                match beam.import_wallet(&args.private_key_hex, args.label) {
                    Ok(info) => CommandResult {
                        success: true,
                        output: Some(serde_json::to_string(&info).unwrap_or_default()),
                        error: None,
                        exit_code: Some(0),
                    },
                    Err(e) => self.error(&e),
                }
            }

            // ── beam_get_wallet ──────────────────────────────────────────
            "beam_get_wallet" => {
                let args: BeamGetWalletArgs = match serde_json::from_value(params.clone()) {
                    Ok(a) => a,
                    Err(e) => return self.error(&format!("Invalid parameters: {}", e)),
                };
                match beam.get_wallet(&args.address) {
                    Ok(info) => CommandResult {
                        success: true,
                        output: Some(serde_json::to_string(&info).unwrap_or_default()),
                        error: None,
                        exit_code: Some(0),
                    },
                    Err(e) => self.error(&e),
                }
            }

            // ── beam_list_wallets ────────────────────────────────────────
            "beam_list_wallets" => match beam.list_wallets() {
                Ok(wallets) => CommandResult {
                    success: true,
                    output: Some(serde_json::to_string(&wallets).unwrap_or_default()),
                    error: None,
                    exit_code: Some(0),
                },
                Err(e) => self.error(&e),
            },

            // ── beam_estimate_gas ────────────────────────────────────────
            "beam_estimate_gas" => {
                let args: BeamEstimateGasArgs = match serde_json::from_value(params.clone()) {
                    Ok(a) => a,
                    Err(e) => return self.error(&format!("Invalid parameters: {}", e)),
                };
                if workspace_path.is_empty() {
                    return self.error("Workspace path required — run beam_rpc_connect first");
                }
                match beam
                    .estimate_gas(
                        &workspace_path,
                        &args.from,
                        Some(&args.to),
                        args.value.as_deref(),
                        args.data.as_deref(),
                    )
                    .await
                {
                    Ok(estimate) => CommandResult {
                        success: true,
                        output: Some(serde_json::to_string(&estimate).unwrap_or_default()),
                        error: None,
                        exit_code: Some(0),
                    },
                    Err(e) => self.error(&e),
                }
            }

            // ── beam_sign_transaction ────────────────────────────────────
            // All signatures have passed Airlock L2 before this point.
            "beam_sign_transaction" => {
                let args: BeamSignTransactionArgs = match serde_json::from_value(params.clone()) {
                    Ok(a) => a,
                    Err(e) => return self.error(&format!("Invalid parameters: {}", e)),
                };
                if workspace_path.is_empty() {
                    return self.error("Workspace path required — run beam_rpc_connect first");
                }
                let tx = TransactionRequest {
                    from: args.from,
                    to: Some(args.to),
                    value: args.value,
                    data: args.data,
                    gas_limit: args.gas_limit,
                    gas_price: args.gas_price,
                    nonce: args.nonce,
                };
                match beam.sign_transaction(&workspace_path, &tx).await {
                    Ok(signed) => CommandResult {
                        success: true,
                        output: Some(serde_json::to_string(&signed).unwrap_or_default()),
                        error: None,
                        exit_code: Some(0),
                    },
                    Err(e) => self.error(&e),
                }
            }

            // ── beam_send_transaction ────────────────────────────────────
            // All broadcasts have passed Airlock L2 before this point.
            "beam_send_transaction" => {
                let args: BeamSendTransactionArgs = match serde_json::from_value(params.clone()) {
                    Ok(a) => a,
                    Err(e) => return self.error(&format!("Invalid parameters: {}", e)),
                };
                if workspace_path.is_empty() {
                    return self.error("Workspace path required — run beam_rpc_connect first");
                }
                let tx = TransactionRequest {
                    from: args.from,
                    to: Some(args.to),
                    value: args.value,
                    data: args.data,
                    gas_limit: args.gas_limit,
                    gas_price: args.gas_price,
                    nonce: args.nonce,
                };
                match beam.send_transaction(&workspace_path, &tx).await {
                    Ok(receipt) => CommandResult {
                        success: true,
                        output: Some(serde_json::to_string(&receipt).unwrap_or_default()),
                        error: None,
                        exit_code: Some(0),
                    },
                    Err(e) => self.error(&e),
                }
            }

            _ => self.error(&format!("Unknown evm method: {}", method)),
        }
    }
}
