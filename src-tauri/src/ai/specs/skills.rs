use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentSkills {
    pub capabilities: Vec<Capability>,
    // Map of tool_name -> config
    pub tools: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Permission {
    Read,
    Write,
    Execute,
    Network,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    pub name: String, // e.g., "filesystem", "browser"
    pub description: String,
    pub scopes: Vec<String>, // e.g., "/Users/fer/Documents", "*.google.com"
    pub permissions: Vec<Permission>,
}
