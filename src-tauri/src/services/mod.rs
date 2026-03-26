// Rainy MaTE - Services Module
// Business logic layer

pub mod agent_kill_switch;
pub mod agent_library;
pub mod agent_run_control;
pub mod airlock;
pub mod airlock_messages;
pub mod app_identity;
pub mod atm_auth;
pub mod atm_client;
pub mod audit_emitter;
pub mod browser_controller;
pub mod cloud_bridge;
pub mod command_poller;
pub mod command_poller_agent;
pub mod default_agent_spec;
pub mod document;
pub mod embedder;
pub mod file_manager;
pub mod file_operations;
pub mod fleet_control;
pub mod folder_manager;
pub mod image;
pub mod llm_client;
pub mod local_agent_security;
pub mod macos_native_notifications;
pub mod managed_research; // Phase 3 AI Research
pub mod manifest_signing;
pub mod mcp_http;
pub mod mcp_service;
pub mod memory;
pub mod memory_vault;
pub mod neural_service;
pub mod persistent_scheduler;
pub mod prompt_skills;
pub mod security;
pub mod session_coordinator;
pub mod settings;
pub mod skill_executor;
pub mod skill_installer;
pub mod socket_client;
pub mod task_manager;
pub mod third_party_skill_registry;
pub mod tool_manifest;
pub mod tool_policy;
pub mod wasm_sandbox;

pub mod workflow_recorder;
pub mod workspace;

pub use agent_library::AgentLibraryService;
pub use agent_run_control::AgentRunControl;
pub use airlock::AirlockService;
pub use airlock_messages::{AirlockMessage, AirlockMessageStore};
pub use atm_client::ATMClient;
pub use browser_controller::BrowserController;
pub use command_poller::CommandPoller;
pub use document::DocumentService;
pub use file_manager::FileManager;
pub use file_operations::FileOperationEngine;
pub use folder_manager::FolderManager;
pub use image::ImageService;
pub use managed_research::ManagedResearchService;
pub use mcp_service::McpService;
pub use memory::MemoryManager;
pub use neural_service::NeuralService;
pub use prompt_skills::{
    DiscoveredPromptSkill, PromptSkillBinding, PromptSkillDiscoveryService, PromptSkillRegistry,
};
pub use security::NodeAuthenticator;
pub use skill_executor::SkillExecutor;

pub use llm_client::LLMClient;
pub use local_agent_security::{EffectiveLocalAgentPolicy, LocalAgentSecurityService};
pub use macos_native_notifications::MacOSNativeNotificationBridge;
pub use settings::SettingsManager;
pub use skill_installer::SkillInstaller;
pub use socket_client::SocketClient;
pub use task_manager::TaskManager;
pub use third_party_skill_registry::ThirdPartySkillRegistry;
pub use tool_policy::get_tool_policy;

pub use workflow_recorder::WorkflowRecorderService;
pub use workspace::{
    ConfigFormat, PermissionOverride, Workspace, WorkspaceAnalytics, WorkspaceManager,
    WorkspacePermissions, WorkspaceTemplate,
};
