use crate::models::neural::{AirlockLevel, ParameterSchema};
use crate::services::third_party_skill_registry::{
    InstalledThirdPartyMethod, InstalledThirdPartySkill, SkillPermissionFs, SkillPermissions,
    ThirdPartySkillRegistry,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct SkillToml {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_runtime")]
    pub runtime: String,

    pub binary: SkillBinary,
    #[serde(default)]
    pub permissions: SkillTomlPermissions,
    #[serde(default)]
    pub methods: Vec<SkillTomlMethod>,
    #[serde(default)]
    pub signature: Option<SkillTomlSignature>,
}

fn default_runtime() -> String {
    "wasi-core-v1".to_string()
}

#[derive(Debug, Deserialize)]
pub struct SkillBinary {
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct SkillTomlPermissions {
    #[serde(default)]
    pub filesystem: Vec<SkillTomlFsPerm>,
    #[serde(default)]
    pub network: SkillTomlNetworkPerm,
}

#[derive(Debug, Deserialize)]
pub struct SkillTomlFsPerm {
    pub guest_path: String,
    pub host_path: String,
    pub mode: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct SkillTomlNetworkPerm {
    #[serde(default)]
    pub domains: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct SkillTomlMethod {
    pub name: String,
    pub description: String,
    pub airlock_level: AirlockLevel,
    #[serde(default)]
    pub parameters: HashMap<String, SkillTomlParameter>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SkillTomlParameter {
    #[serde(rename = "type")]
    pub param_type: String,
    #[serde(default)]
    pub required: Option<bool>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SkillTomlSignature {
    pub algorithm: String,
    pub digest: String,
}

impl SkillToml {
    pub fn validate(&self) -> Result<(), String> {
        if self.id.trim().is_empty()
            || self.name.trim().is_empty()
            || self.version.trim().is_empty()
        {
            return Err("skill.toml requires id, name, version".to_string());
        }
        if self.runtime != "wasi-core-v1" {
            return Err(format!(
                "Unsupported runtime '{}'; expected wasi-core-v1",
                self.runtime
            ));
        }
        if self.methods.is_empty() {
            return Err("skill.toml must declare at least one method".to_string());
        }
        Ok(())
    }

    pub fn into_installed(
        self,
        binary_path: String,
        install_source: &str,
        trust_state: &str,
    ) -> InstalledThirdPartySkill {
        InstalledThirdPartySkill {
            id: self.id,
            name: self.name,
            version: self.version,
            author: self.author,
            description: self.description,
            runtime: self.runtime,
            binary_path,
            binary_sha256: self.binary.sha256,
            enabled: true,
            trust_state: trust_state.to_string(),
            install_source: install_source.to_string(),
            installed_at: ThirdPartySkillRegistry::now_ts(),
            permissions: SkillPermissions {
                filesystem: self
                    .permissions
                    .filesystem
                    .into_iter()
                    .map(|p| SkillPermissionFs {
                        guest_path: p.guest_path,
                        host_path: p.host_path,
                        mode: p.mode,
                    })
                    .collect(),
                network_domains: self.permissions.network.domains,
            },
            methods: self
                .methods
                .into_iter()
                .map(|m| InstalledThirdPartyMethod {
                    name: m.name,
                    description: m.description,
                    airlock_level: m.airlock_level,
                    parameters: m
                        .parameters
                        .into_iter()
                        .map(|(k, v)| {
                            (
                                k,
                                ParameterSchema {
                                    param_type: v.param_type,
                                    required: v.required,
                                    description: v.description,
                                },
                            )
                        })
                        .collect(),
                })
                .collect(),
        }
    }
}
