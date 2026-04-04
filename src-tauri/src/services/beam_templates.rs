use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::process::Command;

use crate::models::neural::{
    AirlockLevel, CommandPriority, CommandStatus, QueuedCommand, RainyPayload,
};
use crate::services::beam_rpc::{
    BeamNetwork, BeamRpcService, GasEstimate, TransactionReceipt, TransactionRequest,
};
use crate::services::{AirlockService, MateLaunchpadService, WorkspaceManager};

const TEMPLATE_ROOT_SEGMENTS: &[&str] = &["templates", "beam"];
const TEMPLATE_MEMORY_MARKER_PREFIX: &str = "<!-- rainy-mate:beam-memory:";
const TEMPLATE_GUARDRAILS_MARKER_PREFIX: &str = "<!-- rainy-mate:beam-guardrails:";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BeamTemplateManifest {
    id: String,
    title: String,
    summary: String,
    description: String,
    contract_name: String,
    contract_file: String,
    category: String,
    recommended_network: String,
    tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BeamTemplateSummary {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub description: String,
    pub contract_name: String,
    pub contract_file: String,
    pub category: String,
    pub recommended_network: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BeamTemplateDetail {
    #[serde(flatten)]
    pub summary: BeamTemplateSummary,
    pub template_root: String,
    pub source_code: String,
    pub memory_markdown: String,
    pub guardrails_markdown: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BeamTemplateScaffoldResult {
    pub template_id: String,
    pub workspace_path: String,
    pub source_path: String,
    pub memory_file_path: String,
    pub guardrails_file_path: String,
    pub scaffolded_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BeamCompilationArtifact {
    pub abi: Value,
    pub abi_path: String,
    pub bytecode: String,
    pub bytecode_path: String,
    pub bytecode_size_bytes: usize,
    pub compiler_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BeamDeploymentTransactionPreview {
    pub kind: String,
    pub from: String,
    pub to: Option<String>,
    pub network: String,
    pub chain_id: u64,
    pub gas_limit: u64,
    pub gas_price: u64,
    pub estimated_fee_wei: String,
    pub estimated_fee_beam: String,
    pub explorer_url: String,
    pub rpc_url: String,
    pub data_bytes: usize,
    pub contract_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BeamDeploymentPlan {
    pub request_id: Option<String>,
    pub workspace_path: String,
    pub template: BeamTemplateSummary,
    pub network: String,
    pub wallet_address: String,
    pub source_path: String,
    pub build_dir: String,
    pub scaffolded_files: Vec<String>,
    pub memory_file_path: String,
    pub guardrails_file_path: String,
    pub compilation: BeamCompilationArtifact,
    pub gas_estimate: GasEstimate,
    pub transaction_preview: BeamDeploymentTransactionPreview,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BeamDeploymentResult {
    pub plan: BeamDeploymentPlan,
    pub receipt: TransactionReceipt,
}

#[derive(Clone)]
pub struct BeamTemplateService {
    beam_rpc: Arc<BeamRpcService>,
    repo_root: PathBuf,
}

impl BeamTemplateService {
    pub fn new(beam_rpc: Arc<BeamRpcService>) -> Self {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")))
            .to_path_buf();
        Self {
            beam_rpc,
            repo_root,
        }
    }

    pub fn list_templates(&self) -> Result<Vec<BeamTemplateSummary>, String> {
        let mut templates = self
            .template_dirs()?
            .into_iter()
            .map(|dir| self.load_template_summary_from_dir(&dir))
            .collect::<Result<Vec<_>, _>>()?;
        templates.sort_by(|a, b| a.title.cmp(&b.title));
        Ok(templates)
    }

    pub fn get_template(&self, template_id: &str) -> Result<BeamTemplateDetail, String> {
        let dir = self.template_dir(template_id)?;
        self.load_template_detail_from_dir(&dir)
    }

    pub async fn scaffold_template(
        &self,
        workspace_path: &str,
        template_id: &str,
    ) -> Result<BeamTemplateScaffoldResult, String> {
        let detail = self.get_template(template_id)?;
        let workspace_root = Path::new(workspace_path);
        let target_dir = workspace_root
            .join(".rainy-mate")
            .join("beam")
            .join("templates")
            .join(template_id);
        tokio::fs::create_dir_all(&target_dir)
            .await
            .map_err(|e| format!("Failed to create Beam template directory: {}", e))?;

        let source_path = target_dir.join(&detail.summary.contract_file);
        tokio::fs::write(&source_path, detail.source_code.as_bytes())
            .await
            .map_err(|e| format!("Failed to write template source: {}", e))?;

        let (memory_file_path, guardrails_file_path) =
            self.seed_workspace_overlay(workspace_root, &detail).await?;

        Ok(BeamTemplateScaffoldResult {
            template_id: template_id.to_string(),
            workspace_path: workspace_path.to_string(),
            source_path: source_path.to_string_lossy().to_string(),
            memory_file_path,
            guardrails_file_path,
            scaffolded_files: vec![source_path.to_string_lossy().to_string()],
        })
    }

    pub async fn prepare_deployment(
        &self,
        workspace_path: &str,
        template_id: &str,
        network: BeamNetwork,
        wallet_address: &str,
        request_id: Option<String>,
    ) -> Result<BeamDeploymentPlan, String> {
        let detail = self.get_template(template_id)?;
        self.beam_rpc
            .write_workspace_config(workspace_path, network)?;
        let scaffold = self.scaffold_template(workspace_path, template_id).await?;
        let build_dir = Path::new(workspace_path)
            .join(".rainy-mate")
            .join("beam")
            .join("build")
            .join(template_id);
        tokio::fs::create_dir_all(&build_dir)
            .await
            .map_err(|e| format!("Failed to create Beam build directory: {}", e))?;

        let compilation = self
            .compile_contract(
                Path::new(&scaffold.source_path),
                &detail.summary.contract_name,
                &build_dir,
            )
            .await?;
        let deployment_data = format!("0x{}", compilation.bytecode);
        let gas_estimate = self
            .beam_rpc
            .estimate_gas(
                workspace_path,
                wallet_address,
                None,
                Some("0x0"),
                Some(deployment_data.as_str()),
            )
            .await?;
        let workspace_cfg = self.beam_rpc.read_workspace_config(workspace_path)?;
        let preview = BeamDeploymentTransactionPreview {
            kind: "contract_creation".to_string(),
            from: wallet_address.to_string(),
            to: None,
            network: workspace_cfg.network.clone(),
            chain_id: workspace_cfg.chain.chain_id,
            gas_limit: gas_estimate.gas_limit,
            gas_price: gas_estimate.gas_price,
            estimated_fee_wei: gas_estimate.estimated_fee_wei.to_string(),
            estimated_fee_beam: gas_estimate.estimated_fee_beam.clone(),
            explorer_url: workspace_cfg.chain.explorer_url.clone(),
            rpc_url: workspace_cfg.chain.rpc_url.clone(),
            data_bytes: compilation.bytecode.len() / 2,
            contract_name: detail.summary.contract_name.clone(),
        };

        Ok(BeamDeploymentPlan {
            request_id,
            workspace_path: workspace_path.to_string(),
            template: detail.summary,
            network: workspace_cfg.network,
            wallet_address: wallet_address.to_string(),
            source_path: scaffold.source_path,
            build_dir: build_dir.to_string_lossy().to_string(),
            scaffolded_files: scaffold.scaffolded_files,
            memory_file_path: scaffold.memory_file_path,
            guardrails_file_path: scaffold.guardrails_file_path,
            compilation,
            gas_estimate,
            transaction_preview: preview,
        })
    }

    pub async fn deploy_template(
        &self,
        workspace_manager: &WorkspaceManager,
        airlock: Option<&AirlockService>,
        workspace_path: &str,
        template_id: &str,
        network: BeamNetwork,
        wallet_address: &str,
        request_id: Option<String>,
    ) -> Result<BeamDeploymentResult, String> {
        let plan = self
            .prepare_deployment(
                workspace_path,
                template_id,
                network,
                wallet_address,
                request_id.clone(),
            )
            .await?;

        if let Some(airlock) = airlock {
            let command = QueuedCommand {
                id: format!("beam_deploy_{}", uuid::Uuid::new_v4()),
                workspace_id: Some(workspace_path.to_string()),
                desktop_node_id: None,
                intent: "evm.beam_send_transaction".to_string(),
                payload: RainyPayload {
                    skill: Some("evm".to_string()),
                    method: Some("beam_send_transaction".to_string()),
                    params: Some(serde_json::json!({
                        "templateId": plan.template.id,
                        "templateTitle": plan.template.title,
                        "network": plan.network,
                        "walletAddress": plan.wallet_address,
                        "gasLimit": plan.gas_estimate.gas_limit,
                        "gasPrice": plan.gas_estimate.gas_price,
                        "estimatedFeeBeam": plan.gas_estimate.estimated_fee_beam,
                        "sourcePath": plan.source_path,
                        "buildDir": plan.build_dir,
                        "requestId": plan.request_id,
                        "transactionKind": "contract_creation",
                    })),
                    content: Some(format!(
                        "Deploy Beam template '{}' to {} with wallet {}",
                        plan.template.title, plan.network, plan.wallet_address
                    )),
                    allowed_paths: vec![workspace_path.to_string()],
                    blocked_paths: Vec::new(),
                    allowed_domains: Vec::new(),
                    blocked_domains: Vec::new(),
                    tool_access_policy: None,
                    tool_access_policy_version: None,
                    tool_access_policy_hash: None,
                    connector_id: None,
                    user_id: None,
                },
                priority: CommandPriority::High,
                status: CommandStatus::Pending,
                airlock_level: AirlockLevel::Dangerous,
                approval_timeout_secs: Some(30),
                approved_by: None,
                result: None,
                created_at: Some(chrono::Utc::now().timestamp_millis()),
                started_at: None,
                completed_at: None,
                schema_version: Some("beam-template-deploy-v1".to_string()),
            };

            let approved = airlock.check_permission(&command).await?;
            if !approved {
                if let Some(request_id) = request_id.as_deref() {
                    if let Ok(workspace) =
                        workspace_manager.ensure_workspace_for_path(workspace_path)
                    {
                        let _ = MateLaunchpadService::record_workspace_launch(
                            workspace_manager,
                            &workspace.id,
                            request_id,
                            "beam_deploy",
                            None,
                            false,
                            &[
                                "beam_estimate_gas".to_string(),
                                "beam_send_transaction".to_string(),
                            ],
                            &[workspace_path.to_string(), plan.source_path.clone()],
                            &[],
                        );
                    }
                }
                return Err(
                    "Beam deployment blocked by Airlock policy or user decision".to_string()
                );
            }
        }

        let receipt = self
            .beam_rpc
            .send_transaction(
                workspace_path,
                &TransactionRequest {
                    from: wallet_address.to_string(),
                    to: None,
                    value: Some("0x0".to_string()),
                    data: Some(format!("0x{}", plan.compilation.bytecode)),
                    gas_limit: Some(plan.gas_estimate.gas_limit),
                    gas_price: Some(plan.gas_estimate.gas_price),
                    nonce: None,
                },
            )
            .await?;

        if let Some(request_id) = request_id.as_deref() {
            if let Ok(workspace) = workspace_manager.ensure_workspace_for_path(workspace_path) {
                let _ = MateLaunchpadService::record_workspace_launch(
                    workspace_manager,
                    &workspace.id,
                    request_id,
                    "beam_deploy",
                    None,
                    true,
                    &[
                        "beam_rpc_connect".to_string(),
                        "beam_estimate_gas".to_string(),
                        "beam_send_transaction".to_string(),
                    ],
                    &[
                        workspace_path.to_string(),
                        plan.source_path.clone(),
                        plan.build_dir.clone(),
                    ],
                    &[
                        plan.compilation.abi_path.clone(),
                        plan.compilation.bytecode_path.clone(),
                    ],
                );
            }
        }

        Ok(BeamDeploymentResult { plan, receipt })
    }

    fn template_dirs(&self) -> Result<Vec<PathBuf>, String> {
        let root = self.templates_root();
        let entries = std::fs::read_dir(&root)
            .map_err(|e| format!("Failed to read Beam templates at {}: {}", root.display(), e))?;
        let mut dirs = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read template entry: {}", e))?;
            let path = entry.path();
            if path.is_dir() {
                dirs.push(path);
            }
        }
        if dirs.is_empty() {
            return Err(format!("No Beam templates found in {}", root.display()));
        }
        Ok(dirs)
    }

    fn template_dir(&self, template_id: &str) -> Result<PathBuf, String> {
        let dir = self.templates_root().join(template_id);
        if dir.is_dir() {
            Ok(dir)
        } else {
            Err(format!("Beam template '{}' was not found", template_id))
        }
    }

    fn templates_root(&self) -> PathBuf {
        TEMPLATE_ROOT_SEGMENTS
            .iter()
            .fold(self.repo_root.clone(), |path, segment| path.join(segment))
    }

    fn load_template_summary_from_dir(&self, dir: &Path) -> Result<BeamTemplateSummary, String> {
        let manifest = self.load_manifest(dir)?;
        Ok(BeamTemplateSummary {
            id: manifest.id,
            title: manifest.title,
            summary: manifest.summary,
            description: manifest.description,
            contract_name: manifest.contract_name,
            contract_file: manifest.contract_file,
            category: manifest.category,
            recommended_network: manifest.recommended_network,
            tags: manifest.tags,
        })
    }

    fn load_template_detail_from_dir(&self, dir: &Path) -> Result<BeamTemplateDetail, String> {
        let summary = self.load_template_summary_from_dir(dir)?;
        let source_code =
            std::fs::read_to_string(dir.join(&summary.contract_file)).map_err(|e| {
                format!(
                    "Failed to read template source {}: {}",
                    dir.join(&summary.contract_file).display(),
                    e
                )
            })?;
        let memory_markdown = std::fs::read_to_string(dir.join("MEMORY.md"))
            .map_err(|e| format!("Failed to read template MEMORY.md: {}", e))?;
        let guardrails_markdown = std::fs::read_to_string(dir.join("GUARDRAILS.md"))
            .map_err(|e| format!("Failed to read template GUARDRAILS.md: {}", e))?;
        Ok(BeamTemplateDetail {
            summary,
            template_root: dir.to_string_lossy().to_string(),
            source_code,
            memory_markdown,
            guardrails_markdown,
        })
    }

    fn load_manifest(&self, dir: &Path) -> Result<BeamTemplateManifest, String> {
        let raw = std::fs::read_to_string(dir.join("template.json"))
            .map_err(|e| format!("Failed to read template manifest: {}", e))?;
        serde_json::from_str(&raw).map_err(|e| format!("Failed to parse template manifest: {}", e))
    }

    async fn seed_workspace_overlay(
        &self,
        workspace_root: &Path,
        detail: &BeamTemplateDetail,
    ) -> Result<(String, String), String> {
        let overlay_dir = workspace_root.join(".rainy-mate");
        tokio::fs::create_dir_all(&overlay_dir)
            .await
            .map_err(|e| format!("Failed to create workspace overlay dir: {}", e))?;

        let memory_path = overlay_dir.join("MEMORY.md");
        let guardrails_path = overlay_dir.join("GUARDRAILS.md");
        self.append_overlay_block(
            &memory_path,
            format!("{}{} -->", TEMPLATE_MEMORY_MARKER_PREFIX, detail.summary.id).as_str(),
            detail.memory_markdown.as_str(),
            "# MEMORY\n\nCapture durable business context, preferences, and facts worth remembering across sessions.\n",
        )
        .await?;
        self.append_overlay_block(
            &guardrails_path,
            format!("{}{} -->", TEMPLATE_GUARDRAILS_MARKER_PREFIX, detail.summary.id).as_str(),
            detail.guardrails_markdown.as_str(),
            "# GUARDRAILS\n\nList non-negotiable rules, risks, and mistakes the agent must not repeat.\n",
        )
        .await?;
        Ok((
            memory_path.to_string_lossy().to_string(),
            guardrails_path.to_string_lossy().to_string(),
        ))
    }

    async fn append_overlay_block(
        &self,
        path: &Path,
        marker: &str,
        content: &str,
        default_content: &str,
    ) -> Result<(), String> {
        let existing = match tokio::fs::read_to_string(path).await {
            Ok(value) => value,
            Err(_) => default_content.to_string(),
        };
        if existing.contains(marker) {
            return Ok(());
        }
        let mut next = existing.trim_end().to_string();
        if !next.ends_with('\n') {
            next.push('\n');
        }
        next.push('\n');
        next.push_str(marker);
        next.push('\n');
        next.push_str(content.trim());
        next.push('\n');
        tokio::fs::write(path, next.as_bytes())
            .await
            .map_err(|e| format!("Failed to update {}: {}", path.display(), e))
    }

    async fn compile_contract(
        &self,
        source_path: &Path,
        contract_name: &str,
        build_dir: &Path,
    ) -> Result<BeamCompilationArtifact, String> {
        let output = Command::new("pnpm")
            .args([
                "exec",
                "solcjs",
                "--abi",
                "--bin",
                "--optimize",
                "--optimize-runs",
                "200",
                "--overwrite",
                "-o",
                build_dir
                    .to_str()
                    .ok_or_else(|| "Invalid build dir".to_string())?,
                source_path
                    .to_str()
                    .ok_or_else(|| "Invalid source path".to_string())?,
            ])
            .current_dir(&self.repo_root)
            .kill_on_drop(true)
            .output()
            .await
            .map_err(|e| format!("Failed to run solcjs: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("solcjs compilation failed: {}", stderr.trim()));
        }

        let base_name = source_path
            .file_stem()
            .and_then(|value| value.to_str())
            .ok_or_else(|| "Invalid source filename".to_string())?;
        let abi_path = build_dir.join(format!("{}_{}.abi", base_name, contract_name));
        let bytecode_path = build_dir.join(format!("{}_{}.bin", base_name, contract_name));
        let abi_raw = tokio::fs::read_to_string(&abi_path)
            .await
            .map_err(|e| format!("Failed to read generated ABI: {}", e))?;
        let bytecode = tokio::fs::read_to_string(&bytecode_path)
            .await
            .map_err(|e| format!("Failed to read generated bytecode: {}", e))?
            .trim()
            .to_string();
        if bytecode.is_empty() {
            return Err("Compiled bytecode was empty".to_string());
        }
        let abi = serde_json::from_str(&abi_raw)
            .map_err(|e| format!("Failed to parse generated ABI JSON: {}", e))?;

        Ok(BeamCompilationArtifact {
            abi,
            abi_path: abi_path.to_string_lossy().to_string(),
            bytecode_path: bytecode_path.to_string_lossy().to_string(),
            bytecode_size_bytes: bytecode.len() / 2,
            bytecode,
            compiler_version: "solcjs 0.8.34".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_service() -> BeamTemplateService {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let beam_rpc = Arc::new(BeamRpcService::new(tempdir.path().to_path_buf()));
        BeamTemplateService::new(beam_rpc)
    }

    #[test]
    fn beam_templates_are_available() {
        let service = make_service();
        let templates = service.list_templates().expect("templates");
        assert!(templates.len() >= 5);
        assert!(templates
            .iter()
            .any(|template| template.id == "simple-erc20"));
        assert!(templates.iter().any(|template| template.id == "ai-oracle"));
    }

    #[tokio::test]
    async fn overlay_seed_is_idempotent() {
        let service = make_service();
        let tempdir = tempfile::tempdir().expect("workspace");
        let detail = service.get_template("simple-erc20").expect("detail");

        let (memory_path, guardrails_path) = service
            .seed_workspace_overlay(tempdir.path(), &detail)
            .await
            .expect("seed once");
        service
            .seed_workspace_overlay(tempdir.path(), &detail)
            .await
            .expect("seed twice");

        let memory = tokio::fs::read_to_string(memory_path)
            .await
            .expect("memory file");
        let guardrails = tokio::fs::read_to_string(guardrails_path)
            .await
            .expect("guardrails file");

        assert_eq!(memory.matches(TEMPLATE_MEMORY_MARKER_PREFIX).count(), 1);
        assert_eq!(
            guardrails
                .matches(TEMPLATE_GUARDRAILS_MARKER_PREFIX)
                .count(),
            1
        );
    }
}
