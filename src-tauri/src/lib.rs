// Rainy Cowork - Main Library
// Tauri 2 backend with AI workspace agent capabilities
// Uses rainy-sdk for premium AI features

mod ai;
mod commands;
mod models;
mod services;

use ai::AIProviderManager;
use services::{
    CoworkAgent, DocumentService, FileManager, FileOperationEngine, FolderManager, ImageService,
    SettingsManager, TaskManager, WebResearchService, WorkspaceManager,
};
use std::sync::Arc;
use tokio::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize AI provider manager as Arc for thread-safe access
    let ai_provider = Arc::new(AIProviderManager::new());

    // Initialize task manager with Arc clone (needs its own reference)
    let task_manager = TaskManager::new(ai_provider.clone());

    // Initialize file manager
    let file_manager = Arc::new(FileManager::new());

    // Initialize file operation engine (AI-driven operations)
    let file_ops = Arc::new(FileOperationEngine::new());

    // Initialize settings manager
    let settings_manager = Arc::new(Mutex::new(SettingsManager::new()));

    // Initialize cowork agent (natural language file operations)
    let cowork_agent = Arc::new(CoworkAgent::new(
        ai_provider.clone(),
        file_ops.clone(),
        file_manager.clone(),
        settings_manager.clone(),
    ));

    // Initialize web research service
    let web_research = WebResearchService::new();

    // Initialize document service
    let document_service = DocumentService::new();

    // Initialize image service
    let image_service = ImageService::new();

    // Initialize workspace manager
    let workspace_manager = Arc::new(WorkspaceManager::new().expect("Failed to create workspace manager"));

    // Initialize folder manager (requires app handle for data dir)
    // We'll initialize it in setup since we need the app handle

    tauri::Builder::default()
        // Plugins
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        // Managed state
        .manage(task_manager)
        .manage(file_manager)
        .manage(file_ops)
        .manage(cowork_agent)
        .manage(web_research)
        .manage(document_service)
        .manage(image_service)
        .manage(workspace_manager) // Arc<WorkspaceManager>
        .manage(ai_provider) // Arc<AIProviderManager>
        .manage(settings_manager) // Arc<Mutex<SettingsManager>>
        .setup(|app| {
            use tauri::Manager;

            // Initialize folder manager with app data dir
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to get app data dir");
            let folder_manager = FolderManager::new(app_data_dir);

            // Manage the folder manager state
            app.manage(folder_manager);

            // Initialize folder manager in background
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let fm = app_handle.state::<FolderManager>();
                if let Err(e) = fm.init().await {
                    tracing::error!("Failed to init folder manager: {}", e);
                }
            });

            // Initialize file operation engine in background
            let app_handle2 = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let ops = app_handle2.state::<Arc<FileOperationEngine>>();
                if let Err(e) = ops.init().await {
                    tracing::error!("Failed to init file operation engine: {}", e);
                }
            });

            Ok(())
        })
        // Commands
        .invoke_handler(tauri::generate_handler![
            // Task commands
            commands::create_task,
            commands::execute_task,
            commands::pause_task,
            commands::resume_task,
            commands::cancel_task,
            commands::get_task,
            commands::list_tasks,
            commands::set_task_manager_workspace,
            // AI commands
            commands::list_providers,
            commands::validate_api_key,
            commands::store_api_key,
            commands::get_api_key,
            commands::delete_api_key,
            commands::has_api_key,
            commands::get_provider_models,
            // Cowork status commands
            commands::get_cowork_status,
            commands::get_cowork_models,
            commands::can_use_feature,
            // File commands
            commands::select_workspace,
            commands::set_workspace,
            commands::get_workspace,
            commands::list_directory,
            commands::read_file,
            commands::write_file,
            commands::create_snapshot,
            commands::rollback_file,
            commands::list_file_changes,
            commands::fetch_web_content,
            commands::get_web_cache_stats,
            commands::clear_web_cache,
            // Document commands
            commands::list_document_templates,
            commands::get_templates_by_category,
            commands::get_template,
            commands::generate_document,
            commands::markdown_to_html,
            // Image commands
            commands::get_image_metadata,
            commands::generate_thumbnail,
            commands::get_image_dimensions,
            commands::is_image_supported,
            // Folder commands
            commands::add_user_folder,
            commands::list_user_folders,
            commands::remove_user_folder,
            commands::update_folder_access,
            // File Operations commands (NEW - AI Agent)
            commands::move_files,
            commands::organize_folder,
            commands::batch_rename,
            commands::safe_delete_files,
            commands::analyze_workspace,
            commands::undo_file_operation,
            commands::list_file_operations,
            // Versioning commands
            commands::create_file_version,
            commands::get_file_versions,
            commands::restore_file_version,
            // Transaction commands
            commands::begin_file_transaction,
            commands::commit_file_transaction,
            commands::rollback_file_transaction,
            commands::get_file_transaction,
            // Enhanced undo/redo commands
            commands::undo_file_operation_enhanced,
            commands::redo_file_operation,
            commands::list_enhanced_file_operations,
            commands::set_file_ops_workspace,
            // Agent commands (NEW - AI Agent)
            commands::plan_task,
            commands::execute_agent_task,
            commands::get_agent_plan,
            commands::cancel_agent_plan,
            commands::agent_analyze_workspace,
            // Settings commands
            commands::get_user_settings,
            commands::get_selected_model,
            commands::set_selected_model,
            commands::set_theme,
            commands::set_notifications,
            commands::get_available_models,
            // Workspace commands
            commands::create_workspace,
            commands::load_workspace,
            commands::save_workspace,
            commands::list_workspaces,
            commands::delete_workspace,
            commands::add_permission_override,
            commands::remove_permission_override,
            commands::get_permission_overrides,
            commands::get_effective_permissions,
            commands::get_workspace_templates,
            commands::create_workspace_from_template,
            commands::save_workspace_template,
            commands::delete_workspace_template,
            commands::get_workspace_analytics,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
