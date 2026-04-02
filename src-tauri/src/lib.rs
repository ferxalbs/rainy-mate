// Rainy MaTE - Main Library
// Tauri 2 backend with AI workspace agent capabilities
// Uses rainy-sdk for premium AI features

mod ai;
mod commands;
pub mod db;
mod models;
mod services;

use crate::ai::agent::chat_sessions;
use crate::ai::agent::manager::AgentManager;
use crate::ai::agent::runtime_registry::RuntimeRegistry;
use crate::db::Database;
use ai::{AIProviderManager, IntelligentRouter, ProviderRegistry};
use services::{
    ATMClient, AgentLibraryService, AgentRunControl, BrowserController, CommandPoller,
    DocumentService, FileManager, FileOperationEngine, FolderManager, ImageService,
    KeychainAccessService, LLMClient, ManagedResearchService, MemoryManager, NeuralService,
    NodeAuthenticator, QuickDelegateModalService, SettingsManager, SkillExecutor, SocketClient,
    WorkflowRecorderService, WorkspaceManager,
};
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::{Mutex, RwLock};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    fn startup_error(message: impl Into<String>) -> std::io::Error {
        std::io::Error::other(message.into())
    }

    // Initialize AI provider manager as Arc for thread-safe access
    let keychain_access = KeychainAccessService::new();
    let ai_provider = Arc::new(AIProviderManager::new(keychain_access.clone()));

    // Initialize provider registry for PHASE 3
    let provider_registry = Arc::new(ProviderRegistry::new());

    // Initialize task manager with Arc clone (needs its own reference)
    let task_manager = Arc::new(services::task_manager::TaskManager::new(
        ai_provider.clone(),
    ));

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
    let workspace_manager = match WorkspaceManager::new() {
        Ok(manager) => Arc::new(manager),
        Err(error) => {
            tracing::error!("Failed to create workspace manager: {}", error);
            return;
        }
    };

    // Initialize intelligent router for PHASE 3
    let intelligent_router = Arc::new(RwLock::new(IntelligentRouter::default()));

    // Initialize ATM Client (Rainy ATM)
    // TODO: Load API Key from secure storage if available
    let atm_client = ATMClient::new(
        "https://rainy-atm-cfe3gvcwua-uc.a.run.app".to_string(),
        None,
    )
    .with_keychain(keychain_access.clone());

    // Initialize Node Authenticator
    let authenticator = NodeAuthenticator::new();
    let runtime_registry = Arc::new(RuntimeRegistry::new());
    let agent_run_control = Arc::new(AgentRunControl::new());

    // Initialize Neural Service (Distributed Neural System)
    let neural_service = NeuralService::new(
        "https://rainy-atm-cfe3gvcwua-uc.a.run.app".to_string(),
        "pending-pairing".to_string(), // Initial state, will be updated after pairing
        authenticator,
        Some(runtime_registry.clone()),
        keychain_access.clone(),
    );

    // Initialize Browser Controller (Native CDP)
    let browser_controller = Arc::new(BrowserController::new());

    // Initialize MCP Service
    let mcp_service = Arc::new(crate::services::mcp_service::McpService::new());

    // Initialize MCP HTTP Proxy
    let mcp_http_proxy = Arc::new(crate::services::mcp_http::McpHttpProxy::new(
        mcp_service.clone(),
    ));

    // Initialize Skill Executor
    // Note: We removed the legacy web_research service from here
    let skill_executor = Arc::new(SkillExecutor::new(
        workspace_manager.clone(),
        Arc::new(managed_research.clone()),
        browser_controller.clone(),
        mcp_service.clone(),
    ));

    // Initialize Command Poller
    // Note: It starts "stopped". Setup will start it if credentials exist.
    let command_poller = Arc::new(CommandPoller::new(
        neural_service.clone(),
        Arc::new(atm_client.clone()),
        skill_executor.clone(),
        settings_manager.clone(),
    ));

    // Initialize Socket Client (Thunderbolt)
    let socket_client = SocketClient::new("wss://rainy-atm-cfe3gvcwua-uc.a.run.app/ws".to_string());

    // Initialize LLM Client (Brain)
    // API Key will be loaded/set via commands later
    let llm_client = Arc::new(Mutex::new(LLMClient::new("".to_string())));
    let workflow_recorder = Arc::new(WorkflowRecorderService::new());
    let remote_workspace_grants = Arc::new(services::RemoteWorkspaceGrantStore::new());
    let agent_library = match AgentLibraryService::new_default() {
        Ok(service) => Arc::new(service),
        Err(error) => {
            tracing::error!("Failed to initialize agent library service: {}", error);
            return;
        }
    };

    // Initialize folder manager (requires app handle for data dir)
    // We'll initialize it in setup since we need the app handle

    let tauri_app = tauri::Builder::default()
        // Plugins
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_process::init())
        // Managed state
        .manage(task_manager.clone())
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
        .manage(keychain_access.clone())
        .manage(commands::router::IntelligentRouterState(
            intelligent_router.clone(),
        )) // Arc<RwLock<IntelligentRouter>>
        .manage(atm_client) // ATMClient
        .manage(commands::neural::NeuralServiceState(neural_service)) // NeuralService
        .manage(browser_controller) // Arc<BrowserController>
        .manage(command_poller) // Arc<CommandPoller>
        .manage(skill_executor) // Arc<SkillExecutor>
        .manage(mcp_service.clone()) // Arc<McpService>
        .manage(mcp_http_proxy.clone()) // Arc<McpHttpProxy>
        .manage(runtime_registry.clone()) // Arc<RuntimeRegistry>
        .manage(agent_run_control.clone()) // Arc<AgentRunControl>
        .manage(socket_client) // SocketClient
        .manage(llm_client) // Arc<Mutex<LLMClient>>
        .manage(workflow_recorder) // Arc<WorkflowRecorderService>
        .manage(remote_workspace_grants.clone()) // Arc<RemoteWorkspaceGrantStore>
        .manage(agent_library) // Arc<AgentLibraryService>
        .manage(commands::airlock::AirlockServiceState(Arc::new(
            Mutex::new(None),
        ))) // Placeholder, initialized in setup
        .setup(move |app| {
            use crate::services::AirlockService;
            use tauri::Manager;

            // Initialize auto-updater plugin (desktop only)
            #[cfg(desktop)]
            app.handle()
                .plugin(tauri_plugin_updater::Builder::new().build())?;

            // IMPORTANT: Initialize libsql global C-state before any sqlx SQLite pool.
            // This prevents libsql/sqlx threading initialization conflicts that can
            // panic at runtime (libsql local database assertion + poisoned Once).
            tauri::async_runtime::block_on(async {
                let _ = libsql::Builder::new_local(":memory:").build().await;
            });

            // Initialize folder manager with app data dir
            let app_data_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| startup_error(format!("Failed to get app data dir: {}", e)))?;

            app.manage(Arc::new(QuickDelegateModalService::new(
                app.handle().clone(),
            )));

            tauri::async_runtime::block_on(async {
                mcp_service.set_app_handle(app.handle().clone()).await;
            });
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

            // Hydrate credentials in one serialized pass to avoid concurrent Security.framework access.
            let app_handle_keychain = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                use crate::ai::provider_trait::{AIProviderFactory, ProviderWithStats};
                use crate::ai::provider_types::{ProviderConfig, ProviderId, ProviderType};
                use crate::ai::providers::{GeminiProviderFactory, RainySDKProviderFactory};

                let keychain = app_handle_keychain.state::<KeychainAccessService>();
                let client = app_handle_keychain.state::<ATMClient>();
                let neural = app_handle_keychain.state::<commands::neural::NeuralServiceState>();
                let registry =
                    app_handle_keychain.state::<commands::ai_providers::ProviderRegistryState>();
                let router_state =
                    app_handle_keychain.state::<commands::router::IntelligentRouterState>();

                let snapshot = match keychain.load_startup_snapshot().await {
                    Ok(snapshot) => snapshot,
                    Err(err) => {
                        tracing::warn!("Failed to hydrate startup keychain snapshot: {}", err);
                        return;
                    }
                };

                if snapshot.atm_admin_key.is_some() {
                    if let Err(e) = client.load_credentials_from_keychain().await {
                        tracing::warn!("Failed to load ATM admin key: {}", e);
                    }
                }

                if snapshot.owner_auth_bundle_raw.is_some()
                    || snapshot.neural_platform_key.is_some()
                    || snapshot.neural_user_api_key.is_some()
                    || snapshot.neural_workspace_id.is_some()
                {
                    if let Err(e) = neural.0.load_credentials_from_keychain().await {
                        tracing::warn!("Failed to load Neural credentials: {}", e);
                    }
                }

                if let Some(api_key) = snapshot
                    .provider_keys
                    .get("rainy_api")
                    .cloned()
                    .unwrap_or(None)
                {
                    let provider_id = ProviderId::new("rainy_api");
                    if registry.0.get(&provider_id).is_err() {
                        let config = ProviderConfig {
                            id: provider_id.clone(),
                            provider_type: ProviderType::RainySDK,
                            api_key: Some(api_key),
                            base_url: None,
                            model: "gemini-3-flash-preview".to_string(),
                            params: std::collections::HashMap::new(),
                            enabled: true,
                            priority: 10,
                            rate_limit: None,
                            timeout: 120,
                        };
                        if let Ok(provider) =
                            <RainySDKProviderFactory as AIProviderFactory>::create(config).await
                        {
                            let _ = registry.0.register(provider.clone());
                            router_state
                                .0
                                .write()
                                .await
                                .add_provider(Arc::new(ProviderWithStats::new(provider)));
                        }
                    }
                }

                if let Some(api_key) = snapshot
                    .provider_keys
                    .get("gemini")
                    .cloned()
                    .unwrap_or(None)
                {
                    let provider_id = ProviderId::new("gemini_byok");
                    if registry.0.get(&provider_id).is_err() {
                        let config = ProviderConfig {
                            id: provider_id.clone(),
                            provider_type: ProviderType::Google,
                            api_key: Some(api_key),
                            base_url: None,
                            model: "gemini-3-flash-preview".to_string(),
                            params: std::collections::HashMap::new(),
                            enabled: true,
                            priority: 20,
                            rate_limit: None,
                            timeout: 120,
                        };
                        if let Ok(provider) =
                            <GeminiProviderFactory as AIProviderFactory>::create(config).await
                        {
                            let _ = registry.0.register(provider.clone());
                            router_state
                                .0
                                .write()
                                .await
                                .add_provider(Arc::new(ProviderWithStats::new(provider)));
                        }
                    }
                }
            });

            // Initialize memory manager with app data dir
            let app_data_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| startup_error(format!("Failed to get app data dir: {}", e)))?;
            let memory_db_path = app_data_dir.join("memory_db");
            let memory_manager = Arc::new(MemoryManager::new(100, memory_db_path));

            // Initialize Crystalline Memory (Watcher)
            let mm_clone = memory_manager.clone();
            tauri::async_runtime::spawn(async move {
                mm_clone.init().await;
            });

            // Manage memory manager state
            app.manage(commands::memory::MemoryManagerState(memory_manager.clone()));

            // Inject MemoryManager into SkillExecutor (Late Binding)
            {
                let se = app.state::<Arc<SkillExecutor>>();
                let mm = memory_manager.clone();
                tauri::async_runtime::block_on(async move {
                    se.set_memory_manager(mm).await;
                });
            }

            // Initialize Database and AgentManager
            // We block here to ensure DB is ready for core services like CommandPoller
            let db = tauri::async_runtime::block_on(async { Database::init(app.handle()).await })
                .map_err(|e| startup_error(format!("Failed to initialize database: {}", e)))?;

            let airlock_message_store =
                Arc::new(crate::services::AirlockMessageStore::new(db.pool.clone()));
            tauri::async_runtime::block_on(async {
                airlock_message_store.init().await.map_err(|e| {
                    startup_error(format!("Failed to initialize Airlock message store: {}", e))
                })
            })?;

            // Initialize Airlock Service with app handle + persistence
            let airlock =
                AirlockService::new(app.handle().clone(), Some(airlock_message_store.clone()));
            let airlock_for_poller = airlock.clone();

            let airlock_state = app.state::<commands::airlock::AirlockServiceState>();
            {
                let mut guard = tauri::async_runtime::block_on(airlock_state.0.lock());
                *guard = Some(airlock);
            }

            let agent_manager = AgentManager::new(db.pool.clone());
            app.manage(agent_manager.clone());

            // Initialize Persistent Scheduler
            let persistent_scheduler = std::sync::Arc::new(
                crate::services::persistent_scheduler::PersistentScheduler::new(
                    db.pool.clone(),
                    (*app.state::<std::sync::Arc<crate::services::task_manager::TaskManager>>())
                        .clone(),
                ),
            );
            tauri::async_runtime::block_on(async {
                persistent_scheduler
                    .set_app_handle(app.handle().clone())
                    .await;
                persistent_scheduler
                    .init()
                    .await
                    .map_err(|e| startup_error(format!("Failed to init scheduler: {}", e)))?;
                {
                    let se = app.state::<Arc<SkillExecutor>>();
                    se.set_scheduler(persistent_scheduler.clone()).await;
                }
                persistent_scheduler.start_loop();
                Ok::<(), std::io::Error>(())
            })?;
            app.manage(persistent_scheduler);

            tracing::info!("Database and AgentManager initialized successfully");

            // Start Command Poller
            // Check if we have credentials, if so start polling
            let poller = (*app.state::<Arc<CommandPoller>>()).clone();
            let router_for_poller = intelligent_router.clone();
            let app_data_for_poller = app.path().app_data_dir().map_err(|e| {
                startup_error(format!(
                    "Failed to get app data dir for CommandPoller: {}",
                    e
                ))
            })?;

            let agent_manager_for_poller = Arc::new(agent_manager);
            let runtime_registry_for_poller = runtime_registry.clone();
            let memory_manager_for_poller = memory_manager.clone();

            // Create SessionCoordinator (unifies local + remote session lifecycle)
            let session_coordinator = Arc::new(
                crate::services::session_coordinator::SessionCoordinator::new(
                    agent_manager_for_poller.clone(),
                    app.handle().clone(),
                ),
            );
            app.manage(session_coordinator.clone());

            let session_coordinator_for_poller = session_coordinator.clone();
            let app_handle_for_poller = app.handle().clone();

            // Readiness signal: replaces fixed sleep timers in spawned tasks.
            // Notified once all core services are registered below.
            let services_ready = Arc::new(tokio::sync::Notify::new());
            let services_ready_for_ws = services_ready.clone();

            // Signal readiness immediately — all blocking init is done above this point
            services_ready.notify_waiters();

            tauri::async_runtime::spawn(async move {
                // Inject Airlock service
                poller.set_airlock_service(airlock_for_poller).await;

                // Inject agent context for agent.run commands
                poller
                    .set_agent_context(
                        app_handle_for_poller,
                        router_for_poller,
                        app_data_for_poller,
                        agent_manager_for_poller,
                        runtime_registry_for_poller,
                        memory_manager_for_poller,
                        session_coordinator_for_poller,
                        remote_workspace_grants.clone(),
                    )
                    .await;

                // If credentials exist (handled internally by poll_and_execute check), start loop
                poller.start().await;
            });

            // Initialize Cloud Bridge (WebSocket)
            let app_handle_cb = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                // Wait for core services to be ready before connecting WebSocket
                services_ready_for_ws.notified().await;

                // Start Socket Client (Thunderbolt)
                let socket = app_handle_cb.state::<SocketClient>().inner().clone();
                let poller_for_ws = app_handle_cb.state::<Arc<CommandPoller>>().inner().clone();

                // Subscribe to real-time triggers
                let mut rx = socket.subscribe();
                tauri::async_runtime::spawn(async move {
                    while let Ok(msg) = rx.recv().await {
                        if msg.event == "command_queued" || msg.event == "new_command" {
                            tracing::info!("Real-time trigger received: {}", msg.event);
                            let action = msg
                                .payload
                                .get("action")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default();
                            if action == "fleet_kill_switch" {
                                poller_for_ws
                                    .arm_kill_switch("websocket:fleet_kill_switch")
                                    .await;
                            }
                            poller_for_ws.trigger();
                        }
                    }
                });

                socket.connect().await;

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
            commands::index_knowledge_file,
            commands::query_agent_memory,
            // Memory Vault Explorer commands
            commands::list_vault_entries,
            commands::list_memory_workspaces,
            commands::get_vault_detailed_stats,
            commands::delete_vault_entries_batch,
            commands::clear_workspace_vault,
            // Settings commands
            commands::get_user_settings,
            commands::get_selected_model,
            commands::set_selected_model,
            commands::get_embedder_provider,
            commands::set_embedder_provider,
            commands::get_embedder_model,
            commands::set_embedder_model,
            commands::set_theme,
            commands::set_notifications,
            commands::get_notification_status,
            commands::request_notification_permission,
            commands::send_test_notification,
            commands::focus_airlock_request,
            commands::get_system_readiness,
            commands::open_quick_delegate_modal,
            commands::get_quick_delegate_status,
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
            commands::get_effective_local_agent_policy,
            commands::get_workspace_templates,
            commands::create_workspace_from_template,
            commands::save_workspace_template,
            commands::delete_workspace_template,
            commands::get_workspace_analytics,
            commands::list_mate_pack_definitions,
            commands::list_first_run_scenarios,
            commands::get_workspace_launchpad,
            commands::update_workspace_launch_config,
            commands::build_workspace_first_run_prompt,
            commands::prepare_workspace_launch,
            commands::record_workspace_launch_result,
            commands::create_workspace_scheduled_run,
            commands::create_workspace_prompt_scheduled_run,
            commands::list_workspace_scheduled_runs,
            commands::delete_workspace_scheduled_run,
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
            commands::ensure_default_atm_agent,
            commands::create_atm_agent,
            commands::list_atm_agents,
            commands::list_atm_workspace_shared_agents,
            commands::import_atm_workspace_shared_agent,
            commands::list_atm_marketplace_agents,
            commands::publish_atm_marketplace_agent,
            commands::import_atm_marketplace_agent,
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
            commands::get_atm_tool_access_policy,
            commands::update_atm_tool_access_policy,
            commands::get_atm_fleet_status,
            commands::push_atm_fleet_policy,
            commands::trigger_atm_fleet_kill_switch,
            commands::retire_atm_fleet_node,
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
            commands::resume_neural_runtime,
            commands::classify_neural_error,
            commands::get_neural_workspace_id,
            // Airlock Commands (Security)
            commands::respond_to_airlock,
            commands::get_pending_airlock_approvals,
            commands::list_airlock_messages,
            commands::ack_airlock_message,
            commands::send_airlock_message,
            commands::set_headless_mode,
            // Skill Commands (Direct Local Execution)
            commands::execute_skill,
            commands::parse_tool_calls,
            commands::execute_plan_from_content,
            commands::list_installed_skills,
            commands::list_prompt_skills,
            commands::install_local_skill,
            commands::install_skill_from_atm,
            commands::set_installed_skill_enabled,
            commands::set_prompt_skill_all_agents_enabled,
            commands::refresh_prompt_skill_snapshot,
            commands::remove_installed_skill,
            // Agent Workflow (Native Rust)
            commands::agent::run_agent_workflow,
            commands::agent::cancel_agent_run,
            commands::agent::get_chat_session,
            commands::agent::list_chat_sessions,
            commands::agent::create_chat_session,
            commands::agent::create_or_reuse_empty_chat_session,
            commands::agent::ensure_default_local_agent,
            commands::agent::delete_chat_session,
            commands::agent::update_chat_title,
            commands::agent::ensure_chat_title,
            commands::agent::list_active_sessions,
            commands::agent::prepare_attachment_previews,
            commands::chat_artifacts::open_chat_artifact,
            // Workflow Factory (THE FORGE foundation)
            commands::start_workflow_recording,
            commands::record_workflow_step,
            commands::stop_workflow_recording,
            commands::get_workflow_recording,
            commands::get_active_workflow_recording,
            commands::generate_agent_spec_from_recording,
            commands::validate_generated_agent,
            commands::save_generated_agent,
            commands::list_generated_agents,
            commands::load_generated_agent,
            // Deployment (Phase 1)
            commands::deploy_agent,
            commands::save_agent_spec,
            commands::load_agent_spec,
            commands::list_agent_specs,
            commands::deploy_agent_spec,
            // Agent Persistence (Phase 3)
            chat_sessions::save_agent_to_db,
            chat_sessions::load_agents_from_db,
            chat_sessions::save_chat_message,
            chat_sessions::get_chat_history,
            chat_sessions::get_chat_history_window,
            chat_sessions::get_default_chat_scope,
            chat_sessions::get_or_create_workspace_chat,
            chat_sessions::get_chat_compaction_state,
            chat_sessions::get_chat_runtime_telemetry,
            chat_sessions::clear_chat_history,
            chat_sessions::compact_session_cmd,
            crate::services::mcp_http::handle_mcp_request,
            commands::list_mcp_servers,
            commands::upsert_mcp_server,
            commands::remove_mcp_server,
            commands::connect_mcp_saved_server,
            commands::connect_mcp_server,
            commands::disconnect_mcp_server,
            commands::refresh_mcp_server_tools,
            commands::list_mcp_runtime_servers,
            commands::get_mcp_runtime_status,
            commands::get_mcp_permission_mode,
            commands::set_mcp_permission_mode,
            commands::get_pending_mcp_approvals,
            commands::respond_to_mcp_approval,
            commands::import_mcp_servers_from_json,
            commands::get_or_create_default_mcp_json_config,
            commands::save_default_mcp_json_config,
            commands::import_mcp_servers_from_default_json,
            crate::services::persistent_scheduler::add_scheduled_job,
            crate::services::persistent_scheduler::list_scheduled_jobs,
            crate::services::persistent_scheduler::remove_scheduled_job,
        ])
        .build(tauri::generate_context!());

    let tauri_app = match tauri_app {
        Ok(app) => app,
        Err(error) => {
            tracing::error!("error while building tauri application: {}", error);
            return;
        }
    };

    tauri_app.run(|app_handle, event| {
        #[cfg(target_os = "macos")]
        if let tauri::RunEvent::Ready = event {
            let airlock_state = app_handle.state::<commands::airlock::AirlockServiceState>();
            crate::services::MacOSNativeNotificationBridge::initialize(
                app_handle.clone(),
                commands::airlock::AirlockServiceState(airlock_state.0.clone()),
            );
            let quick_delegate = app_handle.state::<Arc<QuickDelegateModalService>>();
            crate::services::MacOSQuickDelegateBridge::initialize(
                app_handle.clone(),
                quick_delegate.inner().clone(),
            );
        }
    });
}
