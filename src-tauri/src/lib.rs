// Rainy Cowork - Main Library
// Tauri 2 backend with AI workspace agent capabilities
// Uses rainy-sdk for premium AI features

mod ai;
mod commands;
mod models;
mod services;

use ai::AIProviderManager;
use services::{
    DocumentService, FileManager, FolderManager, ImageService, TaskManager, WebResearchService,
};
use std::sync::Arc;
use tokio::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize AI provider manager as Arc<Mutex> for mutable access
    let ai_provider = Arc::new(Mutex::new(AIProviderManager::new()));

    // Initialize task manager with Arc clone (needs its own reference)
    let task_manager = TaskManager::new(ai_provider.clone());

    // Initialize file manager
    let file_manager = FileManager::new();

    // Initialize web research service
    let web_research = WebResearchService::new();

    // Initialize document service
    let document_service = DocumentService::new();

    // Initialize image service
    let image_service = ImageService::new();

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
        .manage(web_research)
        .manage(document_service)
        .manage(image_service)
        .manage(ai_provider) // Arc<Mutex<AIProviderManager>>
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
