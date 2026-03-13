# Rainy MaTE — Agent Reference

> **Version**: 0.5.93 · **Stack**: Tauri 2 + Rust (engine) · Vite/React 19 (views) · Bun (ATM cloud)
> **Canonical truth**: `CHANGELOG.md` — read the last 2–3 entries before touching anything. Every validated invariant, architectural decision, and regression guard lives there with its version tag. Use it to reconstruct context without asking the user.

---

## 0. New Session Protocol

When resuming work, execute these steps in order — no guessing, no redundant questions:

1. **Read CHANGELOG.md** — last 2–3 entries for current version, recent changes, and active validations.
2. **Check ROADMAP.md** if the user mentions a planned feature — it tracks what's not built yet.
3. **Run `cargo check -q`** from `src-tauri/` — confirms no compile drift.
4. **Run `pnpm exec tsc --noEmit`** — confirms frontend type integrity.
5. **State your reading** — summarize current version and last 3 notable changes before proposing anything.

> Any work started without reading the CHANGELOG risks re-implementing completed work or breaking hardened invariants.

---

## 1. Architecture at a Glance

```
┌──────────────────────────────────────────────────────────────┐
│                  Rainy MaTE (Desktop)                        │
│  src/        React 19 UI — views only, zero business logic   │
│  src-tauri/  Rust engine ← ALL real work lives here          │
│    ├── ai/                LLM providers, router, agent loop   │
│    │   ├── agent/         ReAct workflow (Think + Act)        │
│    │   ├── providers/     Gemini, OpenAI, Anthropic, etc.     │
│    │   └── router/        IntelligentRouter (model selection) │
│    ├── commands/          Tauri invoke surface (thin layer)   │
│    ├── services/          ALL domain logic                    │
│    │   ├── skill_executor/     Agent tool dispatcher          │
│    │   │   ├── args.rs         Tool JSON schemas (schemars)   │
│    │   │   ├── registry.rs     Tool definitions + manifest    │
│    │   │   ├── filesystem.rs   File I/O handlers              │
│    │   │   ├── shell.rs        Shell / git wrappers           │
│    │   │   ├── web.rs          HTTP + search tools            │
│    │   │   └── browser.rs      Native CDP browser automation  │
│    │   ├── memory_vault/       AES-256-GCM encrypted SQLite   │
│    │   ├── memory/             MemoryManager + embedder       │
│    │   ├── wasm_sandbox/       Wasmtime third-party skill host│
│    │   ├── skill_installer/    skill.toml parser + verifier   │
│    │   ├── task_manager.rs     AI task queue + orchestration  │
│    │   ├── workspace.rs        Workspace lifecycle + perms    │
│    │   ├── file_operations.rs  AI-driven file ops + undo/redo │
│    │   ├── managed_research.rs AI web research service        │
│    │   ├── airlock.rs          3-tier permission gate         │
│    │   ├── tool_policy.rs      Canonical tool→risk mapping    │
│    │   ├── tool_manifest.rs    Runtime skill manifest builder  │
│    │   ├── neural_service.rs   ATM node registration+heartbeat│
│    │   ├── command_poller.rs   Cloud command polling loop     │
│    │   ├── atm_client.rs       Rainy ATM REST client          │
│    │   ├── manifest_signing.rs HMAC-SHA256 skill signing       │
│    │   └── third_party_skill_registry.rs  Installed skill DB  │
│    └── models/             Shared data types                  │
└──────────────────────────────────────────────────────────────┘
               ▲  WebSocket + REST
┌─────────────────────────┐
│  Rainy ATM (rainy-atm/) │  Bun runtime — Google Cloud Run and this project is private not is OSS 
│  Dynamic tool registry  │
│  Workspace lane queue   │
│  Cloud agent executor   │
│  Spec signing + audit   │
└─────────────────────────┘
```

**Iron law**: TypeScript/React is the view layer only. All logic, I/O, computation, and security enforcement runs in Rust. Never put business logic in `.tsx` or `.ts`.

---

## 2. Application Startup Order

Understanding `lib.rs` startup is critical — services are initialized in dependency order:

```
1. AIProviderManager, ProviderRegistry
2. TaskManager (needs AIProviderManager)
3. FileManager, FileOperationEngine, SettingsManager
4. ManagedResearchService (needs AIProviderManager)
5. DocumentService, ImageService
6. WorkspaceManager, IntelligentRouter
7. ATMClient, NodeAuthenticator, NeuralService
8. BrowserController
9. SkillExecutor (needs WorkspaceManager, ManagedResearch, BrowserController)
10. CommandPoller (needs NeuralService, SkillExecutor)
11. SocketClient, LLMClient
--- setup() block (has app handle) ---
12. tauri_plugin_updater (desktop only)
13. FolderManager (needs app_data_dir)
14. FileOperationEngine.init()
15. ATMClient.load_credentials_from_keychain() [async, best-effort]
16. NeuralService.load_credentials_from_keychain() [async, best-effort]
17. MemoryManager + MemoryVault init (needs app_data_dir)
18. SkillExecutor.set_memory_manager() [late binding]
19. AirlockService.new(app_handle)
20. Database.init() + AgentManager [BLOCKING — must succeed]
21. CommandPoller.start() [async — starts polling if credentials exist]
22. CloudBridge + SocketClient [async — connects after 3s delay]
```

> If you add a new managed service, follow this order. Services that need `app_data_dir` or `AppHandle` must go in the `setup()` block, not before it.

---

## 3. Module Map — Where to Touch What

| Task                        | Files to change                                                                                                      |
| --------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| Add a new agent tool        | `skill_executor/args.rs` → `registry.rs` → `filesystem/shell/web/browser.rs` → `tool_policy.rs` → `neural-config.ts` |
| Change ReAct loop logic     | `ai/agent/workflow.rs` (ThinkStep / ActStep)                                                                         |
| Change LLM provider routing | `ai/router/` + `ai/providers/`                                                                                       |
| Add a Tauri command         | `commands/<domain>.rs` → `lib.rs` `invoke_handler!`                                                                  |
| Add a new service           | `services/<name>.rs` → `services/mod.rs` → `lib.rs` startup order                                                    |
| Change memory/embedding     | `services/memory/` + `services/memory_vault/`                                                                        |
| Add a DB migration          | `src-tauri/migrations/<timestamp>_<desc>.sql`                                                                        |
| Add Wasm skill support      | `services/wasm_sandbox/` + `services/skill_installer/` + `services/third_party_skill_registry.rs`                    |
| Change Airlock policy       | `services/airlock.rs` + `services/tool_policy.rs`                                                                    |
| Change workspace logic      | `services/workspace.rs` + `commands/workspace.rs`                                                                    |
| Add file operation + undo   | `services/file_operations.rs` + `commands/file_ops.rs`                                                               |
| Add task queue logic        | `services/task_manager.rs` + `commands/task.rs`                                                                      |
| Add cloud ATM logic         | `rainy-atm/src/` (Bun — never Node.js)                                                                               |
| Add a UI component          | `src/components/<domain>/` — thin wrapper, `invoke()` only                                                           |
| Change neural state UI      | `src/components/agent-chat/neural-config.ts`                                                                         |

---

## 4. Decision Tree — Where Does This Logic Go?

```
Is it file I/O, DB, HTTP, security, data processing, or any computation?
  └─ YES → Rust (src-tauri/src/services/ or ai/)
         └─ Does it need to be called from UI?
              └─ YES → also add a thin command in commands/<domain>.rs
              └─ NO  → call it internally from another service
  └─ NO, purely UI state (modal open, tab selected, theme)?
        └─ TypeScript/React only — no invoke() needed

Does it involve new cloud/backend behavior?
  └─ YES → rainy-atm/src/ (Bun runtime, never Node)
  └─ NO  → desktop Rust

Does it touch the agent's tool surface?
  └─ YES → Add tool checklist (section 6)
  └─ NO  → standard service/command pattern
```

---

## 5. Build & Dev Commands

```bash
# Full Tauri dev loop (Rust + UI hot-reload)
pnpm run tauri dev

# Frontend only (no Rust recompile)
pnpm run dev

# ── Rust gates (run before EVERY commit) ──────────────────────
cd src-tauri && cargo check -q
cd src-tauri && cargo test

# ── TypeScript gate ───────────────────────────────────────────
pnpm exec tsc --noEmit

# ── Production bundle ─────────────────────────────────────────
pnpm run build

# ── ATM cloud backend (always Bun, never Node) ────────────────
cd rainy-atm && bun run dev
cd rainy-atm && bun test
cd rainy-atm && bun run build
cd rainy-atm && bunx tsc --noEmit

# ── Package management (pnpm only — npm is BANNED) ────────────
pnpm add <package>
pnpm remove <package>
pnpm run <script>
```

> **Gate rule**: `cargo check -q` AND `pnpm exec tsc --noEmit` must pass before any change is declared done. These are the exact gates used in every CHANGELOG validation block.

---

## 6. Agent Runtime — ReAct Loop (Rust)

The agent engine lives entirely in `src-tauri/src/ai/agent/`:

| File                | Role                                                                     |
| ------------------- | ------------------------------------------------------------------------ |
| `runtime.rs`        | `AgentRuntime` — entry point, context window, streaming                  |
| `workflow.rs`       | `ThinkStep` + `ActStep` + `AgentState`                                   |
| `context_window.rs` | Sliding token window (≈4 chars/token), evicts oldest non-system messages |
| `memory.rs`         | `AgentMemory` — semantic retrieval + long-term store                     |
| `manager.rs`        | `AgentManager` — SQLite persistence of agents + chat history             |
| `error.rs`          | `AgentError` enum with `is_retryable()` classifier                       |

**ReAct loop flow**:

1. **ThinkStep** → inject RAG context from memory vault → call `IntelligentRouter` → emit `StreamChunk` events → if tool calls → go to ActStep
2. **ActStep** → resolve tool policy → check Airlock → execute `SkillExecutor` → persist results to memory → return to ThinkStep
3. Workflow stops when ThinkStep produces no tool calls. Hard caps: 50 steps default, 200 max.

**Size limits** (constants in `workflow.rs`):

| Constant                   | Value | Purpose              |
| -------------------------- | ----- | -------------------- |
| `MAX_MODEL_MESSAGE_BYTES`  | 95 KB | Per-message cap      |
| `MAX_TOOL_TEXT_BYTES`      | 48 KB | Per tool result cap  |
| `MAX_MEMORY_CONTEXT_BYTES` | 24 KB | RAG injection budget |

**Streaming mode**: ThinkStep uses `complete_stream()` when there are no tools present, and blocking `complete()` when tool calls are expected (tool calls require a full response to parse).

---

## 7. Adding a New Tool — Checklist

1. **Schema** (`skill_executor/args.rs`): add `#[derive(Deserialize, JsonSchema)]` struct.
2. **Registry** (`skill_executor/registry.rs`): add `tool("name", "description", schema_for!(Args))` entry.
3. **Handler** (`skill_executor/filesystem.rs` | `shell.rs` | `web.rs` | `browser.rs`): implement in the matching domain file. Never grow the `skill_executor.rs` orchestrator.
4. **Policy** (`services/tool_policy.rs`): add `ToolPolicy { skill, airlock_level }` entry. Enforced by `every_registered_tool_has_explicit_policy_entry` test.
5. **Neural state** (`src/components/agent-chat/neural-config.ts`): map tool to existing `NeuralState` in `TOOL_STATE_MAP`, or define a new state.
6. **Validate**: `cargo check -q && cargo test && pnpm exec tsc --noEmit`.

> **Never add a tool without a policy entry.** The test `every_registered_tool_has_explicit_policy_entry` will fail CI.

---

## 8. Skill Executor — Complete Tool Catalog

All tool definitions are source-of-truth in `services/skill_executor/registry.rs`.
Every tool MUST have an entry in `services/tool_policy.rs` — enforced by compile-time regression test.

### Filesystem

| Tool                  | Level | Description                                                           |
| --------------------- | ----- | --------------------------------------------------------------------- |
| `read_file`           | L0    | Read file contents                                                    |
| `read_many_files`     | L0    | Read multiple UTF-8 files in one call                                 |
| `read_file_chunk`     | L0    | Read large file by byte offset                                        |
| `list_files`          | L0    | List directory contents                                               |
| `list_files_detailed` | L0    | List with size / mtime / type metadata                                |
| `file_exists`         | L0    | Check file/directory existence                                        |
| `get_file_info`       | L0    | Get size, timestamps, type                                            |
| `search_files`        | L0    | Regex search in file names and contents (case-insensitive by default) |
| `ingest_document`     | L0    | Ingest PDF/MD/TXT into semantic memory vault                          |
| `mkdir`               | L1    | Create directory                                                      |
| `write_file`          | L1    | Write file (creates or overwrites)                                    |
| `append_file`         | L1    | Append to file                                                        |
| `move_file`           | L2    | Move or rename file/directory                                         |
| `delete_file`         | L2    | Delete file or directory                                              |

### Shell & Git

| Tool              | Level | Description                                                                                  |
| ----------------- | ----- | -------------------------------------------------------------------------------------------- |
| `git_status`      | L0    | Git status                                                                                   |
| `git_diff`        | L0    | Git diff                                                                                     |
| `git_log`         | L0    | Recent commit history                                                                        |
| `git_show`        | L0    | Show commit/tag/file details                                                                 |
| `git_branch_list` | L0    | List local/remote branches                                                                   |
| `execute_command` | L2    | Shell command (allowlist: `npm`, `pnpm`, `cargo`, `git`, `bun`, `ls`, `grep`, `echo`, `cat`) |

### Web & HTTP

| Tool             | Level | Description                           |
| ---------------- | ----- | ------------------------------------- |
| `web_search`     | L0    | Search the web                        |
| `read_web_page`  | L0    | Headless static page scraper          |
| `http_get_json`  | L1    | Fetch JSON from HTTP(S) endpoint      |
| `http_get_text`  | L1    | Fetch text/HTML from HTTP(S) endpoint |
| `http_post_json` | L1    | POST JSON to HTTP(S) endpoint         |

### Browser (Native CDP)

| Tool                | Level | Description                           |
| ------------------- | ----- | ------------------------------------- |
| `screenshot`        | L0    | Screenshot current page               |
| `get_page_content`  | L0    | Get raw HTML of current page          |
| `get_page_snapshot` | L0    | URL + title + text preview            |
| `extract_links`     | L0    | Extract clickable links (href + text) |
| `browse_url`        | L1    | Open URL in visible browser           |
| `open_new_tab`      | L1    | Open URL in new browser tab           |
| `click_element`     | L1    | Click by CSS selector                 |
| `wait_for_selector` | L1    | Wait for CSS selector to appear       |
| `type_text`         | L1    | Type into input/textarea              |
| `submit_form`       | L1    | Submit form                           |
| `go_back`           | L1    | Browser back navigation               |

### Memory

| Tool              | Level | Description                                                 |
| ----------------- | ----- | ----------------------------------------------------------- |
| `ingest_document` | L0    | Index into semantic memory vault (same as filesystem entry) |

---

## 9. Airlock Security Model

All tool executions are gated through `services/airlock.rs` before `SkillExecutor` runs them.

| Level  | Name      | Behavior                      | Auto-approved headless? |
| ------ | --------- | ----------------------------- | ----------------------- |
| **L0** | Safe      | Auto-approved silently        | Yes                     |
| **L1** | Sensitive | Desktop notification gate     | Yes (headless)          |
| **L2** | Dangerous | Explicit UI approval required | No                      |

**Defense-in-depth enforcement** (all must pass, not just one):

1. Cloud ATM pre-queues by policy before sending commands.
2. `workflow.rs ActStep` checks `is_tool_allowed_by_spec()` before routing.
3. `SkillExecutor` performs a second deny-first check before execution.
4. Policy hash + version prevent replay attacks with stale policies.

**Scope enforcement** (enforced in Rust, never in UI):

- `blocked_paths`: filesystem/shell tools check path against this list before any I/O.
- `allowed_domains` / `blocked_domains`: browser/HTTP tools validate URL before any request.

> **Critical**: Effective Airlock level escalates to the canonical policy level when a command declares a lower level — downscoping bypass is blocked at `airlock.rs`.

---

## 10. Memory System

```
User input / web research / document ingestion
        │
        ▼
MemoryManager (services/memory/)
  • short-term: in-process ring buffer (100 entries)
  • long-term:  MemoryVaultService (AES-256-GCM encrypted libSQL/SQLite)
        │
        ▼
ThinkStep RAG injection (before every LLM call)
  • Semantic ANN search: vector_top_k (libSQL index)
  • Exact fallback:      vector_distance_cos
  • Embedder:            gemini-embedding-001 (3072d) — LOCKED, do not change
  • Top 5 hits injected as system context within MAX_MEMORY_CONTEXT_BYTES budget
  • Confidential memories gated by L2 Airlock before injection
```

**DB file**: `{app_data_dir}/memory_db/` (libSQL + SQLite)
**Encryption**: AES-256-GCM, per-entry nonce, key stored in macOS Keychain Services.
**Migrations**: `src-tauri/migrations/` — add new `.sql` files with `YYYYMMDDHHMMSS_<desc>.sql` naming.

> **Never change** the embedding provider from `gemini-embedding-001` or dimension from 3072. Dimension mismatches crash the vault. This is locked per CHANGELOG 0.5.91–0.5.93.

---

## 11. Wasm Skill Sandbox (QUARANTINE ZONE)

Third-party skills run in a hardened Wasmtime host:

```
skill.toml          — manifest (name, version, methods, permissions)
skill.wasm          — Ed25519-signed binary
        │
        ▼
skill_installer/    — parse + verify Ed25519 sig + domain collision check
third_party_skill_registry.rs — persist metadata in app data dir
        │
        ▼
wasm_sandbox/       — Wasmtime host
  • ResourceLimiter: < 50 MB memory per instance
  • Fuel limit + execution timeout (spawn_blocking + bounded timeout)
  • JSON stdin envelope (method + params) / captured stdout/stderr
  • WASI filesystem: preopened dirs only (read or read_write per manifest)
  • Network: host-mediated prefetch with domain allowlist + SSRF blocking
  • Module compilation cache by SHA-256
```

**Hard constraints**:

- Third-party method names cannot collide with built-in tool names (checked at install).
- Fail-closed: unknown capabilities are denied until explicit host bindings exist.
- Skill removal deletes both the registry entry and the installed package directory.

---

## 12. ATM Cloud Runtime (`rainy-atm/`)

Bun-based REST + WebSocket server deployed to Google Cloud Run.

| Component            | File                                   | Role                                                                   |
| -------------------- | -------------------------------------- | ---------------------------------------------------------------------- |
| Agent executor       | `src/services/agent-executor.ts`       | Runs cloud agent sessions                                              |
| Tool registry        | `src/services/tool-registry.ts`        | Dynamic tool loading per workspace/agent                               |
| Tool validator       | `src/tools/tool-validator.ts`          | Manifest sanitization (name regex, 20-method cap, 500-char desc limit) |
| Unified lane queue   | `src/services/unified-lane-queue.ts`   | Channel-agnostic Telegram/Discord queue                                |
| Workspace connectors | `src/services/workspace-connectors.ts` | Per-channel routing + rate limits                                      |
| Spec signing         | `src/services/spec-signing.ts`         | HMAC-SHA256 agent spec integrity                                       |
| Audit trail          | `src/services/tool-execution-audit.ts` | Immutable tool execution log                                           |

**Namespacing**: Dynamic tool calls use `skillName__methodName` format (double underscore) to prevent collisions. The ATM normalizes this before routing to desktop.

---

## 13. Neural State UI Mapping

When adding or modifying tools, update `src/components/agent-chat/neural-config.ts`:

| State           | Color   | Triggers                                                                                                           |
| --------------- | ------- | ------------------------------------------------------------------------------------------------------------------ |
| `thinking`      | Purple  | LLM generation / no tool                                                                                           |
| `planning`      | Amber   | `planning` tool                                                                                                    |
| `reading`       | Blue    | `web_search`, `read_web_page`                                                                                      |
| `observing`     | Emerald | `read_file`, `list_files`, `file_exists`, `get_file_info`, `read_file_chunk`, `read_many_files`, `search_files`    |
| `browsing`      | Orange  | `browse_url`, `click_element`, `type_text`, `screenshot`, `get_page_content`, `get_page_snapshot`, `extract_links` |
| `communicating` | Indigo  | `http_get_json`, `http_post_json`, `http_get_text`                                                                 |
| `creating`      | Pink    | `write_file`, `append_file`, `mkdir`                                                                               |
| `pruning`       | Red     | `delete_file`                                                                                                      |
| `executing`     | Cyan    | `execute_command`, `git_status`, `git_diff`, `git_log`, `git_show`, `git_branch_list`, `move_file`                 |

---

## 14. Versioning & Changelog Protocol

For **every significant change** (new features, breaking fixes, architectural changes) (but first you need the confirmation of the user):

1. **Update `CHANGELOG.md`** — use the existing format:
   ```
   ## [X.Y.Z] - YYYY-MM-DD - RELEASE-NAME
   ### Added / Changed / Fixed
   - Description with exact file paths
   ### Validation
   - Command run + result (pass/fail)
   ```
2. **Bump version in all three places simultaneously**:
   - `package.json` → `version`
   - `src-tauri/Cargo.toml` → `version`
   - `src-tauri/tauri.conf.json` → `version`
3. **Add a Validation block** — every command run during testing, with its result.

> **Version format**: `[MAJOR.MINOR.PATCH] - YYYY-MM-DD - RELEASE-NAME`

---

## 15. Code Rules (Non-Negotiable)

- **Rust for everything real**: file I/O, HTTP, DB, encryption, business logic, security — all Rust.
- **TypeScript for display only**: event handlers, `invoke()` calls, DOM/UI state (modals, tabs, theme).
- **No dead code**: remove unused code immediately. If future use, mark `// @TODO reason`, `// @deprecated reason`, or `// @RESERVED reason`. Unmarked dead code is a violation.
- **Modular, not monolithic**: each domain has its own file. Never grow `skill_executor.rs` orchestrator. Never let a service file become a catch-all.
- **pnpm only**: `npm` is banned everywhere. Use `bun` only inside `rainy-atm/`.
- **Conventional Commits** with scope: `feat(agent-chat): ...`, `fix(airlock): ...`, `refactor(memory): ...`
- **Secrets out of Git**: OS keychain and local `.env` files only. Never commit API keys.
- **Async everything**: use `tokio` for async I/O in Rust. Use `rayon` for CPU-bound parallel tasks.
- **Error handling**: propagate `Result<T, String>` through Tauri commands. Use typed error enums inside services.

---

## 16. Anti-Patterns (Explicitly Forbidden)

```rust
// ❌ WRONG — business logic in a Tauri command
#[command]
pub async fn process_users(users: Vec<User>) -> Result<Vec<User>, String> {
    let filtered = users.into_iter().filter(|u| u.active).collect();
    // sorting, filtering, transforming — all in command = wrong
    Ok(sorted)
}

// ✅ RIGHT — command delegates to a service
#[command]
pub async fn process_users(
    users: Vec<User>,
    svc: State<'_, UserService>,
) -> Result<Vec<User>, String> {
    svc.process(users).await.map_err(|e| e.to_string())
}
```

```typescript
// ❌ WRONG — logic in TypeScript
const filtered = users
  .filter((u) => u.active)
  .sort((a, b) => b.score - a.score);
await invoke("save_users", { users: filtered });

// ✅ RIGHT — delegate entirely to Rust
const result = await invoke<ProcessedUsers>("process_and_save_users", {
  users,
});
```

```rust
// ❌ WRONG — growing the skill_executor.rs orchestrator
// In services/skill_executor.rs:
pub async fn handle_new_feature(...) { /* 200 lines of logic */ }

// ✅ RIGHT — add a new domain file
// services/skill_executor/new_domain.rs → thin call from skill_executor.rs
```

---

## 17. Troubleshooting Reference

| Symptom                        | Investigation Path                                                                       |
| ------------------------------ | ---------------------------------------------------------------------------------------- |
| Agent not responding           | Check `src-tauri` terminal logs; verify `rainy-atm` reachable at ATM URL in `lib.rs`     |
| Tool blocked / unexpected deny | Check `AirlockPanel` in UI; inspect `tool_policy.rs` entry for that tool name            |
| Tool missing from model        | Check `registry.rs` entry; run `cargo test manifest_covers_every_registered_tool`        |
| Policy test failing            | New tool added to registry but missing `tool_policy.rs` entry — add it                   |
| TypeScript errors              | `pnpm exec tsc --noEmit`; check `src/services/tauri.ts` for missing command wrappers     |
| Memory vault crash             | Embedding dimension mismatch — embedder must be `gemini-embedding-001` (3072d)           |
| Ghost nodes after reconnect    | Ensure `CommandPoller.stop()` + `CloudBridge.stop()` called before `clear_credentials()` |
| Wasm skill timeout             | Check fuel limit + execution timeout in `wasm_sandbox/mod.rs`                            |
| `cargo test` libSQL panic      | Static `libsqlite3-sys` conflict — see CHANGELOG 0.5.93 for initialization order fix     |
| Duplicate agents on deploy     | ATM upserts by `config.id` — check `routes/admin.ts` and `POST /admin/agents`            |
| Credential keychain miss       | `load_credentials_from_keychain()` is best-effort async — check startup logs             |

---

## 18. Domain-Specific Test Commands

Run the narrowest test scope that covers your change. Never just run `cargo test` blind:

```bash
# Agent ReAct loop
cd src-tauri && cargo test -q context_window --lib
cd src-tauri && cargo test -q agent --lib

# Tool policy integrity
cd src-tauri && cargo test -q manifest_covers_every_registered_tool --lib
cd src-tauri && cargo test -q every_registered_tool_has_explicit_policy_entry --lib

# Airlock security
cd src-tauri && cargo test -q airlock --lib

# Memory vault + vector search
cd src-tauri && cargo test -q memory_vault --lib

# Wasm sandbox
cd src-tauri && cargo test -q wasm_sandbox::tests --lib

# Third-party skill registry
cd src-tauri && cargo test -q third_party_skill_registry::tests --lib

# Manifest signing
cd src-tauri && cargo test -q manifest_signing --lib

# SkillExecutor full suite
cd src-tauri && cargo test -q skill_executor --lib

# ATM cloud (Bun)
cd rainy-atm && bun test
```

---

## 19. Key File Index (Quick Reference)

```
src-tauri/src/
  lib.rs                                ← App startup + all Tauri state wiring
  ai/agent/workflow.rs                  ← ThinkStep + ActStep (ReAct engine)
  ai/agent/runtime.rs                   ← AgentRuntime, streaming, context window
  ai/agent/manager.rs                   ← AgentManager, SQLite agent persistence
  ai/router/                            ← IntelligentRouter, model selection
  ai/providers/                         ← Gemini, OpenAI, Anthropic provider impls
  services/skill_executor.rs            ← SkillExecutor orchestrator (keep thin)
  services/skill_executor/
    registry.rs                         ← Tool definitions (source of truth)
    args.rs                             ← Tool JSON schemas
    filesystem.rs                       ← File I/O execution
    shell.rs                            ← Shell + git execution
    web.rs                              ← HTTP + search execution
    browser.rs                          ← CDP browser execution
  services/airlock.rs                   ← Permission gate (3-tier)
  services/tool_policy.rs               ← Canonical tool→risk mapping
  services/tool_manifest.rs             ← Runtime skill manifest builder
  services/memory_vault/                ← AES-256-GCM encrypted SQLite memory
  services/memory/                      ← MemoryManager + embedder
  services/wasm_sandbox/                ← Wasmtime sandbox host
  services/task_manager.rs              ← AI task queue + orchestration
  services/workspace.rs                 ← Workspace lifecycle + permissions
  services/file_operations.rs           ← AI-driven file ops + undo/redo
  services/managed_research.rs          ← AI web research service
  services/third_party_skill_registry.rs ← Installed skill metadata
  services/neural_service.rs            ← ATM node registration + heartbeat
  services/command_poller.rs            ← Cloud command polling + dispatch
  services/atm_client.rs                ← REST client for Rainy ATM
  services/manifest_signing.rs          ← HMAC-SHA256 skill manifest signing
  commands/                             ← All Tauri invoke() command handlers
src/
  components/agent-chat/neural-config.ts ← Tool → neural state mapping
  services/tauri.ts                     ← All Tauri invoke() wrappers
rainy-atm/src/
  services/tool-registry.ts            ← Dynamic tool loading
  services/agent-executor.ts           ← Cloud agent session runner
  services/spec-signing.ts             ← HMAC-SHA256 spec integrity
  services/tool-execution-audit.ts     ← Immutable execution log
CHANGELOG.md                           ← READ THIS. Primary state oracle.
ROADMAP.md                             ← What is planned but not built yet
```

---

# Changes and Versioning

> **IRON LAW — read before touching any version or changelog:** Every release MUST be backward-compatible by default. Breaking changes are forbidden without an explicit written authorization from the user and a completed compatibility plan. When in doubt, ask.

---

## 1. Mandatory Ask-First Protocol

**Before performing *any* of the following actions, you MUST stop and ask the user for explicit confirmation:**

| Action | Why you must ask |
|--------|-----------------|
| Update `CHANGELOG.md` | User decides what gets recorded and how |
| Bump version in `package.json`, `Cargo.toml`, or `tauri.conf.json` | User controls the release cadence |
| Label a change as a breaking change (MAJOR bump) | Triggers the Breaking Change Plan (see §4) |
| Deprecate any public API, command, or data format | May affect existing users or integrations |
| Remove or rename any Tauri `invoke()` command | Always a breaking change for the UI layer |

> **No exceptions.** Do not auto-commit, auto-bump, or auto-log. Present what you propose to log/bump and wait for approval.

---

## 2. Backward Compatibility Policy (Default: ALWAYS ON)

Every release — patch, minor, or major channel — **must preserve compatibility with the previous release** unless the user has explicitly authorized a breaking change in writing.

Compatibility invariants that must never be broken without authorization:

- **Tauri command surface**: All existing `invoke()` command names and their parameter shapes must remain unchanged.
- **SQLite schema**: Migrations are additive only (new columns/tables). Dropping or renaming columns requires a plan.
- **Tool registry names**: Removing or renaming a registered tool name breaks the Airlock policy test and ATM routing.
- **Memory vault embedding dimension**: Locked at 3072d / `gemini-embedding-001`. Never change.
- **ATM API contracts**: Any change to REST/WebSocket message shapes requires a versioned migration.
- **Skill manifest format** (`skill.toml`): Additive fields only. Removing fields breaks installed third-party skills.

---

## 3. Versioning Strategy (SemVer)

We use **Semantic Versioning** (`MAJOR.MINOR.PATCH`) across three files simultaneously:

| File | Field |
|------|-------|
| `package.json` | `version` |
| `src-tauri/Cargo.toml` | `version` |
| `src-tauri/tauri.conf.json` | `version` |

### Bump Rules

| Change type | Bump | Backward-compatible? |
|-------------|------|----------------------|
| Bug fix, internal refactor | `PATCH` | ✅ Always |
| New feature, new tool, new command | `MINOR` | ✅ Additive only |
| Removed / renamed API, schema drop, protocol change | `MAJOR` | ❌ Requires plan (§4) |

> **Never bump without user approval.** Propose the bump, wait for a green light, then apply it.

---

## 4. Breaking Change Plan (Required for Any MAJOR Bump)

If — and only if — the user explicitly authorizes a breaking change, you must produce and get approval for a **Breaking Change Plan** before writing a single line of code. The plan must cover all of the following:

### 4.1 Impact Assessment
- List every component, command, or data format that will break.
- Identify all active users / integrations affected (ATM, desktop app, third-party skills).

### 4.2 Migration Strategy
- Provide a step-by-step migration path for each breaking point.
- Specify whether a compatibility shim (adapter layer) can be used to support both old and new simultaneously during a transition window.
- For SQLite: write the exact migration SQL and define rollback SQL.
- For Tauri commands: keep old command names as deprecated wrappers calling the new implementation (at minimum one MINOR version).

### 4.3 Rollback Plan
- Define exactly how to revert if the new release causes unexpected failures in production.
- Identify which data (if any) is irreversibly mutated and cannot be rolled back.

### 4.4 Validation Gates
All of the following must pass **before** the breaking change ships:

```bash
cd src-tauri && cargo check -q
cd src-tauri && cargo test
pnpm exec tsc --noEmit
cd src-tauri && cargo test -q every_registered_tool_has_explicit_policy_entry --lib
cd src-tauri && cargo test -q manifest_covers_every_registered_tool --lib
```

### 4.5 CHANGELOG Entry
The CHANGELOG entry for a breaking change must include:

```
## [X.Y.Z] - YYYY-MM-DD - RELEASE-NAME
### ⚠️ BREAKING CHANGES
- What broke and why
- Migration path for each breaking point
- Files changed
### Validation
- Command run + result (pass/fail)
```

> **No breaking change ships without a completed plan reviewed and approved by the user.**

---

## 5. Changelog Format

```
## [X.Y.Z] - YYYY-MM-DD - RELEASE-NAME
### Added / Changed / Fixed / Deprecated / Removed
- Description with exact file paths affected
### Validation
- `cargo check -q` → ✅ pass
- `pnpm exec tsc --noEmit` → ✅ pass
- (any other test commands run)
```

---

## 20. Design System Rules (Mandatory)

Before implementing ANY UI component or visual design, you MUST follow these rules:

### 20.1 Component Library Selection

**Always ask the user** which component library to use before starting any UI work:

| Library | When to Use | MCP Server |
|---------|-------------|------------|
| **shadcn/ui** | Default choice for modern, accessible components | Use `shadcn` MCP server |
| **HeroUI v3** | When compound components or specific HeroUI features are needed | Use HeroUI MCP server |

> **IMPORTANT**: Never assume which library to use. Ask the user explicitly: "Should I use shadcn/ui or HeroUI v3 for this component?"

### 20.2 Background Opacity Standards

Apply background opacity based on the theme mode and component context:

| Context | Light Mode | Dark Mode |
|---------|-------------|------------|
| **Overlay/Modal** | `bg-background/85` | `bg-background/20` |
| **Cards/Panels** | `bg-background/80` | `bg-background/30` |
| **Floating Elements** | `bg-background/90` | `bg-background/20` |
| **Sidebar/Navigation** | `bg-background/95` | `bg-background/10` |

### 20.3 Backdrop Blur Standards

Use backdrop blur to create depth and separation between layers:

| Blur Level | Use Case | Tailwind Class |
|------------|----------|----------------|
| **Medium (Default)** | Cards, modals, dropdowns | `backdrop-blur-md` |
| **Extra Large** | Full-screen overlays, hero sections | `backdrop-blur-2xl` |
| **Subtle** | Navigation bars, sticky headers | `backdrop-blur-sm` |

### 20.4 Complete Design Pattern

**Standard glass morphism card:**
```tsx
// Light mode
<div className="backdrop-blur-md bg-background/80 border border-white/20 rounded-xl shadow-lg">

// Dark mode
<div className="backdrop-blur-md bg-background/30 border border-gray-700/50 rounded-xl shadow-lg">
```

**Modal overlay:**
```tsx
// Light mode
<div className="fixed inset-0 backdrop-blur-md bg-black/40 flex items-center justify-center">
  <div className="backdrop-blur-md bg-background/85 rounded-2xl shadow-2xl p-8">

// Dark mode  
<div className="fixed inset-0 backdrop-blur-md bg-black/60 flex items-center justify-center">
  <div className="backdrop-blur-md bg-background/20 rounded-2xl shadow-2xl p-8">
```

**Sticky navigation:**
```tsx
// Light mode
<nav className="sticky top-0 z-50 backdrop-blur-md bg-background/90 border-b border-gray-200/50>

// Dark mode
<nav className="sticky top-0 z-50 backdrop-blur-md bg-background/20 border-b border-gray-700/50>
```

### 20.5 Accessibility Considerations

- Always ensure sufficient contrast ratios (WCAG 2.1 AA standard)
- Test text readability against blurred backgrounds
- Consider users with motion sensitivity
- Provide alternative high-contrast mode when needed

---

**End of AGENTS.md**