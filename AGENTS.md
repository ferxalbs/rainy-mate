# Rainy MaTE — Agent Reference

> **Version**: 0.5.93 · **Stack**: Tauri 2 + Rust (engine) · Vite/React 19 (views) · Bun (ATM cloud runtime)
> **Read `CHANGELOG.md` first** when resuming work. It is the canonical record of what has been built, tested, and validated. Every major change, validation command, and architectural decision is documented there with its version tag. Use it to reconstruct context without asking the user for background.

---

## 1. Architecture at a Glance

```
┌──────────────────────────────────────────────────────────────┐
│                  Rainy MaTE (Desktop)                        │
│  src/ React 19 UI (views only — zero business logic)         │
│  src-tauri/  Rust engine  ← ALL real work happens here       │
│    ├── ai/              LLM providers, router, agent loop     │
│    │   ├── agent/       ReAct workflow (ThinkStep + ActStep)  │
│    │   ├── providers/   Gemini, OpenAI, Anthropic, etc.       │
│    │   └── router/      IntelligentRouter (model selection)   │
│    ├── commands/        Tauri command surface (invoke layer)  │
│    ├── services/        All domain logic lives here           │
│    │   ├── skill_executor/   Agent tool dispatcher            │
│    │   │   ├── args.rs       Tool JSON schemas (schemars)     │
│    │   │   ├── registry.rs   Tool definitions + policy test   │
│    │   │   ├── filesystem.rs File I/O handlers                │
│    │   │   ├── shell.rs      Shell / git wrappers             │
│    │   │   ├── web.rs        HTTP + search tools              │
│    │   │   └── browser.rs    Native CDP browser automation    │
│    │   ├── memory_vault/     AES-256-GCM encrypted memory DB  │
│    │   ├── memory/           MemoryManager + embedder         │
│    │   ├── wasm_sandbox/     Wasmtime third-party skill host  │
│    │   ├── skill_installer/  skill.toml parser + verifier     │
│    │   ├── airlock.rs        3-tier permission gate           │
│    │   ├── tool_policy.rs    Canonical tool→risk mapping      │
│    │   ├── tool_manifest.rs  Runtime skill manifest builder   │
│    │   ├── neural_service.rs ATM node registration/heartbeat  │
│    │   ├── command_poller.rs Cloud command polling loop       │
│    │   └── atm_client.rs     Rainy ATM REST client            │
│    └── models/               Shared data types (neural, etc.) │
└──────────────────────────────────────────────────────────────┘
              ▲  WebSocket + REST
┌─────────────────────────┐
│  Rainy ATM (rainy-atm/) │  Bun runtime cloud backend
│  Dynamic tool registry  │
│  Workspace lane queue   │
│  Agent executor         │
│  Airlock audit trail    │
└─────────────────────────┘
```

**Iron law**: TypeScript/React is the view layer only. Route all logic, I/O, computation, and security enforcement to Rust. Never put business logic in `.tsx` or `.ts`.

---

## 2. Module Map — Where to Touch What

| Task                        | File(s) to change                                                                                                    |
| --------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| Add a new agent tool        | `skill_executor/args.rs` → `registry.rs` → `filesystem/shell/web/browser.rs` → `tool_policy.rs` → `neural-config.ts` |
| Change LLM provider routing | `ai/router/` + `ai/providers/`                                                                                       |
| Modify agent ReAct loop     | `ai/agent/workflow.rs` (ThinkStep / ActStep)                                                                         |
| Add a Tauri command         | `commands/<domain>.rs` → register in `lib.rs` `invoke_handler!`                                                      |
| Change memory/embedding     | `services/memory/` + `services/memory_vault/`                                                                        |
| Add Wasm skill support      | `services/wasm_sandbox/` + `services/skill_installer/` + `services/third_party_skill_registry.rs`                    |
| Change Airlock policy       | `services/airlock.rs` + `services/tool_policy.rs`                                                                    |
| Add a UI component          | `src/components/<domain>/` — thin wrapper, Tauri `invoke` only                                                       |
| Change neural state bubble  | `src/components/agent-chat/neural-config.ts`                                                                         |
| ATM cloud logic             | `rainy-atm/src/` (Bun runtime)                                                                                       |

---

## 3. Build & Dev Commands

```bash
# Desktop app (full Tauri dev loop)
pnpm run tauri dev

# Vite UI only (no Rust recompile)
pnpm run dev

# Rust checks (run before every commit)
cd src-tauri && cargo check -q
cd src-tauri && cargo test

# TypeScript type check
pnpm exec tsc --noEmit

# Production bundle
pnpm run build

# ATM cloud backend (Bun)
cd rainy-atm && bun run dev
cd rainy-atm && bun test

# Package management — pnpm only, npm is BANNED
pnpm add <package>
pnpm remove <package>
```

> **Always run `cargo check -q` AND `pnpm exec tsc --noEmit` before declaring a change done.** These are the same gates used in every CHANGELOG validation block.

---

## 4. Agent Runtime — ReAct Loop (Rust)

The agent engine lives entirely in `src-tauri/src/ai/agent/`:

| File                | Role                                                                 |
| ------------------- | -------------------------------------------------------------------- |
| `runtime.rs`        | `AgentRuntime` — entry point, context window management, streaming   |
| `workflow.rs`       | `ThinkStep` (LLM call) + `ActStep` (tool execution) + `AgentState`   |
| `context_window.rs` | Sliding token window (≈4 chars/token), evicts oldest non-system msgs |
| `memory.rs`         | `AgentMemory` — semantic retrieval + long-term store calls           |
| `manager.rs`        | `AgentManager` — SQLite persistence of agents + chat history         |
| `error.rs`          | `AgentError` enum with `is_retryable()` classifier                   |

**ReAct loop flow**:

1. `ThinkStep` → prepare messages + inject RAG context from memory vault → call `IntelligentRouter` → emit `StreamChunk` events → if tool calls → go to `ActStep`
2. `ActStep` → resolve tool policy → check Airlock → execute `SkillExecutor` → persist results to memory → return to `ThinkStep`
3. Workflow stops when `ThinkStep` produces no tool calls (max 50 steps, hard cap 200).

**Size limits** (defined as constants in `workflow.rs`):

- `MAX_MODEL_MESSAGE_BYTES` = 95 KB per message
- `MAX_TOOL_TEXT_BYTES` = 48 KB per tool result
- `MAX_MEMORY_CONTEXT_BYTES` = 24 KB injected memory context

---

## 5. Skill Executor — Complete Tool Catalog

All tool definitions are source-of-truth in `services/skill_executor/registry.rs`.  
Every tool MUST have an entry in `services/tool_policy.rs` — the test `every_registered_tool_has_explicit_policy_entry` enforces this.

### Filesystem

| Tool                  | Level | Description                                  |
| --------------------- | ----- | -------------------------------------------- |
| `read_file`           | L0    | Read file contents                           |
| `read_many_files`     | L0    | Read multiple UTF-8 files in one call        |
| `read_file_chunk`     | L0    | Read large file by byte offset               |
| `list_files`          | L0    | List directory contents                      |
| `list_files_detailed` | L0    | List with size / mtime / type metadata       |
| `file_exists`         | L0    | Check file/directory existence               |
| `get_file_info`       | L0    | Get size, timestamps, type                   |
| `search_files`        | L0    | Regex search in file names and contents      |
| `ingest_document`     | L0    | Ingest PDF/MD/TXT into semantic memory vault |
| `mkdir`               | L1    | Create directory                             |
| `write_file`          | L1    | Write file (creates or overwrites)           |
| `append_file`         | L1    | Append to file                               |
| `move_file`           | L2    | Move or rename file/directory                |
| `delete_file`         | L2    | Delete file or directory                     |

### Shell & Git

| Tool              | Level | Description                                                                                      |
| ----------------- | ----- | ------------------------------------------------------------------------------------------------ |
| `git_status`      | L0    | Git status                                                                                       |
| `git_diff`        | L0    | Git diff                                                                                         |
| `git_log`         | L0    | Recent commit history                                                                            |
| `git_show`        | L0    | Show commit/tag/file details                                                                     |
| `git_branch_list` | L0    | List local/remote branches                                                                       |
| `execute_command` | L2    | Run shell command (allowlist: `npm`, `pnpm`, `cargo`, `git`, `bun`, `ls`, `grep`, `echo`, `cat`) |

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
| `screenshot`        | L0    | Screenshot current browser page       |
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

### Memory (via SkillExecutor → MemoryManager)

| Tool              | Level | Description                                      |
| ----------------- | ----- | ------------------------------------------------ |
| `ingest_document` | L0    | Same as filesystem — indexes into semantic vault |

---

## 6. Adding a New Tool — Checklist

1. **Schema** (`skill_executor/args.rs`): Add `#[derive(Deserialize, JsonSchema)]` struct.
2. **Registry** (`skill_executor/registry.rs`): Add `tool("name", "description", schema_for!(Args))` entry.
3. **Handler** (`skill_executor/filesystem.rs` | `shell.rs` | `web.rs` | `browser.rs`): Implement execution logic in the matching domain file. Never grow `skill_executor.rs` orchestrator.
4. **Policy** (`services/tool_policy.rs`): Add `ToolPolicy { skill, airlock_level }` entry. This is enforced by a compile-time regression test.
5. **Neural state** (`src/components/agent-chat/neural-config.ts`): Map the tool to an existing `NeuralState` in `TOOL_STATE_MAP`, or define a new state (add to `NeuralState` type + `getNeuralStateConfig`).
6. **Validate**: `cargo check -q` + `cargo test` + `pnpm exec tsc --noEmit`.

> **Never add a tool without a policy entry.** The test `every_registered_tool_has_explicit_policy_entry` will fail CI.

---

## 7. Airlock Security Model

All tool executions are gated through `services/airlock.rs` before `SkillExecutor` runs them.

| Level  | Name      | Behavior                      | Auto-approved in Headless? |
| ------ | --------- | ----------------------------- | -------------------------- |
| **L0** | Safe      | Auto-approved silently        | Yes                        |
| **L1** | Sensitive | Desktop notification gate     | Yes (headless)             |
| **L2** | Dangerous | Explicit UI approval required | No                         |

**Enforcement is defense-in-depth**:

- Cloud ATM pre-queues by policy before sending commands.
- `workflow.rs ActStep` checks `is_tool_allowed_by_spec()` before routing.
- `SkillExecutor` performs a second deny-first check before execution.
- Policy hash + version prevent replay attacks with stale policies.

**Scope enforcement** (enforced in Rust, not UI):

- `blocked_paths`: filesystem tools check path against this list before any I/O.
- `allowed_domains` / `blocked_domains`: browser/HTTP tools validate URL before request.

---

## 8. Memory System

Memory flows through two layers:

```
User input / web research
        │
        ▼
MemoryManager (services/memory/)
  • short-term: in-process ring buffer
  • long-term: MemoryVaultService (AES-256-GCM encrypted SQLite via libSQL)
        │
        ▼
ThinkStep RAG injection
  • Semantic search via vector_top_k (ANN) or vector_distance_cos (exact fallback)
  • Gemini embedding-001 (3072-dim) — model locked, do not change provider
  • Top 5 hits injected as system context before each ThinkStep LLM call
  • Confidential memories gated by L2 Airlock before injection
```

**DB file**: `{app_data_dir}/memory_db/` (libSQL + SQLite)  
**Encryption**: AES-256-GCM, per-entry nonce, key stored in OS keychain (macOS Keychain Services).  
**Migrations**: `src-tauri/migrations/` — add new `.sql` files with timestamped names.

---

## 9. Wasm Skill Sandbox (QUARANTINE ZONE)

Third-party skills can be installed as Wasm binaries:

```
skill.toml          — manifest (name, version, methods, permissions)
skill.wasm          — Ed25519-signed binary
        │
        ▼
skill_installer/    — parse + verify Ed25519 sig + domain collision check
third_party_skill_registry.rs — persist metadata in app data dir
        │
        ▼
wasm_sandbox/       — Wasmtime host (fuel limit, stack limit, <50MB memory)
  • JSON stdin envelope (method + params)
  • Captured stdout/stderr
  • WASI filesystem: preopened dirs only (read or read_write per manifest)
  • Network: host-mediated prefetch with domain allowlist + SSRF blocking
```

**Key constraints**:

- Third-party methods cannot collide with built-in tool names.
- Sandbox is fail-closed — unknown capabilities are denied until explicit host bindings exist.
- Execution timeout enforced via `spawn_blocking` + bounded timeout.
- Compiled Wasm modules are cached by SHA-256 for repeat-execution performance.

---

## 10. ATM Cloud Runtime (`rainy-atm/`)

Bun-based REST + WebSocket server deployed to Google Cloud Run.

| Component            | File                                   | Role                                              |
| -------------------- | -------------------------------------- | ------------------------------------------------- |
| Agent executor       | `src/services/agent-executor.ts`       | Runs agent sessions                               |
| Tool registry        | `src/services/tool-registry.ts`        | Dynamic tool loading per workspace/agent          |
| Tool validator       | `src/tools/tool-validator.ts`          | Manifest sanitization (name regex, 20-method cap) |
| Unified lane queue   | `src/services/unified-lane-queue.ts`   | Channel-agnostic Telegram/Discord queue           |
| Workspace connectors | `src/services/workspace-connectors.ts` | Per-channel routing + rate limits                 |
| Spec signing         | `src/services/spec-signing.ts`         | HMAC-SHA256 agent spec integrity                  |
| Audit trail          | `src/services/tool-execution-audit.ts` | Immutable tool execution log                      |

**ATM dev commands** (always Bun, never Node):

```bash
cd rainy-atm && bun run dev
cd rainy-atm && bun test
cd rainy-atm && bun run build
cd rainy-atm && bunx tsc --noEmit
```

---

## 11. Neural State UI Mapping

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

## 12. Versioning & Changelog Protocol

For **every significant change** (new features, breaking fixes, architectural changes):

1. Update `CHANGELOG.md` — use the existing format: version tag, named release, Added/Changed/Fixed sections with exact file paths.
2. Bump version in **all three places simultaneously**:
   - `package.json` → `version`
   - `src-tauri/Cargo.toml` → `version`
   - `src-tauri/tauri.conf.json` → `version`
3. Add a **Validation** block listing every command run and its result (pass/fail).

**Version format**: `[MAJOR.MINOR.PATCH] - YYYY-MM-DD - RELEASE-NAME`

> When picking up work mid-session or after a gap, read the last 2–3 CHANGELOG entries to reconstruct the system's current state. This avoids re-implementing already-done work or breaking recently hardened invariants.

---

## 13. Code Rules (Non-Negotiable)

- **Rust for everything real**: file I/O, HTTP, DB, encryption, business logic, security — all Rust.
- **TypeScript for display only**: event handlers, `invoke()` calls, DOM state (modals, tabs, theme).
- **No dead code**: remove unused code immediately. If future use, mark `// @TODO`, `// @deprecated`, or `// @RESERVED`.
- **Modular, not monolithic**: each domain has its own file. Never grow `skill_executor.rs` (orchestrator stays thin). Never grow a single component beyond its purpose.
- **No npm**: `pnpm` always. `npm` is banned.
- **Conventional Commits** with scope: `feat(agent-chat): ...`, `fix(airlock): ...`, `refactor(memory): ...`
- **PR checklist**: summary, rationale, test commands run, linked issues, screenshots for UI changes.
- Keep secrets out of Git — use OS keychain and local env files.

---

## 14. Troubleshooting Reference

| Symptom                        | Investigation path                                                                        |
| ------------------------------ | ----------------------------------------------------------------------------------------- |
| Agent not responding           | Check `src-tauri` terminal logs; verify `rainy-atm` reachable at ATM URL in `lib.rs`      |
| Tool blocked / unexpected deny | Check `AirlockPanel` in UI; inspect `tool_policy.rs` entry for the tool name              |
| Tool missing from model        | Check `registry.rs` has the entry; run `cargo test manifest_covers_every_registered_tool` |
| Policy test failing            | New tool added to registry without `tool_policy.rs` entry — add it                        |
| TypeScript errors              | `pnpm exec tsc --noEmit`; check `src/services/tauri.ts` for missing command wrappers      |
| Memory vault crash             | Likely embedding dimension mismatch — embedder must be `gemini-embedding-001` (3072d)     |
| Ghost nodes after reconnect    | Ensure `CommandPoller.stop()` + `CloudBridge.stop()` called before `clear_credentials()`  |
| Wasm skill timeout             | Check `wasm_sandbox` fuel limit + execution timeout in `wasm_sandbox/mod.rs`              |
| `cargo test` libSQL panic      | Static libsqlite3-sys conflict — see CHANGELOG 0.5.93 for initialization order fix        |

---

## 15. Key File Index (Quick Reference)

```
src-tauri/src/
  lib.rs                           ← App startup + Tauri state wiring
  ai/agent/workflow.rs             ← ThinkStep + ActStep (ReAct engine)
  ai/agent/runtime.rs             ← AgentRuntime, streaming, context window
  services/skill_executor.rs      ← SkillExecutor orchestrator (keep thin)
  services/skill_executor/
    registry.rs                   ← Tool definitions (source of truth)
    args.rs                       ← Tool JSON schemas
    filesystem.rs                 ← File I/O execution
    shell.rs                      ← Shell + git execution
    web.rs                        ← HTTP + search execution
    browser.rs                    ← CDP browser execution
  services/airlock.rs             ← Permission gate (3-tier)
  services/tool_policy.rs         ← Canonical tool→risk mapping
  services/tool_manifest.rs       ← Runtime skill manifest builder
  services/memory_vault/          ← AES-256-GCM encrypted SQLite memory
  services/memory/                ← MemoryManager + embedder
  services/wasm_sandbox/          ← Wasmtime sandbox host
  services/third_party_skill_registry.rs ← Installed skill metadata
  services/neural_service.rs      ← ATM node registration + heartbeat
  services/command_poller.rs      ← Cloud command polling + dispatch
  services/atm_client.rs          ← REST client for Rainy ATM
src/
  components/agent-chat/neural-config.ts  ← Tool → neural state mapping
  services/tauri.ts               ← All Tauri invoke() wrappers
rainy-atm/
  src/services/tool-registry.ts   ← Dynamic tool loading
  src/services/agent-executor.ts  ← Cloud agent session runner
CHANGELOG.md                      ← READ THIS to understand current state
```
