mod types;

pub use types::{WasmExecutionRequest, WasmExecutionResult};

use dashmap::DashMap;
use sha2::{Digest, Sha256};
use std::fs;
use std::net::IpAddr;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Semaphore;
use wasmtime::Config;
use wasmtime::{Engine, Linker, Module, ResourceLimiter, Store};
use wasmtime_wasi::p1;
use wasmtime_wasi::p1::WasiP1Ctx;
use wasmtime_wasi::p2::pipe::{MemoryInputPipe, MemoryOutputPipe};
use wasmtime_wasi::WasiCtxBuilder;
use wasmtime_wasi::{DirPerms, FilePerms};

struct SandboxCtx {
    wasi: WasiP1Ctx,
    limits: SandboxLimits,
}

struct SandboxLimits {
    max_memory_bytes: usize,
    max_table_elements: usize,
}

impl ResourceLimiter for SandboxLimits {
    fn memory_growing(
        &mut self,
        _current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> std::result::Result<bool, wasmtime::Error> {
        Ok(desired <= self.max_memory_bytes)
    }

    fn table_growing(
        &mut self,
        _current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> std::result::Result<bool, wasmtime::Error> {
        Ok(desired <= self.max_table_elements)
    }
}

pub struct WasmSandboxService {
    concurrency: Arc<Semaphore>,
    engine: Engine,
    module_cache: Arc<DashMap<String, Module>>,
    max_binary_bytes: usize,
    max_stdio_bytes: usize,
    max_memory_bytes: usize,
    fuel_limit: u64,
    exec_timeout_ms: u64,
}

impl Default for WasmSandboxService {
    fn default() -> Self {
        Self::new()
    }
}

impl WasmSandboxService {
    pub fn new() -> Self {
        let mut config = Config::new();
        config.consume_fuel(true);
        config.max_wasm_stack(512 * 1024);
        let engine = Engine::new(&config).expect("Failed to initialize Wasmtime engine");

        Self {
            concurrency: Arc::new(Semaphore::new(4)),
            engine,
            module_cache: Arc::new(DashMap::new()),
            max_binary_bytes: 8 * 1024 * 1024,
            max_stdio_bytes: 64 * 1024,
            max_memory_bytes: 50 * 1024 * 1024, // 50 MB max memory
            fuel_limit: 5_000_000,
            exec_timeout_ms: 3_000,
        }
    }

    pub fn sha256_hex(bytes: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        hex::encode(hasher.finalize())
    }

    pub fn validate_wasm_file(&self, path: &std::path::Path) -> Result<Vec<u8>, String> {
        let bytes = fs::read(path).map_err(|e| format!("Failed to read wasm binary: {}", e))?;
        if bytes.len() > self.max_binary_bytes {
            return Err(format!(
                "Wasm binary too large ({} bytes > {} bytes)",
                bytes.len(),
                self.max_binary_bytes
            ));
        }
        if bytes.len() < 8 || &bytes[0..4] != b"\0asm" {
            return Err("Invalid wasm binary header".to_string());
        }
        Ok(bytes)
    }

    pub async fn execute(&self, req: WasmExecutionRequest) -> WasmExecutionResult {
        let _permit = match self.concurrency.acquire().await {
            Ok(p) => p,
            Err(_) => {
                return WasmExecutionResult {
                    stdout: String::new(),
                    stderr: "Sandbox runtime unavailable".to_string(),
                    success: false,
                }
            }
        };

        let bytes = match self.validate_wasm_file(std::path::Path::new(&req.skill.binary_path)) {
            Ok(b) => b,
            Err(e) => {
                return WasmExecutionResult {
                    stdout: String::new(),
                    stderr: e,
                    success: false,
                }
            }
        };

        let params_value = serde_json::from_str::<serde_json::Value>(&req.params_json)
            .unwrap_or_else(|_| serde_json::Value::String(req.params_json.clone()));
        let network_results = match self
            .resolve_network_requests(&params_value, &req.skill.permissions.network_domains)
            .await
        {
            Ok(v) => v,
            Err(e) => {
                return WasmExecutionResult {
                    stdout: String::new(),
                    stderr: e,
                    success: false,
                }
            }
        };
        let envelope = serde_json::json!({
            "method": req.method.name,
            "params": params_value,
            "networkResults": network_results,
        })
        .to_string();

        let engine = self.engine.clone();
        let module_cache = self.module_cache.clone();
        let max_stdio_bytes = self.max_stdio_bytes;
        let max_memory_bytes = self.max_memory_bytes;
        let fuel_limit = self.fuel_limit;
        let bytes_for_exec = bytes;
        let module_sha = WasmSandboxService::sha256_hex(&bytes_for_exec);
        let fs_perms = req.skill.permissions.filesystem.clone();
        let envelope_for_exec = envelope;

        let task = tokio::task::spawn_blocking(move || {
            Self::execute_wasi_module_static(
                &engine,
                &module_cache,
                &module_sha,
                &bytes_for_exec,
                &envelope_for_exec,
                &fs_perms,
                max_stdio_bytes,
                max_memory_bytes,
                fuel_limit,
            )
        });

        match tokio::time::timeout(std::time::Duration::from_millis(self.exec_timeout_ms), task)
            .await
        {
            Ok(Ok(Ok(res))) => res,
            Ok(Ok(Err(e))) => WasmExecutionResult {
                stdout: String::new(),
                stderr: e,
                success: false,
            },
            Ok(Err(e)) => WasmExecutionResult {
                stdout: String::new(),
                stderr: format!("WASM sandbox worker panicked or was cancelled: {}", e),
                success: false,
            },
            Err(_) => WasmExecutionResult {
                stdout: String::new(),
                stderr: format!(
                    "WASM sandbox execution timed out after {}ms",
                    self.exec_timeout_ms
                ),
                success: false,
            },
        }
    }

    fn execute_wasi_module_static(
        engine: &Engine,
        module_cache: &DashMap<String, Module>,
        module_sha: &str,
        bytes: &[u8],
        params_json: &str,
        fs_perms: &[crate::services::third_party_skill_registry::SkillPermissionFs],
        max_stdio_bytes: usize,
        max_memory_bytes: usize,
        fuel_limit: u64,
    ) -> Result<WasmExecutionResult, String> {
        let module = if let Some(existing) = module_cache.get(module_sha) {
            existing.clone()
        } else {
            let compiled = Module::from_binary(engine, bytes)
                .map_err(|e| format!("Failed to compile wasm module: {}", e))?;
            module_cache.insert(module_sha.to_string(), compiled.clone());
            compiled
        };

        let stdin_pipe = MemoryInputPipe::new(params_json.as_bytes().to_vec());
        let stdout_pipe = MemoryOutputPipe::new(max_stdio_bytes);
        let stderr_pipe = MemoryOutputPipe::new(max_stdio_bytes);

        let mut builder = WasiCtxBuilder::new();
        builder.stdin(stdin_pipe.clone());
        builder.stdout(stdout_pipe.clone());
        builder.stderr(stderr_pipe.clone());
        for perm in fs_perms {
            let (dir_perms, file_perms) = Self::map_fs_mode(&perm.mode)?;
            if !Path::new(&perm.host_path).is_absolute() {
                return Err(format!(
                    "WASM sandbox filesystem permission host_path must be absolute: {}",
                    perm.host_path
                ));
            }
            if perm.guest_path.trim().is_empty() {
                return Err(
                    "WASM sandbox filesystem permission guest_path must be non-empty".to_string(),
                );
            }
            builder
                .preopened_dir(&perm.host_path, &perm.guest_path, dir_perms, file_perms)
                .map_err(|e| {
                    format!(
                        "Failed to preopen '{}' as '{}': {}",
                        perm.host_path, perm.guest_path, e
                    )
                })?;
        }
        let wasi = builder.build_p1();

        let ctx = SandboxCtx {
            wasi,
            limits: SandboxLimits {
                max_memory_bytes,
                max_table_elements: 10_000,
            },
        };

        let mut linker: Linker<SandboxCtx> = Linker::new(engine);
        p1::add_to_linker_sync(&mut linker, |cx| &mut cx.wasi)
            .map_err(|e| format!("Failed to add WASI linker imports: {}", e))?;

        let mut store: Store<SandboxCtx> = Store::new(engine, ctx);
        store.limiter(|cx| &mut cx.limits);
        store
            .set_fuel(fuel_limit)
            .map_err(|e| format!("Failed to set fuel limit: {}", e))?;

        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| format!("Failed to instantiate wasm module: {}", e))?;

        let start = instance
            .get_func(&mut store, "_start")
            .ok_or_else(|| "WASM module missing required WASI export '_start'".to_string())?;

        let mut results = [];
        let call_result = start.call(&mut store, &[], &mut results);

        let stdout = String::from_utf8_lossy(&stdout_pipe.contents()).to_string();
        let stderr_text = String::from_utf8_lossy(&stderr_pipe.contents()).to_string();

        match call_result {
            Ok(()) => Ok(WasmExecutionResult {
                stdout,
                stderr: stderr_text,
                success: true,
            }),
            Err(e) => Ok(WasmExecutionResult {
                stdout,
                stderr: if stderr_text.trim().is_empty() {
                    format!("WASM trap: {}", e)
                } else {
                    format!("{}\nWASM trap: {}", stderr_text, e)
                },
                success: false,
            }),
        }
    }

    fn map_fs_mode(mode: &str) -> Result<(DirPerms, FilePerms), String> {
        match mode.trim() {
            "read" => Ok((DirPerms::READ, FilePerms::READ)),
            "read_write" => Ok((DirPerms::all(), FilePerms::all())),
            other => Err(format!(
                "Unsupported filesystem permission mode '{}' (expected 'read' or 'read_write')",
                other
            )),
        }
    }

    async fn resolve_network_requests(
        &self,
        params_value: &serde_json::Value,
        allowed_domains: &[String],
    ) -> Result<serde_json::Value, String> {
        let requests = params_value
            .get("networkRequests")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        if requests.is_empty() {
            return Ok(serde_json::Value::Array(Vec::new()));
        }

        if allowed_domains.is_empty() {
            return Err(
                "WASM sandbox networkRequests provided but skill declares no network permissions"
                    .to_string(),
            );
        }
        if requests.len() > 4 {
            return Err(
                "WASM sandbox supports at most 4 networkRequests per execution".to_string(),
            );
        }

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| format!("Failed to create sandbox HTTP client: {}", e))?;

        let mut results = Vec::new();
        for (idx, item) in requests.iter().enumerate() {
            let id = item
                .get("id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("req_{}", idx));
            let url = item
                .get("url")
                .and_then(|v| v.as_str())
                .ok_or_else(|| format!("networkRequests[{}].url is required", idx))?;
            let response_type = item
                .get("responseType")
                .and_then(|v| v.as_str())
                .unwrap_or("text");

            Self::validate_http_url(url)?;
            Self::enforce_allowed_domains(url, allowed_domains)?;

            let response = client
                .get(url)
                .send()
                .await
                .map_err(|e| format!("Sandbox HTTP GET failed for '{}': {}", url, e))?;

            let status = response.status().as_u16();
            let body_bytes = response
                .bytes()
                .await
                .map_err(|e| format!("Failed to read response body for '{}': {}", url, e))?;
            let truncated = body_bytes.len() > self.max_stdio_bytes;
            let body_slice = if truncated {
                &body_bytes[..self.max_stdio_bytes]
            } else {
                body_bytes.as_ref()
            };

            let body = match response_type {
                "json" => {
                    let parsed = serde_json::from_slice::<serde_json::Value>(body_slice)
                        .map_err(|e| format!("Invalid JSON from '{}': {}", url, e))?;
                    parsed
                }
                "text" => {
                    serde_json::Value::String(String::from_utf8_lossy(body_slice).to_string())
                }
                other => {
                    return Err(format!(
                        "Unsupported networkRequests[{}].responseType '{}' (expected 'text' or 'json')",
                        idx, other
                    ));
                }
            };

            results.push(serde_json::json!({
                "id": id,
                "url": url,
                "status": status,
                "responseType": response_type,
                "truncated": truncated,
                "body": body,
            }));
        }

        Ok(serde_json::Value::Array(results))
    }

    fn validate_http_url(url: &str) -> Result<(), String> {
        let parsed = reqwest::Url::parse(url).map_err(|e| format!("Invalid URL: {}", e))?;
        match parsed.scheme() {
            "http" | "https" => {}
            _ => return Err("Only http:// and https:// URLs are allowed".to_string()),
        }

        let host = parsed
            .host_str()
            .ok_or_else(|| "URL must include a valid host".to_string())?
            .to_ascii_lowercase();
        if host == "localhost" || host.ends_with(".localhost") {
            return Err("localhost URLs are blocked".to_string());
        }

        if let Ok(ip) = host.parse::<IpAddr>() {
            match ip {
                IpAddr::V4(v4) => {
                    if v4.is_loopback() || v4.is_private() || v4.is_link_local() {
                        return Err("Private or loopback IPs are blocked".to_string());
                    }
                }
                IpAddr::V6(v6) => {
                    if v6.is_loopback() || v6.is_unique_local() || v6.is_unspecified() {
                        return Err("Private or loopback IPs are blocked".to_string());
                    }
                }
            }
        }
        Ok(())
    }

    fn enforce_allowed_domains(url: &str, allowed_domains: &[String]) -> Result<(), String> {
        let parsed = reqwest::Url::parse(url).map_err(|e| format!("Invalid URL: {}", e))?;
        let host = parsed
            .host_str()
            .ok_or_else(|| "URL must include a valid host".to_string())?
            .to_ascii_lowercase();

        if !allowed_domains
            .iter()
            .any(|rule| Self::domain_rule_matches(&host, rule))
        {
            return Err(format!(
                "Domain '{}' is not permitted by skill network permissions",
                host
            ));
        }
        Ok(())
    }

    fn domain_rule_matches(host: &str, rule: &str) -> bool {
        let normalized_host = host.trim().trim_end_matches('.').to_ascii_lowercase();
        let normalized_rule = rule.trim().trim_end_matches('.').to_ascii_lowercase();
        if normalized_rule.is_empty() {
            return false;
        }
        if normalized_rule == "*" {
            return true;
        }
        if let Some(root) = normalized_rule.strip_prefix("*.") {
            return normalized_host == root || normalized_host.ends_with(&format!(".{}", root));
        }
        normalized_host == normalized_rule
            || normalized_host.ends_with(&format!(".{}", normalized_rule))
    }
}

#[cfg(test)]
mod tests {
    use super::WasmSandboxService;

    #[test]
    fn map_fs_mode_accepts_supported_modes() {
        assert!(WasmSandboxService::map_fs_mode("read").is_ok());
        assert!(WasmSandboxService::map_fs_mode("read_write").is_ok());
    }

    #[test]
    fn map_fs_mode_rejects_invalid_mode() {
        let err =
            WasmSandboxService::map_fs_mode("write_only").expect_err("invalid mode should fail");
        assert!(err.contains("Unsupported filesystem permission mode"));
    }

    #[test]
    fn domain_rule_matches_handles_wildcards() {
        assert!(WasmSandboxService::domain_rule_matches(
            "api.example.com",
            "*.example.com"
        ));
        assert!(WasmSandboxService::domain_rule_matches(
            "example.com",
            "*.example.com"
        ));
        assert!(!WasmSandboxService::domain_rule_matches(
            "example.org",
            "*.example.com"
        ));
    }

    #[test]
    fn validate_http_url_blocks_private_and_localhost() {
        assert!(WasmSandboxService::validate_http_url("https://example.com").is_ok());
        assert!(WasmSandboxService::validate_http_url("http://localhost:3000").is_err());
        assert!(WasmSandboxService::validate_http_url("http://127.0.0.1").is_err());
    }
}
