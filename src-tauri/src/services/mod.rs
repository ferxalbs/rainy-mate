// Rainy Cowork - Services Module
// Business logic layer

pub mod airlock;
pub mod atm_client;
pub mod browser_controller;
pub mod cloud_bridge;
pub mod command_poller;
pub mod document;
pub mod file_manager;
pub mod file_operations;
pub mod folder_manager;
pub mod image;
pub mod llm_client;
pub mod managed_research; // Phase 3 AI Research
pub mod manifest_signing;
pub mod memory;
pub mod neural_service;
pub mod security;
pub mod settings;
pub mod skill_executor;
pub mod socket_client;
pub mod task_manager;
pub mod tool_policy;

pub mod workspace;

pub use airlock::AirlockService;
pub use atm_client::ATMClient;
pub use browser_controller::BrowserController;
pub use command_poller::CommandPoller;
pub use document::DocumentService;
pub use file_manager::FileManager;
pub use file_operations::FileOperationEngine;
pub use folder_manager::FolderManager;
pub use image::ImageService;
pub use managed_research::ManagedResearchService;
pub use memory::MemoryManager;
pub use neural_service::NeuralService;
pub use security::NodeAuthenticator;
pub use skill_executor::SkillExecutor;

pub use llm_client::LLMClient;
pub use settings::SettingsManager;
pub use socket_client::SocketClient;
pub use task_manager::TaskManager;
pub use tool_policy::get_tool_policy;

pub use workspace::{
    ConfigFormat, PermissionOverride, Workspace, WorkspaceAnalytics, WorkspaceManager,
    WorkspacePermissions, WorkspaceTemplate,
};
