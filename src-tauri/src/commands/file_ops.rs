// Rainy Cowork - File Operations Commands
// Tauri commands for advanced file operations and AI agent
// Part of Phase 2: Enhanced Tauri Commands

use crate::services::file_operations::{
    ConflictStrategy, FileOpChange, FileOperationEngine, FileVersion, FileVersionInfo,
    MoveOperation, OrganizeResult, OrganizeStrategy, RenamePattern, RenamePreview, Transaction,
    WorkspaceAnalysis,
};
use std::sync::Arc;
use tauri::State;

// ============ File Operation Commands ============

/// Move multiple files to a destination
#[tauri::command]
pub async fn move_files(
    paths: Vec<String>,
    destination: String,
    on_conflict: Option<String>,
    state: State<'_, Arc<FileOperationEngine>>,
) -> Result<Vec<FileOpChange>, String> {
    let conflict_strategy = match on_conflict.as_deref() {
        Some("overwrite") => ConflictStrategy::Overwrite,
        Some("rename") => ConflictStrategy::Rename,
        Some("ask") => ConflictStrategy::Ask,
        _ => ConflictStrategy::Skip,
    };

    let operations: Vec<MoveOperation> = paths
        .into_iter()
        .map(|source| {
            let file_name = std::path::Path::new(&source)
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            MoveOperation {
                source,
                destination: format!("{}/{}", destination, file_name),
                on_conflict: conflict_strategy,
            }
        })
        .collect();

    state
        .move_files(operations)
        .await
        .map_err(|e| e.to_string())
}

/// Organize folder contents by strategy
#[tauri::command]
pub async fn organize_folder(
    path: String,
    strategy: String,
    dry_run: Option<bool>,
    state: State<'_, Arc<FileOperationEngine>>,
) -> Result<OrganizeResult, String> {
    let organize_strategy = match strategy.as_str() {
        "by_date" => OrganizeStrategy::ByDate,
        "by_extension" => OrganizeStrategy::ByExtension,
        "by_content" => OrganizeStrategy::ByContent,
        _ => OrganizeStrategy::ByType,
    };

    state
        .organize_folder(&path, organize_strategy, dry_run.unwrap_or(false))
        .await
        .map_err(|e| e.to_string())
}

/// Batch rename files with pattern
#[tauri::command]
pub async fn batch_rename(
    files: Vec<String>,
    pattern: String,
    find: Option<String>,
    replace: Option<String>,
    counter_start: Option<u32>,
    preview_only: Option<bool>,
    state: State<'_, Arc<FileOperationEngine>>,
) -> Result<Vec<RenamePreview>, String> {
    let rename_pattern = RenamePattern {
        template: pattern,
        find,
        replace,
        counter_start,
        counter_padding: Some(3),
    };

    state
        .batch_rename(files, rename_pattern, preview_only.unwrap_or(true))
        .await
        .map_err(|e| e.to_string())
}

/// Safely delete files (move to trash)
#[tauri::command]
pub async fn safe_delete_files(
    paths: Vec<String>,
    state: State<'_, Arc<FileOperationEngine>>,
) -> Result<Vec<FileOpChange>, String> {
    state.safe_delete(paths).await.map_err(|e| e.to_string())
}

/// Analyze workspace for optimization suggestions
#[tauri::command]
pub async fn analyze_workspace(
    path: String,
    state: State<'_, Arc<FileOperationEngine>>,
) -> Result<WorkspaceAnalysis, String> {
    state
        .analyze_workspace(&path)
        .await
        .map_err(|e| e.to_string())
}

/// Undo a previous file operation
#[tauri::command]
pub async fn undo_file_operation(
    operation_id: String,
    state: State<'_, Arc<FileOperationEngine>>,
) -> Result<Vec<FileOpChange>, String> {
    state
        .undo_operation(&operation_id)
        .await
        .map_err(|e| e.to_string())
}

/// List undoable operations
#[tauri::command]
pub async fn list_file_operations(
    state: State<'_, Arc<FileOperationEngine>>,
) -> Result<Vec<(String, String, String)>, String> {
    let ops = state.list_operations();
    Ok(ops
        .into_iter()
        .map(|(id, desc, ts)| (id, desc, ts.to_rfc3339()))
        .collect())
}

// ============ Versioning Commands ============

/// Create a version snapshot of a file
#[tauri::command]
pub async fn create_file_version(
    file_path: String,
    description: String,
    state: State<'_, Arc<FileOperationEngine>>,
) -> Result<FileVersion, String> {
    state
        .create_version_snapshot(&file_path, &description)
        .await
        .map_err(|e| e.to_string())
}

/// Get version information for a file
#[tauri::command]
pub async fn get_file_versions(
    file_path: String,
    state: State<'_, Arc<FileOperationEngine>>,
) -> Result<FileVersionInfo, String> {
    state
        .get_file_version_info(&file_path)
        .await
        .map_err(|e| e.to_string())
}

/// Restore a file from a specific version
#[tauri::command]
pub async fn restore_file_version(
    file_path: String,
    version_id: String,
    state: State<'_, Arc<FileOperationEngine>>,
) -> Result<FileOpChange, String> {
    state
        .restore_version(&file_path, &version_id)
        .await
        .map_err(|e| e.to_string())
}

// ============ Transaction Commands ============

/// Start a new transaction
#[tauri::command]
pub async fn begin_file_transaction(
    description: String,
    state: State<'_, Arc<FileOperationEngine>>,
) -> Result<String, String> {
    state
        .begin_transaction(&description)
        .await
        .map_err(|e| e.to_string())
}

/// Commit a transaction
#[tauri::command]
pub async fn commit_file_transaction(
    transaction_id: String,
    state: State<'_, Arc<FileOperationEngine>>,
) -> Result<Vec<FileOpChange>, String> {
    state
        .commit_transaction(&transaction_id)
        .await
        .map_err(|e| e.to_string())
}

/// Rollback a transaction
#[tauri::command]
pub async fn rollback_file_transaction(
    transaction_id: String,
    state: State<'_, Arc<FileOperationEngine>>,
) -> Result<Vec<FileOpChange>, String> {
    state
        .rollback_transaction(&transaction_id)
        .await
        .map_err(|e| e.to_string())
}

/// Get transaction status
#[tauri::command]
pub async fn get_file_transaction(
    transaction_id: String,
    state: State<'_, Arc<FileOperationEngine>>,
) -> Result<Option<Transaction>, String> {
    Ok(state.get_transaction(&transaction_id))
}

// ============ Enhanced Undo/Redo Commands ============

/// Enhanced undo operation
#[tauri::command]
pub async fn undo_file_operation_enhanced(
    operation_id: String,
    state: State<'_, Arc<FileOperationEngine>>,
) -> Result<Vec<FileOpChange>, String> {
    state
        .undo_enhanced(&operation_id)
        .await
        .map_err(|e| e.to_string())
}

/// Redo a previously undone operation
#[tauri::command]
pub async fn redo_file_operation(
    state: State<'_, Arc<FileOperationEngine>>,
) -> Result<Vec<FileOpChange>, String> {
    state.redo_operation().await.map_err(|e| e.to_string())
}

/// List enhanced operations
#[tauri::command]
pub async fn list_enhanced_file_operations(
    state: State<'_, Arc<FileOperationEngine>>,
) -> Result<Vec<(String, String, String, Option<String>)>, String> {
    let ops = state.list_enhanced_operations();
    Ok(ops
        .into_iter()
        .map(|(id, desc, ts, tx_id)| (id, desc, ts.to_rfc3339(), tx_id))
        .collect())
}

/// Set workspace context for file operations
#[tauri::command]
pub async fn set_file_ops_workspace(
    workspace_id: String,
    workspace_manager: State<'_, Arc<crate::services::WorkspaceManager>>,
    file_ops: State<'_, Arc<FileOperationEngine>>,
) -> Result<(), String> {
    let workspace = workspace_manager
        .load_workspace(&workspace_id)
        .map_err(|e| format!("Failed to load workspace: {}", e))?;

    file_ops.set_workspace(workspace).await;
    tracing::info!(
        "Workspace context set for file operations: {}",
        workspace_id
    );
    Ok(())
}
