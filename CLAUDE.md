# Rainy MaTE — Agent Reference

**Version**: 0.5.97 · **Stack**: Tauri 2 + Rust · Vite/React 19 · Bun (ATM)
**Canonical truth**: `CHANGELOG.md` — read last 2–3 entries before touching anything.

---

## 1. New Session Protocol

1. **Read CHANGELOG.md** — last 2–3 entries
2. **Check ROADMAP.md** if user mentions planned features
3. **Run `cargo check -q`** from `src-tauri/`
4. **Run `pnpm exec tsc --noEmit`**
5. **State your reading** — summarize version and notable changes

---

## 2. Architecture

- **src/** — React 19 UI (views only, zero business logic)
- **src-tauri/src/** — Rust engine (ALL real work)
  - `ai/agent/` — ReAct workflow (ThinkStep + ActStep)
  - `ai/providers/` — Gemini, OpenAI, Anthropic
  - `ai/router/` — IntelligentRouter (model selection)
  - `commands/` — Tauri invoke surface
  - `services/skill_executor/` — Tool dispatcher + handlers (filesystem, shell, web, browser)
  - `services/memory_vault/` — AES-256-GCM encrypted SQLite
  - `services/airlock.rs` — 3-tier permission gate
  - `services/tool_policy.rs` — Tool→risk mapping
- **rainy-atm/** — Bun runtime (cloud backend)

**Iron law**: TypeScript/React = view layer only. All logic, I/O, security = Rust.

---

## 3. Startup Order (lib.rs)

```
1. AIProviderManager, ProviderRegistry
2. TaskManager
3. FileManager, FileOperationEngine, SettingsManager
4. ManagedResearchService, DocumentService, ImageService
5. WorkspaceManager, IntelligentRouter
6. ATMClient, NeuralService
7. SkillExecutor
8. CommandPoller, SocketClient
--- setup() block ---
9. FolderManager, FileOperationEngine.init()
10. ATM/Neural credentials from keychain (async)
11. MemoryManager + MemoryVault init
12. AirlockService.new(), Database.init(), AgentManager
13. CommandPoller.start(), CloudBridge + SocketClient
```

---

## 4. Module Map

| Task               | Files to change                                                                            |
| ------------------ | ------------------------------------------------------------------------------------------ |
| Add agent tool     | `skill_executor/args.rs` → `registry.rs` → handler → `tool_policy.rs` → `neural-config.ts` |
| Change ReAct loop  | `ai/agent/workflow.rs`                                                                     |
| Change LLM routing | `ai/router/` + `ai/providers/`                                                             |
| Add Tauri command  | `commands/<domain>.rs` → `lib.rs`                                                          |
| Add service        | `services/<name>.rs` → `services/mod.rs` → `lib.rs`                                        |
| DB migration       | `src-tauri/migrations/<timestamp>_<desc>.sql`                                              |
| Airlock policy     | `services/airlock.rs` + `services/tool_policy.rs`                                          |
| UI component       | `src/components/<domain>/` — thin wrapper, `invoke()` only                                 |

---

## 5. Commands

```bash
# Dev
pnpm run tauri dev          # Full Tauri
pnpm run dev                # Frontend only

# Gates (ALWAYS run before commit)
cd src-tauri && cargo check -q
cd src-tauri && cargo test
pnpm exec tsc --noEmit

# Production
pnpm run build

# ATM cloud (Bun only)
cd rainy-atm && bun run dev
cd rainy-atm && bun test

# Package manager
pnpm add <pkg>; pnpm remove <pkg>  # npm BANNED
```

---

## 6. Agent Runtime (ReAct Loop)

**Files**: `ai/agent/runtime.rs`, `workflow.rs`, `context_window.rs`, `memory.rs`, `manager.rs`

**Flow**: ThinkStep → RAG injection → IntelligentRouter → ActStep (tool execution) → memory persist → repeat

- Hard caps: 50 steps default, 200 max
- Size limits: MAX_MODEL_MESSAGE_BYTES=95KB, MAX_TOOL_TEXT_BYTES=48KB, MAX_MEMORY_CONTEXT_BYTES=24KB

---

## 7. Adding a New Tool

1. **Schema**: `skill_executor/args.rs` — add `#[derive(Deserialize, JsonSchema)]` struct
2. **Registry**: `skill_executor/registry.rs` — add `tool("name", "desc", schema_for!(Args))`
3. **Handler**: `skill_executor/filesystem.rs|shell.rs|web.rs|browser.rs`
4. **Policy**: `services/tool_policy.rs` — add `ToolPolicy { skill, airlock_level }`
5. **Neural state**: `src/components/agent-chat/neural-config.ts`
6. **Validate**: `cargo check -q && cargo test && pnpm exec tsc --noEmit`

---

## 8. Tool Catalog

### Filesystem (L0-L2)

| Tool                                                                                                                                                      | Level | Description      |
| --------------------------------------------------------------------------------------------------------------------------------------------------------- | ----- | ---------------- |
| `read_file`, `read_many_files`, `read_file_chunk`, `list_files`, `list_files_detailed`, `file_exists`, `get_file_info`, `search_files`, `ingest_document` | L0    | Read/list/search |
| `mkdir`, `write_file`, `append_file`                                                                                                                      | L1    | Write            |
| `move_file`, `delete_file`                                                                                                                                | L2    | Move/delete      |

### Shell & Git

| Tool                                                               | Level | Description                                                        |
| ------------------------------------------------------------------ | ----- | ------------------------------------------------------------------ |
| `git_status`, `git_diff`, `git_log`, `git_show`, `git_branch_list` | L0    | Git operations                                                     |
| `execute_command`                                                  | L2    | Shell (whitelist: npm, pnpm, cargo, git, bun, ls, grep, echo, cat) |

### Web & HTTP

| Tool                                               | Level | Description   |
| -------------------------------------------------- | ----- | ------------- |
| `web_search`, `read_web_page`                      | L0    | Search/scrape |
| `http_get_json`, `http_get_text`, `http_post_json` | L1    | HTTP requests |

### Browser (CDP)

| Tool                                                                                                      | Level | Description |
| --------------------------------------------------------------------------------------------------------- | ----- | ----------- |
| `screenshot`, `get_page_content`, `get_page_snapshot`, `extract_links`                                    | L0    | Read page   |
| `browse_url`, `open_new_tab`, `click_element`, `wait_for_selector`, `type_text`, `submit_form`, `go_back` | L1    | Interactive |

---

## 9. Airlock Security

| Level | Name      | Behavior                      | Auto-approved? |
| ----- | --------- | ----------------------------- | -------------- |
| L0    | Safe      | Auto-approved silently        | Yes            |
| L1    | Sensitive | Desktop notification gate     | Yes (headless) |
| L2    | Dangerous | Explicit UI approval required | No             |

**Defense-in-depth**: Cloud ATM pre-queue → workflow.rs check → SkillExecutor check → policy hash

---

## 10. Memory System

- **Short-term**: in-process ring buffer (100 entries)
- **Long-term**: MemoryVaultService (AES-256-GCM encrypted libSQL/SQLite)
- **Embedder**: `gemini-embedding-001` (3072d) — **LOCKED, never change**
- **RAG**: vector_top_k with top 5 hits injected (24KB budget)

---

## 11. Wasm Sandbox

- Third-party skills run in Wasmtime host
- Constraints: <50MB memory, fuel limit + timeout, WASI filesystem, domain allowlist
- Ed25519-signed binaries, no method name collisions with built-in tools

---

## 12. ATM Cloud (rainy-atm/)

Bun-based REST + WebSocket (Google Cloud Run)

| Component                 | Role                   |
| ------------------------- | ---------------------- |
| `agent-executor.ts`       | Cloud agent sessions   |
| `tool-registry.ts`        | Dynamic tool loading   |
| `tool-validator.ts`       | Manifest sanitization  |
| `unified-lane-queue.ts`   | Telegram/Discord queue |
| `spec-signing.ts`         | HMAC-SHA256 integrity  |
| `tool-execution-audit.ts` | Execution log          |

---

## 13. Neural State UI

| State           | Color   | Triggers                                          |
| --------------- | ------- | ------------------------------------------------- |
| `thinking`      | Purple  | LLM generation                                    |
| `planning`      | Amber   | planning tool                                     |
| `reading`       | Blue    | web_search, read_web_page                         |
| `observing`     | Emerald | read_file, list_files, file_exists, get_file_info |
| `browsing`      | Orange  | browse_url, click_element, type_text, screenshot  |
| `communicating` | Indigo  | http_get_json, http_post_json                     |
| `creating`      | Pink    | write_file, append_file, mkdir                    |
| `pruning`       | Red     | delete_file                                       |
| `executing`     | Cyan    | execute*command, git*\*, move_file                |

---

## 14. Key File Index

```
src-tauri/src/
  lib.rs                    ← App startup + Tauri state
  ai/agent/workflow.rs      ← ThinkStep + ActStep
  ai/agent/runtime.rs      ← AgentRuntime, streaming
  ai/router/                ← Model selection
  services/skill_executor/registry.rs  ← Tool definitions
  services/skill_executor/filesystem.rs ← File I/O
  services/airlock.rs       ← 3-tier permission
  services/tool_policy.rs   ← Tool→risk mapping
  services/memory_vault/    ← Encrypted memory
  commands/                 ← Tauri invoke handlers
src/components/agent-chat/neural-config.ts  ← Tool→state
CHANGELOG.md               ← Primary truth
ROADMAP.md                ← Planned features
```

---

## 15. Code Rules

### Core

- **Rust for everything real**: file I/O, HTTP, DB, encryption, security
- **TypeScript for display only**: event handlers, `invoke()`, DOM state
- **pnpm only** — npm banned
- **Conventional Commits**: `feat(agent-chat):`, `fix(airlock):`
- **Async**: tokio for I/O, rayon for CPU tasks

### Modularization (Mandatory)

- Single Responsibility: each module one clear responsibility
- High Cohesion, Low Coupling
- No Circular Dependencies
- <400 lines per module
- Tests at boundaries

### Dead Code

- Remove unused code immediately
- Mark for future: `// @TODO reason`, `// @deprecated reason`, `// @RESERVED reason`

---

## 16. Anti-Patterns

```rust
// ❌ WRONG — business logic in command
#[command]
pub async fn process_users(users: Vec<User>) -> Result<Vec<User>, String> {
    let filtered = users.into_iter().filter(|u| u.active).collect();
    Ok(sorted)
}

// ✅ RIGHT — delegate to service
#[command]
pub async fn process_users(users: Vec<User>, svc: State<'_, UserService>) -> Result<Vec<User>, String> {
    svc.process(users).await.map_err(|e| e.to_string())
}
```

```typescript
// ❌ WRONG — logic in TS
const filtered = users
  .filter((u) => u.active)
  .sort((a, b) => b.score - a.score);
await invoke("save_users", { users: filtered });

// ✅ RIGHT — delegate to Rust
const result = await invoke<ProcessedUsers>("process_and_save_users", {
  users,
});
```

---

## 17. Troubleshooting

| Symptom              | Fix                                                                       |
| -------------------- | ------------------------------------------------------------------------- |
| Agent not responding | Check src-tauri logs; verify ATM URL in lib.rs                            |
| Tool blocked         | Check AirlockPanel; inspect tool_policy.rs                                |
| Tool missing         | Check registry.rs; run `cargo test manifest_covers_every_registered_tool` |
| Policy test failing  | Missing tool_policy.rs entry                                              |
| TypeScript errors    | `pnpm exec tsc --noEmit`; check tauri.ts wrappers                         |
| Memory vault crash   | Embedding dimension must be 3072d                                         |

---

## 18. Test Commands

```bash
# Agent
cd src-tauri && cargo test -q agent --lib

# Tool policy
cd src-tauri && cargo test -q manifest_covers_every_registered_tool --lib
cd src-tauri && cargo test -q every_registered_tool_has_explicit_policy_entry --lib

# Airlock
cd src-tauri && cargo test -q airlock --lib

# Memory vault
cd src-tauri && cargo test -q memory_vault --lib

# Wasm sandbox
cd src-tauri && cargo test -q wasm_sandbox::tests --lib

# ATM
cd rainy-atm && bun test
```

---

## 19. Versioning

### Ask First (MANDATORY)

- Update CHANGELOG.md
- Bump version (package.json, Cargo.toml, tauri.conf.json)
- Any breaking change

### SemVer

| Change      | Bump                  |
| ----------- | --------------------- |
| Bug fix     | PATCH                 |
| New feature | MINOR                 |
| Breaking    | MAJOR (requires plan) |

### Changelog Format

```
## [X.Y.Z] - YYYY-MM-DD - RELEASE-NAME
### Added / Changed / Fixed
- Description
### Validation
- cargo check -q → pass
- pnpm exec tsc --noEmit → pass
```

---

## 20. Design Rules

### Component Library

- **HeroUI v3** — default
- **shadcn/ui** — ask user first

### Background Opacity

- Overlay/Modal: light `bg-background/85`, dark `bg-background/20`
- Cards: light `bg-background/80`, dark `bg-background/30`
- Navigation: light `bg-background/90`, dark `bg-background/10`

### Backdrop Blur

- Default: `backdrop-blur-md`
- Use with semi-transparent bg + optional border

### Accessibility

- WCAG 2.1 AA contrast
- Test readability against blur
- Consider motion sensitivity

---

## 21. gstack — Browser & Skills

Use the `/browse` skill from gstack for all web browsing tasks. **Never use `mcp__claude-in-chrome__*` tools.**

### Available Skills

| Skill | Description |
|---|---|
| `/office-hours` | Office hours assistant |
| `/plan-ceo-review` | CEO/founder-mode plan review |
| `/plan-eng-review` | Eng manager-mode plan review |
| `/plan-design-review` | Designer's eye plan review |
| `/design-consultation` | Full design system consultation |
| `/review` | Pre-landing PR review |
| `/ship` | Ship workflow (merge, test, PR) |
| `/browse` | Headless browser for QA and dogfooding |
| `/qa` | Systematic QA test + fix loop |
| `/qa-only` | Report-only QA testing |
| `/design-review` | Visual QA with fixes |
| `/setup-browser-cookies` | Import real browser cookies |
| `/retro` | Weekly engineering retrospective |
| `/debug` | Systematic debugging assistant |
| `/document-release` | Post-ship documentation update |

---

**End of CLAUDE.md**

## Skill routing

When the user's request matches an available skill, ALWAYS invoke it using the Skill
tool as your FIRST action. Do NOT answer directly, do NOT use other tools first.
The skill has specialized workflows that produce better results than ad-hoc answers.

Key routing rules:
- Product ideas, "is this worth building", brainstorming → invoke office-hours
- Bugs, errors, "why is this broken", 500 errors → invoke investigate
- Ship, deploy, push, create PR → invoke ship
- QA, test the site, find bugs → invoke qa
- Code review, check my diff → invoke review
- Update docs after shipping → invoke document-release
- Weekly retro → invoke retro
- Design system, brand → invoke design-consultation
- Visual audit, design polish → invoke design-review
- Architecture review → invoke plan-eng-review
