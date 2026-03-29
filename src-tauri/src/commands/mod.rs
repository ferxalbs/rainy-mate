// Rainy MaTE - Tauri Commands Module
// Export all command handlers for registration with Tauri

pub mod agent;
pub mod agent_builder;
pub(crate) mod agent_frontend_events;
pub mod ai;
pub mod ai_providers;
pub mod airlock;
pub mod atm;
pub mod chat_artifacts;
pub mod deployment;

pub mod document;
pub mod file;
pub mod file_ops;
pub mod folder;
pub mod image;
pub mod mcp;
pub mod memory;
pub mod neural;
pub mod quick_delegate;

pub mod research;
pub mod router;
pub mod settings;
pub mod task;
pub mod unified_models;
pub mod web;
pub mod workflow_factory;
pub mod workspace;

pub use agent_builder::*;
pub use ai::*;
pub use ai_providers::*;
pub use airlock::*;
pub use atm::*;
pub use deployment::*;

pub use document::*;
pub use file::*;
pub use file_ops::*;
pub use folder::*;
pub use image::*;
pub use mcp::*;
pub use memory::*;
pub use neural::*;
pub use quick_delegate::*;

pub use router::*;
pub use settings::*;
pub use skills::*;
pub use task::*;
pub use unified_models::*;
pub use web::*;
pub use workflow_factory::*;
pub use workspace::*;

pub mod skills;
