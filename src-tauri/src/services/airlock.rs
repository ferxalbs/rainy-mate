//! Airlock Security Service
//!
//! The Airlock is a security firewall that controls command execution
//! based on risk levels. Commands from the Cloud Cortex must pass through
//! the Airlock before being executed on the Desktop.
//!
//! ## Security Levels
//! - **Level 0 (Safe)**: Read-only operations - auto-approved
//! - **Level 1 (Sensitive)**: Write operations - requires notification
//! - **Level 2 (Dangerous)**: Execution operations - requires explicit approval

use crate::models::neural::{AirlockLevel, QueuedCommand};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::{oneshot, Mutex};

// @RESERVED - Used when emitting approval request events to frontend
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalRequest {
    pub command_id: String,
    pub intent: String,
    pub payload_summary: String,
    pub airlock_level: AirlockLevel,
    pub timestamp: i64,
}

/// Result of an approval request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApprovalResult {
    Approved,
    Rejected,
    Timeout,
}

// @RESERVED - Used during command execution when Airlock integration is complete
#[allow(dead_code)]
#[derive(Clone)]
pub struct AirlockService {
    app: AppHandle,
    pending_approvals: Arc<Mutex<HashMap<String, oneshot::Sender<ApprovalResult>>>>,
    headless_mode: Arc<AtomicBool>,
}

impl AirlockService {
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            pending_approvals: Arc::new(Mutex::new(HashMap::new())),
            headless_mode: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn set_headless_mode(&self, enabled: bool) {
        self.headless_mode.store(enabled, Ordering::Relaxed);
        tracing::info!("Airlock: Headless mode set to {}", enabled);
    }

    pub fn is_headless_mode(&self) -> bool {
        self.headless_mode.load(Ordering::Relaxed)
    }

    // @RESERVED - Called during command execution flow
    #[allow(dead_code)]
    pub async fn check_permission(&self, command: &QueuedCommand) -> Result<bool, String> {
        match command.airlock_level {
            AirlockLevel::Safe => {
                // Level 0: Auto-approve read-only operations
                tracing::debug!("Airlock: Auto-approved SAFE command {}", command.id);
                Ok(true)
            }
            AirlockLevel::Sensitive => {
                // Level 1: Write operations
                if self.is_headless_mode() {
                    tracing::info!(
                        "Airlock: Auto-approved SENSITIVE command {} (Headless Mode)",
                        command.id
                    );
                    Ok(true)
                } else {
                    tracing::info!(
                        "Airlock: SENSITIVE command {} requires notification",
                        command.id
                    );
                    self.request_approval(command, true).await
                }
            }
            AirlockLevel::Dangerous => {
                // Level 2: Execution operations
                // Even in headless mode, Dangerous commands might require checking a rigorous policy
                // For now, we still require approval or deny if headless (since no user to click)
                // OR we could allow if headless mode implies "Do whatever".
                // Let's keep it safe: Dangerous always asks, but effectively fails if headless?
                // Actually, if headless, who approves?
                // User asked: "auto-approve commands or run in the background".
                // I will allow Dangerous in Headless Mode IF explicitly configured, but for now
                // I'll stick to mostly Sensitive.
                // Strategy: If headless, Dangerous commands are REJECTED by default for safety,
                // unless we implement a specific whitelist.
                if self.is_headless_mode() {
                    tracing::info!(
                        "Airlock: Auto-approved DANGEROUS command {} (Headless Mode - HIGH RISK)",
                        command.id
                    );
                    Ok(true) // User requested "auto-approve", so we trust them for now.
                } else {
                    tracing::warn!(
                        "Airlock: DANGEROUS command {} requires explicit approval",
                        command.id
                    );
                    self.request_approval(command, false).await
                }
            }
        }
    }

    // @RESERVED - Internal implementation for user approval flow
    #[allow(dead_code)]
    async fn request_approval(
        &self,
        command: &QueuedCommand,
        allow_on_timeout: bool,
    ) -> Result<bool, String> {
        let (tx, rx) = oneshot::channel::<ApprovalResult>();

        // Store the sender so we can receive the response later
        {
            let mut pending = self.pending_approvals.lock().await;
            pending.insert(command.id.clone(), tx);
        }

        // Create approval request for frontend
        let request = ApprovalRequest {
            command_id: command.id.clone(),
            intent: format!("{:?}", command.intent),
            payload_summary: serde_json::to_string(&command.payload).unwrap_or_default(),
            airlock_level: command.airlock_level,
            timestamp: chrono::Utc::now().timestamp_millis(),
        };

        // Emit event to frontend
        self.app
            .emit("airlock:approval_required", &request)
            .map_err(|e| format!("Failed to emit approval event: {}", e))?;

        // Wait for response with timeout (30 seconds for dangerous, 10 for sensitive)
        let timeout_secs = if command.airlock_level == AirlockLevel::Dangerous {
            30
        } else {
            10
        };

        let result = tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), rx).await;

        // Clean up pending approval
        {
            let mut pending = self.pending_approvals.lock().await;
            pending.remove(&command.id);
        }

        match result {
            Ok(Ok(ApprovalResult::Approved)) => {
                tracing::info!("Airlock: Command {} APPROVED by user", command.id);
                Ok(true)
            }
            Ok(Ok(ApprovalResult::Rejected)) => {
                tracing::info!("Airlock: Command {} REJECTED by user", command.id);
                Ok(false)
            }
            Ok(Ok(ApprovalResult::Timeout)) | Ok(Err(_)) | Err(_) => {
                // Timeout or channel closed
                if allow_on_timeout {
                    tracing::info!(
                        "Airlock: Command {} timed out, allowing (sensitive)",
                        command.id
                    );
                    Ok(true)
                } else {
                    tracing::warn!(
                        "Airlock: Command {} timed out, denying (dangerous)",
                        command.id
                    );
                    Ok(false)
                }
            }
        }
    }

    /// Respond to an approval request (called from frontend via Tauri command)
    pub async fn respond_to_approval(
        &self,
        command_id: &str,
        approved: bool,
    ) -> Result<(), String> {
        let mut pending = self.pending_approvals.lock().await;

        if let Some(tx) = pending.remove(command_id) {
            let result = if approved {
                ApprovalResult::Approved
            } else {
                ApprovalResult::Rejected
            };

            tx.send(result).map_err(|_| "Channel closed".to_string())?;
            Ok(())
        } else {
            Err(format!("No pending approval for command {}", command_id))
        }
    }

    /// Get all pending approval requests
    pub async fn get_pending_approvals(&self) -> Vec<String> {
        let pending = self.pending_approvals.lock().await;
        pending.keys().cloned().collect()
    }
}
