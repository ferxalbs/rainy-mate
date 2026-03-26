use crate::models::neural::CommandResult;
use crate::services::third_party_skill_registry::{
    InstalledThirdPartyMethod, InstalledThirdPartySkill,
};

#[derive(Debug, Clone)]
pub struct WasmExecutionRequest {
    pub skill: InstalledThirdPartySkill,
    pub method: InstalledThirdPartyMethod,
    pub params_json: String,
}

#[derive(Debug, Clone)]
pub struct WasmExecutionResult {
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
}

impl WasmExecutionResult {
    pub fn into_command_result(self) -> CommandResult {
        if self.success {
            CommandResult {
                success: true,
                output: Some(self.stdout),
                error: if self.stderr.trim().is_empty() {
                    None
                } else {
                    Some(self.stderr)
                },
                exit_code: Some(0),
            }
        } else {
            CommandResult {
                success: false,
                output: if self.stdout.trim().is_empty() {
                    None
                } else {
                    Some(self.stdout)
                },
                error: Some(if self.stderr.trim().is_empty() {
                    "WASM sandbox execution failed".to_string()
                } else {
                    self.stderr
                }),
                exit_code: Some(1),
            }
        }
    }
}
