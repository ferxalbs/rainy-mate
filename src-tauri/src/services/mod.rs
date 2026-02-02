// Rainy Cowork - Services Module
// Business logic layer

pub mod ai_agent;
pub mod atm_client;
pub mod document;
pub mod file_manager;
pub mod file_operations;
pub mod folder_manager;
pub mod image;
pub mod managed_research; // Phase 3 AI Research
pub mod memory;
pub mod reflection;
pub mod settings;
pub mod task_manager;
pub mod web_research;
pub mod workspace;

pub use ai_agent::CoworkAgent;
pub use atm_client::ATMClient;
pub use document::DocumentService;
pub use file_manager::FileManager;
pub use file_operations::FileOperationEngine;
pub use folder_manager::FolderManager;
pub use image::ImageService;
pub use managed_research::ManagedResearchService;
pub use memory::MemoryManager;
pub use reflection::ReflectionEngine;
pub use settings::SettingsManager;
pub use task_manager::TaskManager;
pub use web_research::WebResearchService;
pub use workspace::{
    ConfigFormat, PermissionOverride, Workspace, WorkspaceAnalytics, WorkspaceManager,
    WorkspacePermissions, WorkspaceTemplate,
};
