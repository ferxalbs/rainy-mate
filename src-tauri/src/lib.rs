// Rainy Cowork - Main Library
// Tauri 2 backend with AI workspace agent capabilities
// Uses rainy-sdk for premium AI features

mod agents;
mod ai;
mod commands;
pub mod db;
mod models;
mod services;

use crate::ai::agent::manager::{self, AgentManager};
use crate::db::Database;
use agents::AgentRegistry;
use ai::{AIProviderManager, IntelligentRouter, ProviderRegistry};
use services::{
    ATMClient, BrowserController, CommandPoller, DocumentService, FileManager, FileOperationEngine,
    FolderManager, ImageService, ManagedResearchService, MemoryManager, NeuralService,
    NodeAuthenticator, ReflectionEngine, SettingsManager, SkillExecutor, WorkspaceManager,
};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize AI provider manager as Arc for thread-safe access
    let ai_provider = Arc::new(AIProviderManager::new());

    // Initialize provider registry for PHASE 3
    let provider_registry = Arc::new(ProviderRegistry::new());

    // Initialize task manager with Arc clone (needs its own reference)
    let task_manager = services::task_manager::TaskManager::new(ai_provider.clone());

    // Initialize file manager
    let file_manager = Arc::new(FileManager::new());

    // Initialize file operation engine (AI-driven operations)
    let file_ops = Arc::new(FileOperationEngine::new());

    // Initialize settings manager
    let settings_manager = Arc::new(Mutex::new(SettingsManager::new()));

    // Initialize web research service (Phase 3 AI features)
    let managed_research = ManagedResearchService::new(ai_provider.clone());

    // Initialize document service
    let document_service = DocumentService::new();

    // Initialize image service
    let image_service = ImageService::new();

    // Initialize workspace manager
    let workspace_manager =
        Arc::new(WorkspaceManager::new().expect("Failed to create workspace manager"));

    // Initialize agent registry for multi-agent system
    let agent_registry = Arc::new(AgentRegistry::new(ai_provider.clone()));

    // Initialize reflection engine for self-improvement
    let reflection_engine = Arc::new(ReflectionEngine::new(ai_provider.clone()));

    // Initialize intelligent router for PHASE 3
    let intelligent_router = Arc::new(RwLock::new(IntelligentRouter::default()));

    // Initialize ATM Client (Rainy ATM)
    // TODO: Load API Key from secure storage if available
    let atm_client = ATMClient::new(
        "https://rainy-atm-cfe3gvcwua-uc.a.run.app".to_string(),
        None,
    );

    // Initialize Node Authenticator
    let authenticator = NodeAuthenticator::new();

    // Initialize Neural Service (Distributed Neural System)
    let neural_service = NeuralService::new(
        "https://rainy-atm-cfe3gvcwua-uc.a.run.app".to_string(),
        "pending-pairing".to_string(), // Initial state, will be updated after pairing
        authenticator,
    );

    // Initialize Browser Controller (Native CDP)
    let browser_controller = Arc::new(BrowserController::new());

    // Initialize Skill Executor
    // Note: We removed the legacy web_research service from here
    let skill_executor = Arc::new(SkillExecutor::new(
        workspace_manager.clone(),
        Arc::new(managed_research.clone()),
        browser_controller.clone(),
    ));

    // Initialize Command Poller
    // Note: It starts "stopped". Setup will start it if credentials exist.
    let command_poller = Arc::new(CommandPoller::new(
        neural_service.clone(),
        skill_executor.clone(),
    ));

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
        .manage(managed_research) // Manage the new AI research service
        .manage(document_service)
        .manage(image_service)
        .manage(workspace_manager) // Arc<WorkspaceManager>
        .manage(ai_provider) // Arc<AIProviderManager>
        .manage(commands::ai_providers::ProviderRegistryState(
            provider_registry,
        )) // Arc<ProviderRegistry>
        .manage(settings_manager) // Arc<Mutex<SettingsManager>>
        .manage(commands::agents::AgentRegistryState(agent_registry)) // Arc<AgentRegistry>
        .manage(commands::reflection::ReflectionEngineState(
            reflection_engine,
        )) // Arc<ReflectionEngine>
        .manage(commands::router::IntelligentRouterState(
            intelligent_router.clone(),
        )) // Arc<RwLock<IntelligentRouter>>
        .manage(atm_client) // ATMClient
        .manage(commands::neural::NeuralServiceState(neural_service)) // NeuralService
        .manage(browser_controller) // Arc<BrowserController>
        .manage(command_poller) // Arc<CommandPoller>
        .manage(skill_executor) // Arc<SkillExecutor>
        .manage(commands::airlock::AirlockServiceState(Arc::new(
            Mutex::new(None),
        ))) // Placeholder, initialized in setup
        .setup(move |app| {
            use crate::services::AirlockService;
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

            // Load ATM admin key from keychain (best-effort) for session continuity
            let app_handle3 = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let client = app_handle3.state::<ATMClient>();
                if let Err(e) = client.load_credentials_from_keychain().await {
                    tracing::warn!("Failed to load ATM admin key: {}", e);
                }
            });

            // Load Neural credentials/workspace from keychain for cloud<->desktop auto reconnect.
            let app_handle_neural = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let neural = app_handle_neural.state::<commands::neural::NeuralServiceState>();
                if let Err(e) = neural.0.load_credentials_from_keychain().await {
                    tracing::warn!("Failed to load Neural credentials: {}", e);
                }
            });

            // Initialize memory manager with app data dir
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to get app data dir");
            let memory_db_path = app_data_dir.join("memory_db");
            let memory_manager = Arc::new(MemoryManager::new(100, memory_db_path));

            // Manage memory manager state
            app.manage(commands::memory::MemoryManagerState(memory_manager));

            // Initialize Airlock Service with app handle
            let airlock = AirlockService::new(app.handle().clone());
            let airlock_for_poller = airlock.clone();

            let airlock_state = app.state::<commands::airlock::AirlockServiceState>();
            {
                let mut guard = tauri::async_runtime::block_on(airlock_state.0.lock());
                *guard = Some(airlock);
            }

            // Initialize Database and AgentManager
            // We block here to ensure DB is ready for core services like CommandPoller
            let db = tauri::async_runtime::block_on(async { Database::init(app.handle()).await })
                .expect("Failed to initialize database");

            let agent_manager = AgentManager::new(db.pool);
            app.manage(agent_manager.clone());
            tracing::info!("Database and AgentManager initialized successfully");

            // Start Command Poller
            // Check if we have credentials, if so start polling
            let poller = (*app.state::<Arc<CommandPoller>>()).clone();
            let router_for_poller = intelligent_router.clone();
            let app_data_for_poller = app
                .path()
                .app_data_dir()
                .expect("Failed to get app data dir for CommandPoller");

            let agent_manager_for_poller = Arc::new(agent_manager);

            tauri::async_runtime::spawn(async move {
                // Inject Airlock service
                poller.set_airlock_service(airlock_for_poller).await;

                // Inject agent context for agent.run commands
                poller
                    .set_agent_context(
                        router_for_poller,
                        app_data_for_poller,
                        agent_manager_for_poller,
                    )
                    .await;

                // Wait a bit for app to stabilize
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                // If credentials exist (handled internally by poll_and_execute check), start loop
                poller.start().await;
            });

            // Initialize Cloud Bridge (WebSocket)
            let app_handle_cb = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                // Wait for other services
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                let atm = app_handle_cb
                    .state::<crate::services::ATMClient>()
                    .inner()
                    .clone();
                let bridge = crate::services::cloud_bridge::CloudBridge::new(
                    std::sync::Arc::new(atm),
                    app_handle_cb.clone(),
                );
                bridge.start();
                app_handle_cb.manage(bridge);
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
            commands::save_task_queue_state,
            commands::load_task_queue_state,
            commands::start_background_task_processing,
            // AI commands
            commands::list_providers,
            commands::validate_api_key,
            commands::store_api_key,
            commands::get_api_key,
            commands::delete_api_key,
            commands::has_api_key,
            commands::get_provider_models,
            // AI Provider commands (PHASE 3)
            commands::list_all_providers,
            commands::get_provider_info,
            commands::register_provider,
            commands::unregister_provider,
            commands::set_default_provider,
            commands::get_default_provider,
            commands::get_provider_stats,
            commands::get_all_provider_stats,
            commands::test_provider_connection,
            commands::get_provider_capabilities,
            commands::complete_chat,
            commands::generate_embeddings,
            commands::get_provider_available_models,
            commands::clear_providers,
            commands::get_provider_count,
            // File commands
            commands::select_workspace,
            commands::set_workspace,
            commands::get_workspace,
            commands::list_directory,
            commands::read_file,
            commands::write_file,
            commands::append_file,
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
            // Multi-agent system commands (NEW - Agent Registry)
            commands::register_agent,
            commands::unregister_agent,
            commands::list_agents,
            commands::get_agent_info,
            commands::get_agent_status,
            commands::create_multi_agent_task,
            commands::execute_multi_agent_task,
            commands::cancel_agent_task,
            commands::get_task_status,
            commands::send_agent_message,
            commands::get_agent_messages,
            commands::get_agent_statistics,
            commands::get_agent_capabilities,
            // Memory commands (NEW - Memory System)
            commands::store_memory,
            commands::search_memory,
            commands::get_recent_memory,
            commands::get_all_short_term_memory,
            commands::clear_short_term_memory,
            commands::get_memory_stats,
            commands::get_memory_by_id,
            commands::delete_memory,
            commands::get_short_term_memory_size,
            commands::is_short_term_memory_empty,
            // Settings commands
            commands::get_user_settings,
            commands::get_selected_model,
            commands::set_selected_model,
            commands::set_theme,
            commands::set_notifications,
            commands::get_user_profile,
            commands::set_user_profile,
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
            // Reflection & Governance commands (NEW - Reflection System)
            commands::analyze_task_result,
            commands::get_error_patterns,
            commands::get_strategies,
            commands::optimize_system,
            commands::clear_error_patterns,
            commands::clear_strategies,
            commands::add_security_policy,
            commands::list_security_policies,
            commands::remove_security_policy,
            commands::evaluate_task_quality,
            // Router commands (PHASE 3 - Intelligent Routing)
            commands::get_router_config,
            commands::update_router_config,
            commands::get_router_stats,
            commands::complete_with_routing,
            commands::stream_with_routing,
            commands::embed_with_routing,
            commands::add_provider_to_router,
            commands::remove_provider_from_router,
            commands::get_router_providers,
            commands::router_has_providers,
            // Research commands
            commands::research::perform_research,
            // Unified Model commands (PHASE 4)
            commands::get_unified_models,
            commands::toggle_model,
            commands::set_default_fast_model,
            commands::set_default_deep_model,
            commands::get_user_preferences,
            commands::send_unified_message,
            commands::get_recommended_model,
            commands::unified_chat_stream,
            // ATM Commands (Rainy ATM)
            commands::bootstrap_atm,
            commands::create_atm_agent,
            commands::list_atm_agents,
            commands::list_atm_commands,
            commands::get_atm_command_details,
            commands::get_atm_command_progress,
            commands::get_atm_command_metrics,
            commands::get_atm_workspace_command_metrics,
            commands::get_atm_endpoint_metrics,
            commands::sync_atm_metrics_alerts,
            commands::list_atm_metrics_alerts,
            commands::ack_atm_metrics_alert,
            commands::get_atm_metrics_slo,
            commands::update_atm_metrics_slo,
            commands::get_atm_metrics_alert_retention,
            commands::update_atm_metrics_alert_retention,
            commands::cleanup_atm_metrics_alerts,
            commands::get_atm_admin_permissions,
            commands::update_atm_admin_permissions,
            commands::list_atm_admin_policy_audit,
            commands::set_atm_credentials,
            commands::has_atm_credentials,
            commands::ensure_atm_credentials_loaded,
            commands::generate_pairing_code,
            commands::reset_neural_workspace,
            // Neural System Commands (Desktop Nerve Center)
            commands::set_neural_workspace_id,
            commands::register_node,
            commands::send_heartbeat,
            commands::poll_commands,
            commands::start_command_execution,
            commands::complete_command_execution,
            commands::set_neural_credentials,
            commands::load_neural_credentials,
            commands::has_neural_credentials,
            commands::get_neural_credentials_values,
            commands::clear_neural_credentials,
            // Airlock Commands (Security)
            commands::respond_to_airlock,
            commands::get_pending_airlock_approvals,
            commands::set_headless_mode,
            // Skill Commands (Direct Local Execution)
            commands::execute_skill,
            // Agent Workflow (Native Rust)
            commands::agent::run_agent_workflow,
            // Deployment (Phase 1)
            commands::deploy_agent,
            commands::save_agent_spec,
            commands::load_agent_spec,
            commands::list_agent_specs,
            commands::deploy_agent_spec,
            // Agent Persistence (Phase 3)
            manager::save_agent_to_db,
            manager::load_agents_from_db,
            manager::save_chat_message,
            manager::get_chat_history,
            manager::clear_chat_history,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
