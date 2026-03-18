pub mod discovery;
pub mod parser;
pub mod registry;

pub use crate::ai::specs::PromptSkillBinding;
pub use discovery::PromptSkillDiscoveryService;
pub use registry::{DiscoveredPromptSkill, PromptSkillRegistry};
