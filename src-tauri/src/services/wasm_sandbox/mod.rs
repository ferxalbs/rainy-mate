mod types;

pub use types::{WasmExecutionRequest, WasmExecutionResult};

use sha2::{Digest, Sha256};
use std::fs;
use std::sync::Arc;
use tokio::sync::Semaphore;
use wasmtime::{Config, Engine, Linker, Module, Store};
use wasmtime_wasi::p1;
use wasmtime_wasi::p1::WasiP1Ctx;
use wasmtime_wasi::p2::pipe::{MemoryInputPipe, MemoryOutputPipe};
use wasmtime_wasi::WasiCtxBuilder;

pub struct WasmSandboxService {
    concurrency: Arc<Semaphore>,
    max_binary_bytes: usize,
    max_stdio_bytes: usize,
    fuel_limit: u64,
}

impl Default for WasmSandboxService {
    fn default() -> Self {
        Self::new()
    }
}

impl WasmSandboxService {
    pub fn new() -> Self {
        Self {
            concurrency: Arc::new(Semaphore::new(4)),
            max_binary_bytes: 8 * 1024 * 1024,
            max_stdio_bytes: 64 * 1024,
            fuel_limit: 5_000_000,
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

        if !req.skill.permissions.filesystem.is_empty() {
            return WasmExecutionResult {
                stdout: String::new(),
                stderr: format!(
                    "WASM sandbox execution denied: filesystem capabilities for '{}.{}' are not yet enabled in runtime host",
                    req.skill.id, req.method.name
                ),
                success: false,
            };
        }
        if !req.skill.permissions.network_domains.is_empty() {
            return WasmExecutionResult {
                stdout: String::new(),
                stderr: format!(
                    "WASM sandbox execution denied: network capabilities for '{}.{}' are not yet enabled in runtime host",
                    req.skill.id, req.method.name
                ),
                success: false,
            };
        }

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
        let envelope = serde_json::json!({
            "method": req.method.name,
            "params": params_value,
        })
        .to_string();

        match self.execute_wasi_module(&bytes, &envelope) {
            Ok(res) => res,
            Err(e) => WasmExecutionResult {
                stdout: String::new(),
                stderr: e,
                success: false,
            },
        }
    }

    fn execute_wasi_module(&self, bytes: &[u8], params_json: &str) -> Result<WasmExecutionResult, String> {
        let mut config = Config::new();
        config.async_support(false);
        config.consume_fuel(true);
        config.max_wasm_stack(512 * 1024);

        let engine = Engine::new(&config).map_err(|e| format!("Failed to create Wasmtime engine: {}", e))?;
        let module = Module::from_binary(&engine, bytes)
            .map_err(|e| format!("Failed to compile wasm module: {}", e))?;

        let stdin_pipe = MemoryInputPipe::new(params_json.as_bytes().to_vec());
        let stdout_pipe = MemoryOutputPipe::new(self.max_stdio_bytes);
        let stderr_pipe = MemoryOutputPipe::new(self.max_stdio_bytes);

        let mut builder = WasiCtxBuilder::new();
        builder.stdin(stdin_pipe.clone());
        builder.stdout(stdout_pipe.clone());
        builder.stderr(stderr_pipe.clone());
        let wasi = builder.build_p1();

        let mut linker: Linker<WasiP1Ctx> = Linker::new(&engine);
        p1::add_to_linker_sync(&mut linker, |cx| cx)
            .map_err(|e| format!("Failed to add WASI linker imports: {}", e))?;

        let mut store: Store<WasiP1Ctx> = Store::new(&engine, wasi);
        store
            .set_fuel(self.fuel_limit)
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
}
