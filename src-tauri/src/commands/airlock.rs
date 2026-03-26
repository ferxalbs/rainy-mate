//! Airlock Commands
//!
//! Tauri commands for the Airlock security system.
//! Allows the frontend to respond to approval requests.

use crate::services::AirlockService;
use std::sync::Arc;
use tauri::{command, State};
use tokio::sync::Mutex;

pub struct AirlockServiceState(pub Arc<Mutex<Option<AirlockService>>>);

/// Respond to an airlock approval request
#[command]
pub async fn respond_to_airlock(
    state: State<'_, AirlockServiceState>,
    command_id: String,
    approved: bool,
) -> Result<(), String> {
    let guard = state.0.lock().await;
    if let Some(airlock) = guard.as_ref() {
        airlock.respond_to_approval(&command_id, approved).await
    } else {
        Err("Airlock service not initialized".to_string())
    }
}

/// Get list of pending approval command IDs
#[command]
pub async fn get_pending_airlock_approvals(
    state: State<'_, AirlockServiceState>,
) -> Result<Vec<crate::services::airlock::ApprovalRequest>, String> {
    let guard = state.0.lock().await;
    if let Some(airlock) = guard.as_ref() {
        Ok(airlock.get_pending_approvals().await)
    } else {
        Err("Airlock service not initialized".to_string())
    }
}

#[command]
pub async fn list_airlock_messages(
    state: State<'_, AirlockServiceState>,
    limit: Option<u32>,
) -> Result<Vec<crate::services::AirlockMessage>, String> {
    let guard = state.0.lock().await;
    if let Some(airlock) = guard.as_ref() {
        airlock.list_messages(limit).await
    } else {
        Err("Airlock service not initialized".to_string())
    }
}

#[command]
pub async fn ack_airlock_message(
    state: State<'_, AirlockServiceState>,
    command_id: String,
) -> Result<(), String> {
    let guard = state.0.lock().await;
    if let Some(airlock) = guard.as_ref() {
        airlock.acknowledge_message(&command_id).await
    } else {
        Err("Airlock service not initialized".to_string())
    }
}

#[command]
pub async fn send_airlock_message(
    state: State<'_, AirlockServiceState>,
    title: String,
    body: String,
    command_id: Option<String>,
) -> Result<(), String> {
    let guard = state.0.lock().await;
    if let Some(airlock) = guard.as_ref() {
        airlock
            .send_message(&title, &body, command_id.as_deref())
            .await
    } else {
        Err("Airlock service not initialized".to_string())
    }
}

/// Set headless mode (auto-approve sensitive commands)
#[command]
pub async fn set_headless_mode(
    state: State<'_, AirlockServiceState>,
    enabled: bool,
) -> Result<(), String> {
    let guard = state.0.lock().await;
    if let Some(airlock) = guard.as_ref() {
        airlock.set_headless_mode(enabled);
        Ok(())
    } else {
        Err("Airlock service not initialized".to_string())
    }
}
