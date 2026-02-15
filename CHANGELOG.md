# Changelog

All notable changes to Rainy Cowork will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.5.22] - 2026-02-15 - Airlock Hardening + Tooling Expansion

### Added - Agent Tools (Desktop Runtime)

**Rust Backend (`src-tauri/src/services/`)**

- `skill_executor.rs`:
  - Added filesystem utility tools:
    - `file_exists`
    - `get_file_info`
    - `read_file_chunk`
    - `read_many_files`
  - Added browser automation tools:
    - `extract_links`
    - `wait_for_selector`
    - `type_text`
    - `open_new_tab`
    - `go_back`
    - `submit_form`
    - `get_page_snapshot`
  - Added web/API tools:
    - `http_get_json`
    - `http_post_json`
  - Added shell wrappers for developer workflows:
    - `git_status`
    - `git_diff`
    - `git_log`
  - Hardened command output handling by truncating oversized shell outputs before returning to models.

### Changed - Skill Executor Modularization

**Rust Backend (`src-tauri/src/services/skill_executor/`)**

- Refactored monolithic `skill_executor.rs` into modular structure:
  - `args.rs` (tool argument schemas)
  - `registry.rs` (tool definitions exposed to model providers)
  - `filesystem.rs` (filesystem path resolution + handlers)
  - `shell.rs` (shell and git wrappers)
  - `web.rs` (research + HTTP JSON tools)
  - `browser.rs` (browser automation tools)
- Kept `skill_executor.rs` as a thin orchestrator for routing, policy checks, and shared scope guards.
- Established contributor-ready pattern for adding new tools without expanding a single monolithic file.

**Browser Controller (`src-tauri/src/services/browser_controller.rs`)**

- Added stable primitives used by tools:
  - `wait_for_selector`
  - `type_text` (with optional clear + input/change event dispatch)

### Changed - Tool Policy + Registration

**Frontend + Runtime Policy Maps**

- `src-tauri/src/services/tool_policy.rs`:
  - Updated canonical tool risk mapping for all newly added tools.
- `src/constants/defaultNeuralSkills.ts`:
  - Registered all new methods so node registration exposes them to Cloud Cortex, including:
    - `read_many_files`
    - `git_log`
    - `get_page_snapshot`
- `src/constants/toolPolicy.ts`:
  - Added Airlock policy mappings for:
    - `read_many_files` (Safe)
    - `git_log` (Safe)
    - `get_page_snapshot` (Safe)

### Changed - Airlock UI Modularization

**Frontend (`src/components/agents/builder/`)**

- `AirlockPanel.tsx` refactored into smaller modules:
  - `airlock/PolicySection.tsx`
  - `airlock/ScopesSection.tsx`
  - `airlock/RateLimitsSection.tsx`
  - `airlock/constants.ts`
  - `airlock/utils.ts`
- Reduced panel complexity and improved render behavior using memoization (`useMemo`, `useCallback`, `React.memo`) while preserving functionality.

### Fixed - Real Airlock Scope Enforcement (Rust)

**Rust Runtime (`src-tauri/src/`)**

- `models/neural.rs`:
  - Extended `RainyPayload` with scope fields:
    - `blocked_paths`
    - `allowed_domains`
    - `blocked_domains`
- `ai/agent/workflow.rs`:
  - Injects Airlock scope data from `AgentSpec` into each queued tool command payload.
  - Expanded filesystem tool guard list to include recently added filesystem methods.
- `commands/skills.rs`:
  - Updated local `execute_skill` pseudo-command payload to include new scope fields.
- `services/skill_executor.rs`:
  - Enforces `blocked_paths` in filesystem/shell path resolution.
  - Enforces `allowed_domains` / `blocked_domains` for browser/web URL-based operations.
  - Applies domain checks to:
    - `browse_url` / `open_new_tab`
    - `read_web_page`
    - `http_get_json`
    - `http_post_json`
- `services/airlock.rs`:
  - Added effective risk-level resolution that escalates to canonical policy level when a command declares a lower `airlock_level`.
  - Approval requests now use effective level (prevents downscoping bypass attempts).

### Fixed - `search_files` Reliability + Model Alignment

**Rust Backend (`src-tauri/src/services/skill_executor/`)**

- `args.rs`:
  - Extended `SearchFilesArgs` with:
    - `case_sensitive`
    - `max_files`
  - Clarified default behavior docs for `search_content`.
- `filesystem.rs`:
  - Changed `search_files` default to search text content when `search_content` is omitted.
  - Added case-insensitive search by default via regex builder (unless `case_sensitive: true`).
  - Increased and parameterized scan cap (`max_files`) for larger workspaces.
  - Expanded text-like file coverage for content search (including extensionless files like `Dockerfile`, `Makefile`, and `.env*`).

**Tool Definitions (`src-tauri/src/services/skill_executor/registry.rs`)**

- Updated `search_files` description to explicitly reflect name + content regex behavior.

**Frontend Skill Manifest (`src/constants/defaultNeuralSkills.ts`)**

- Updated `search_files` method metadata:
  - Clearer description for content search behavior.
  - Added exposed parameters:
    - `case_sensitive`
    - `max_files`
  - Clarified `search_content` default in parameter description.

### Changed - AI Feature Cleanup

**Rust Runtime (`src-tauri/src/ai/features/`)**

- Removed unused `security_service` module to reduce dead code and production maintenance surface.
- Removed `pub mod security_service;` from `ai/features/mod.rs`.
- Removed unused `ed25519-dalek` dependency from `src-tauri/Cargo.toml`.

### Validation

- `pnpm run build` — passes
- `cd src-tauri && cargo check -q` — passes
- `pnpm exec tsc --noEmit` — passes
- `cd src-tauri && cargo test -q skill_executor::tests::` — passes
- `cd src-tauri && cargo test services::tool_policy::tests::maps_core_tools -- --nocapture` — passes
- `cd src-tauri && cargo test services::skill_executor::tests::shell_allowlist_matches_agents_policy -- --nocapture` — passes
- `cd src-tauri && cargo test services::skill_executor::tests::domain_scope_enforces_blocked_before_allowed -- --nocapture` — passes
- `cd src-tauri && cargo test services::airlock::tests::pending_approvals_are_sorted_by_timestamp -- --nocapture` — passes
- `cd src-tauri && cargo test services::airlock::tests::effective_airlock_level_escalates_when_declared_is_lower_than_policy -- --nocapture` — passes

## [0.5.21] - 2026-02-13 - Update Check Button

### Added

- **Check for Updates Button**: Added a manual update check button to the AppSidebar footer.
  - Uses `@tauri-apps/plugin-updater` `check()` API
  - Full state feedback: idle → checking (spinner) → up-to-date / available / error
  - When update is available, button becomes "Update vX.X.X" and triggers `downloadAndInstall()` + `relaunch()`
  - Auto-resets status messages after 3 seconds
  - Fully responsive: icon-only with tooltip when sidebar is collapsed

## [0.5.20] - 2026-02-12 - Welcome Rainy MaTE

### Changed - Visual Identity & Stability

- **Logo Reform**: Implemented a new "whale" logo design across the application (Sidebar, App Icon).
- **Design Polish**: Refined the sidebar and layout for a cleaner, "perfect", and more stable aesthetic.
- **Branding**: Consolidated visual identity under "Rainy MaTE".

## [0.5.19] - 2026-02-11 - Emergency: Disconnect/Reconnect State Fix

### Fixed - Production-Critical State Management

**Rust Backend (`src-tauri/src/`)**

- `services/atm_client.rs`:
  - `ATMClientState` now stores `platform_key` for node linkage
  - `load_credentials_from_keychain` now also loads `neural_platform_key` from Keychain
  - `create_agent` and `list_agents` now send `x-rainy-platform-key` header
  - `clear_credentials` now clears `platform_key` (previously leaked stale state)

- `services/cloud_bridge.rs`:
  - Added `is_stopped` flag and `stop()` method for graceful shutdown
  - `run_loop` now checks stop flag and exits cleanly
  - Added `restart()` method (reserved for future reconnect flow)

- `services/command_poller.rs`:
  - Activated `stop()` method (removed `#[allow(dead_code)]`)

- `commands/atm.rs`:
  - `reset_neural_workspace` now stops `CommandPoller` and `CloudBridge` **before** clearing credentials
  - Prevents auto-re-registration race conditions that created ghost nodes
  - Uses `try_state` for `CloudBridge` (graceful if not yet initialized)

**Frontend (`src/`)**

- `components/neural/NeuralPanel.tsx`:
  - `handleLogout` now calls `clearNeuralCredentials()` explicitly for defense-in-depth

### Root Cause

Each disconnect/reconnect cycle left orphaned state across 4 subsystems:

- `CommandPoller` never stopped → auto-re-registered ghost nodes
- `CloudBridge` had no stop mechanism → ran forever with stale credentials
- `ATMClient.clear_credentials()` didn't clear `platform_key`
- No coordinated shutdown during logout

### Validation

- `cargo check` — 0 errors, 0 warnings

## [0.5.18] - 2026-02-11 - Agent Deploy Upsert (No Duplicate Edit Deploys)

### Fixed - Agent Store Edit/Deploy Duplication

**Cloud Runtime (`Rainy ATM`)**

- `routes/admin.ts`:
  - `POST /admin/agents` now performs an upsert for `v2_spec`/`v3_spec` agents using logical spec id (`config.id`)
  - If an existing agent with the same logical spec id exists in the workspace, the endpoint updates that row instead of inserting a new one
  - Returns `action: "updated"` for upsert updates and `action: "created"` for true inserts
  - Prevents silent duplicate agents when users edit and redeploy from Agent Store/Builder

### Fixed - Airlock Builder UX for Allowlist + Approval Levels

**Frontend (`src/components/agents/builder/`)**

- `AirlockPanel.tsx`:
  - Reworked tool permission UI to make `allowlist` mode explicitly manual and usable
  - Added per-tool controls with clear mode behavior:
    - `allowlist`: explicit `Allow this tool` selector
    - `all`: explicit `Block this tool (deny)` selector
  - Added risk-level guidance tied to approval flow:
    - L0: auto-approved (no modal)
    - L1: approval modal / notification gate
    - L2: explicit approval modal required
  - Added quick allowlist presets (`L0 only`, `L0+L1`, `all`, `clear`) and custom-tool add flow
  - Improved visibility of active tool count and per-level active totals

**Shared Constants (`src/constants/`)**

- `toolPolicy.ts`:
  - Added `KNOWN_TOOL_NAMES` export so Airlock UI always shows a stable tool catalog, even when `tool_levels` starts empty

### Fixed - Dynamic Skill Tool Name Normalization

**Cloud Runtime (`Rainy ATM`)**

- `tool-policy.ts`:
  - Added normalization for namespaced dynamic tool names (`skill__method` -> `method`) before skill/risk mapping
- `tool-executor.ts`:
  - Added parsing of namespaced tool calls so `skill` and `method` are resolved correctly before sending commands to desktop
  - Prevents false `unknown method` failures when model invokes dynamic tools
- `tool-policy.test.ts`:
  - Added test coverage for namespaced tool normalization path

### Fixed - Runtime Enforcement of Agent Airlock Tool Policy/Levels

**Desktop Runtime (`src-tauri/src/ai/agent/`)**

- `workflow.rs`:
  - Tool advertisement now filters by `spec.airlock.tool_policy` before presenting tools to the model
  - Tool execution now blocks any tool denied by `spec.airlock.tool_policy` even if the model tries to call it
  - Airlock command level is now resolved from `spec.airlock.tool_levels` when configured (fallback to canonical defaults otherwise)
  - Aligns Builder Airlock configuration with actual runtime behavior (Think + Act stages)

### Validation

- `cd rainy-atm && bunx tsc --noEmit` — No TypeScript errors
- `cd rainy-atm && bun test src/services/agent-runtime-config.test.ts` — 4 tests pass
- `cd rainy-atm && bun test` — 33 tests pass
- `pnpm exec tsc --noEmit` — No TypeScript errors
- `pnpm run build` — Production build passes
- `cd src-tauri && cargo test tool_policy_is_deny_first -- --nocapture` — pass
- `cd src-tauri && cargo test tool_policy_disabled_blocks_all -- --nocapture` — pass

## [0.5.17] - 2026-02-11 - Spec Builder v2

### Added - Spec Builder v2

**Desktop Runtime (`src-tauri/src/`)**

- `src/lib.rs` — Registered `spec_builder` module with `create_default_spec` and `normalize_spec` functions

### Changed

- `routes/admin.ts`:
  - `POST /admin/agents` and `PATCH /admin/agents/:id` now use `normalizeAgentSpec()` to sanitize and validate incoming specs
  - Added comprehensive type-safe validation for all spec fields
  - Improved error messages for invalid spec fields

### Validation

- `cd rainy-atm && bunx tsc --noEmit` — No TypeScript errors
- `cd src-tauri && cargo test spec_builder` — 2 tests pass

## [0.5.16] - 2026-02-11 - Security Hardening (Phase 5)

### Added - Spec Integrity + Tool Audit Trail

**Cloud Runtime (`Rainy ATM`)**

- `services/spec-signing.ts` — **NEW** canonical HMAC-SHA256 signing/verification helpers:
  - Agent spec signature creation + verification
  - Skill manifest signature verification
- `services/tool-execution-audit.ts` — **NEW** immutable tool execution audit writer
- `db/schema.ts`:
  - Added `tool_execution_audit` table
  - Added `idx_tool_execution_audit_workspace` index

**Desktop Runtime (`src-tauri/src/`)**

- `services/manifest_signing.rs` — **NEW** HMAC-SHA256 signing for skill manifests:
  - Canonical JSON serialization (recursive key-sort matching ATM `stableSortValue`)
  - `sign_skills_manifest()` produces hex digest compatible with ATM `verifySkillsManifestSignature()`

### Changed

- `routes/admin.ts`:
  - `POST /admin/agents` and `PATCH /admin/agents/:id` now verify incoming HMAC signatures (when present) and re-sign persisted v2/v3 specs
  - Added `GET /admin/tools/audit` to inspect execution audit records
- `routes/nodes.ts`:
  - Added tool manifest signature verification hook via `x-skills-signature` header
  - Heartbeat command fetch now includes `status IN ('pending', 'approved')`
  - Airlock-denied command completions now map to `rejected` status
- `services/command-bridge.ts`:
  - Initial queue status now reflects airlock intent (`approved` for level 0, `pending` for level 1/2)
- `services/tool-executor.ts`:
  - Logs every tool execution attempt with outcome (`success|error|blocked`) and latency
- `services/agent-executor.ts`:
  - Passes session/agent context into tool audit logging
- `services/neural_service.rs`:
  - Sends `x-skills-signature` HMAC header on node registration

### Validation

- `cd rainy-atm && bunx tsc --noEmit` — No TypeScript errors
- `cd src-tauri && cargo test manifest_signing` — 5 tests pass

## [0.5.15] - 2026-02-11 - Unified Connectors (Phase 4)

### Added - Workspace-Level Connectors + Unified Lane Queue

**Cloud Runtime (`Rainy ATM`)**

- `services/workspace-connectors.ts` — **NEW** workspace-level connector configuration:
  - Parses connector channels + agent routing from workspace config
  - Resolves per-channel auto-reply behavior
  - Resolves per-channel rate limits
  - Resolves routed agent by channel + priority

- `services/unified-lane-queue.ts` — **NEW** channel-agnostic queue orchestrator:
  - `enqueue(channel, job)` wrapper for Telegram/Discord queues
  - Workspace-aware per-channel rate limiting (`rainy:ratelimit:{channel}:{workspace}`)
  - Shared interface for queueing and draining lanes

### Changed

- `routes/webhooks.ts`:
  - Replaced direct `telegramLaneQueue`/`discordLaneQueue` calls with `unifiedLaneQueue`
  - Added workspace connector auto-reply gating before queueing
  - Added agent→channel routing application (auto-select routed agent for the channel)

- `routes/workers.ts`:
  - Replaced channel-specific processing branches with `unifiedLaneQueue.processSession()`
  - Unified pending-count handling and worker re-dispatch logic

### Validation

- `cd rainy-atm && bun run build`
- `cd rainy-atm && bun test`

All passed (28 tests, 0 failures).

## [0.5.14] - 2026-02-11 - ATM Dynamic Tools (Phase 3)

### Added - Dynamic Tool Registry & Manifest Validation

**Cloud Runtime (`Rainy ATM`)**

- `tools/tool-validator.ts` — **NEW** Security validation for tool manifests:
  - Name sanitization (enforces `[a-z][a-z0-9_]*` pattern)
  - Method count limits (max 20 per skill — DoS prevention)
  - Description length limits (max 500 chars — injection prevention)
  - Parameter schema validation (type, count, descriptions)
  - Airlock level validation (0/1/2 only)
  - Duplicate method name detection

- `services/tool-registry.ts` — **NEW** Dynamic tool loading:
  - `ToolRegistry.getToolsForAgent(workspaceId, agentId)` replaces hardcoded tool loading
  - Combines base tools (WORKSPACE_TOOLS) + desktop node skills (online nodes from DB)
  - Merges workspace + agent airlock policies with deny-first precedence
  - Node skill methods namespaced as `skillName__methodName` to prevent collisions
  - Filters combined tool set via `filterAllowedTools()`

### Changed

- `services/agent-executor.ts`:
  - Replaced hardcoded `getWorkspaceToolsForRun()` with `ToolRegistry.getToolsForAgent()`
  - Updated legacy OpenAI fallback to accept dynamic tools array instead of boolean flag
  - Removed unused imports (`WORKSPACE_TOOLS`, `filterAllowedTools`, `normalizeToolAccessPolicy`, `db`)

- `routes/nodes.ts`:
  - Added `validateToolManifest()` on `/register` and `/:nodeId/heartbeat` endpoints
  - Invalid skill manifests filtered out with console warnings

### Validation

- `tool-validator.test.ts` — 12 tests (name sanitization, injection prevention, limits, duplicates)
- `tool-registry.test.ts` — 7 tests (skill→tool conversion, policy merging, deny-first)
- All 28 tests pass across 5 test files, 0 failures

---

## [0.5.13] - 2026-02-10 - Auto-Update System (Beta 1 Production)

### Added - Mandatory Auto-Update System

**Rust Backend (`src-tauri/`)**

- `Cargo.toml` — Added `tauri-plugin-updater` v2 and `tauri-plugin-process` v2 dependencies
- `src/lib.rs` — Registered updater plugin (desktop-only via `#[cfg(desktop)]`) and process plugin
- `tauri.conf.json` — Added `createUpdaterArtifacts: true` and `plugins.updater` config with GitHub Release endpoint
- `capabilities/default.json` — Added `updater:default`, `process:default`, `process:allow-restart` permissions

**Frontend (`src/`)**

- `components/updater/UpdateChecker.tsx` — **NEW** Mandatory update checker:
  - Non-dismissable overlay (no skip/close buttons)
  - Shows current vs new version and release notes
  - Download progress bar with percentage
  - Auto-relaunch after installation
  - Retry on error
  - Dark theme with premium glassmorphism design
- `App.tsx` — Mounted `UpdateChecker` at root level

**CI/CD (`.github/workflows/`)**

- `publish.yml` — **NEW** Release workflow:
  - Triggers on push to `release` branch or `workflow_dispatch`
  - Builds macOS ARM64 (M1+) and x86_64 (Intel) targets
  - Uses `tauri-apps/tauri-action@v0` for automated GitHub Releases
  - Generates `latest.json` for updater endpoint
  - Signs updater artifacts via `TAURI_SIGNING_PRIVATE_KEY` secret
  - Ad-hoc signing for Beta 1 (Apple code signing deferred to Beta 2)
  - Linux and Windows targets commented out for future release

**Dependencies**

- `@tauri-apps/plugin-updater` v2.10.0 (frontend)
- `@tauri-apps/plugin-process` v2.3.1 (frontend)

### Notes

- All updates are mandatory — users cannot skip or dismiss the update overlay
- Before first release: generate signing keys with `pnpm tauri signer generate`
- GitHub Secrets required: `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
- Apple Developer code signing + notarization will be added in Beta 2

---

## [0.5.11] - 2026-02-10 - Tool Policy Parity, Integrity & Replay Protection (P2)

### Added - Policy Parity Across Cloud + Desktop + UI

**Desktop Runtime (`src-tauri/src/`)**

- Added canonical tool policy mapping service:
  - `services/tool_policy.rs` (`tool -> skill + airlock level`)
- Refactored agent workflow to consume canonical mapping:
  - `ai/agent/workflow.rs`
- Added second-gate execution enforcement in `SkillExecutor`:
  - deny-first tool authorization checks before execution
  - policy hash verification (SHA-256)
  - stale policy version rejection

**Desktop Settings Persistence (`src-tauri/src/services/settings.rs`)**

- Added persisted per-workspace policy version floor:
  - survives app restarts
  - used for anti-replay stale policy rejection

**Desktop ATM Bridge (`src-tauri/src/services/atm_client.rs`, `commands/atm.rs`, `lib.rs`)**

- Added typed support for tool policy state endpoints:
  - policy payload
  - policy version
  - policy hash

**Frontend (`src/`)**

- Added canonical UI-side policy mapping:
  - `constants/toolPolicy.ts`
- Refactored `DEFAULT_NEURAL_SKILLS` to resolve Airlock levels from canonical mapping:
  - `constants/defaultNeuralSkills.ts`
- Extended Neural Link policy UI:
  - owner-auth tool policy editor (enabled/mode/allow/deny)
  - policy metadata visibility (version + hash)
  - wired via `services/tauri.ts` tool policy state wrappers

### Changed

- Tool policy enforcement is now defense-in-depth:
  - pre-queue enforcement in cloud runtime
  - pre-execution enforcement in desktop runtime
- Policy integrity metadata is now propagated end-to-end and surfaced in UI.

### Validation

- `pnpm exec tsc --noEmit`
- `cd rainy-atm && bun test`
- `cd src-tauri && cargo check`
- `cd src-tauri && cargo test skill_executor::policy_tests -- --nocapture`

All passed with no new Rust warnings in these runs.

## [0.5.10] - 2026-02-10 - Streaming Runtime & Airlock Enforcement (P1)

### Added - Streaming Responses & Security Classification

**Rust Backend (`src-tauri/src/ai/agent/`)**

- `runtime.rs` — Added `AgentEvent::StreamChunk(String)` variant for token-by-token streaming to frontend
- `workflow.rs` — **ThinkStep** hybrid streaming:
  - When **no tools** present: uses `complete_stream()` for real-time token delivery via `StreamChunk` events
  - When **tools** present: uses blocking `complete()` (tool calls require full response)
  - Accumulates streamed content with `std::sync::Mutex` for safe sync callback access
  - Emits full `Thought` event after streaming completes for history consistency
- `workflow.rs` — **ActStep** Airlock level enforcement:
  - `classify_tool_airlock_level()` — 3-tier dynamic security classification:
    - **Safe** (Level 0): `read_file`, `list_files`, `search_files`, `screenshot`, `web_search`, `read_web_page`, `get_page_content`
    - **Sensitive** (Level 1): `write_file`, `append_file`, `browse_url`, `click_element`
    - **Dangerous** (Level 2): `execute_command`, `delete_file`, `move_file`
    - Unknown tools default to **Sensitive** (conservative)
  - Replaced hardcoded `AirlockLevel::Safe` with dynamic classification

**Services (`src-tauri/src/services/`)**

- `command_poller.rs` — Added `AgentEvent::StreamChunk` handler in `map_agent_event`; passes raw content without truncation

### Technical

- Build: 0 warnings, 0 errors
- Tests: 168 passed, 4 pre-existing failures (keychain, base_agent legacy)
- Architecture: Hybrid streaming preserves tool call detection while enabling real-time text delivery

---

## [0.5.9] - 2026-02-10 - Agent Memory & Runtime Hardening (P0)

### Added - Agent Memory Persistence & Context Management

**Rust Backend (`src-tauri/src/ai/agent/`)**

- `context_window.rs` - **NEW** — Sliding window context manager:
  - Token estimation engine (1 token ≈ 4 chars) with tool call overhead
  - `trim_history()` — evicts oldest non-system messages to stay within budget
  - System messages always preserved
  - Configurable via `AgentSpec.memory_config.max_tokens`
  - 3 unit tests covering trim behavior, system message preservation, and eviction order

- `error.rs` - **NEW** — Unified `AgentError` enum:
  - Variants: Memory, Provider, Tool, Workflow, Timeout, AirlockDenied, Serialization
  - `From` impls for `sqlx::Error`, `serde_json::Error`, `reqwest::Error`
  - `is_retryable()` classifier for retry logic
  - Converts to `String` for backwards compatibility with existing workflow methods

- `runtime.rs` - **AgentRuntime** hardening:
  - Wired `memory.store()` — assistant responses persisted to long-term memory after each turn
  - Integrated `ContextWindow` — history trimmed before each workflow execution
  - Added `AgentEvent::MemoryStored` variant for memory persistence events
  - Replaced 70-line fragile history management with clean 5-line index-based approach

- `workflow.rs` - **Workflow Engine** memory integration:
  - `ThinkStep` now persists user inputs to long-term memory (>10 chars)
  - `ActStep` persists web research results (`web_search`, `read_web_page`) to memory
  - Fixed duplicate doc comments in `ThinkStep::execute` and `Workflow`

- `memory.rs` - Removed dead code annotations from `store()` (now actively used)

**Services (`src-tauri/src/services/`)**

- `command_poller.rs` — Added `AgentEvent::MemoryStored` handling in progress reporter

### Changed

- Removed `@deprecated` tags from `runtime.rs` and `workflow.rs` — these are the canonical v2 modules
- History management in `AgentRuntime::run` simplified from verbose 70-line block to clean index arithmetic

### Technical

- Build: 0 warnings, 0 errors
- Tests: `context_window::tests` — 3/3 passed
- Memory persistence: user inputs, assistant responses, and web research results now stored in SQLite

---

## [0.5.8] - 2026-02-09 - Production Ops Integration (ATM + Native Runtime)

### Added - Metrics Operations Hardening

**Private Cloud Runtime (`Rainy ATM`)**

- Integrated new production operations APIs and policy/audit flows.
- Added owner-authenticated policy mutation path with validation.
- Added immutable policy audit trail and retention lifecycle support.

**Desktop Native Runtime (`src-tauri/src/`)**

- `services/atm_client.rs`, `commands/atm.rs`, `lib.rs`:
  - Added Tauri bridge support for:
    - alert retention get/update/cleanup
    - admin permissions get/update
    - admin permissions audit listing

**Frontend (`src/`)**

- `services/tauri.ts`:
  - Added typed wrappers and DTOs for retention, permission update, and permission audit APIs.

- `components/neural/NeuralPanel.tsx`:
  - Added production dashboard controls:
    - Alert History filters (`open`, `acked`, `resolved`, `all`)
    - Alert retention editor and manual cleanup trigger
    - Admin Policy (Owner Auth) permission toggles
    - Policy audit timeline with changed-key before/after visibility
  - Added UI-level capability gating for:
    - SLO save
    - alert acknowledge
    - retention save
    - cleanup run

### Changed

- Operational controls now follow workspace policy-driven capability checks, enforced both server-side and in UI.
- Metrics alert lifecycle now includes retention-driven pruning for stale `acked`/`resolved` records.

### Fixed

- Fixed TypeScript ack payload parsing edge case (`ackedBy` union fallback issue) in ATM admin alert acknowledge flow.
- Removed dashboard action paths that could execute when policy disallowed the operation.

### Security Fixes

- **SDK Hardening**: Upgraded `rainy-sdk` to v0.6.4 to resolve `bytes` crate vulnerability (RUSTSEC-2026-0007).

## [0.5.7] - 2026-02-09 - Native Runtime Enhancement (AgentSpec V2)

### Added - AgentSpec V2 Persistence & Runtime

**Rust Backend (`src-tauri/src/`)**

- `ai/agent/runtime.rs` - **AgentRuntime** upgrade:
  - Refactored to use `AgentSpec` V2 and `RuntimeOptions`.
  - New `generate_system_prompt` logic that builds rich, multi-layered instructions (Soul, Capabilities, Memory, Rules).
  - Integration with the new `AgentMemory` system.

- `ai/agent/manager.rs` - **AgentManager** (Persistence):
  - Added `spec_json` and `version` columns support in database.
  - New `create_agent` and `get_agent_spec` methods for V2 hydration.
  - Implemented speculative fallback for legacy agents (v1 -> v2).

- `services/command_poller.rs` - **Cloud Bridge**:
  - Injected `AgentManager` into runtime context.
  - Commands now trigger execution based on persisted `agentId` with local ephemeral fallback.

- `commands/agent.rs` - **Agent Commands**:
  - `run_agent_workflow` now loads agent specs directly from the database, ensuring consistency with API operations.

- `ai/agent/verification_test.rs` - **Verification**:
  - Added end-to-end test verifying DB -> Spec -> Runtime cycles.

### Changed

- `ai/agent/workflow.rs` - Updated `AgentState` to hold `Arc<AgentSpec>`.
- `agents/mod.rs` - Marked legacy module as `@DEPRECATED`.

### Fixed

- `services/skill_executor.rs` - Removed unused neural service imports in mock constructor.
- Resolved multiple compilation warnings related to unused fields and imports.

---

## [0.5.6] - 2026-02-03 - Workspace Sync Fixes

### Fixed - Workspace Path Synchronization

**Cloud (`Rainy ATM`)**

- `services/command-bridge.ts` - Commands now include `allowedPaths` from workspace config:
  - Fetches workspace config to get configured paths
  - Includes `allowedPaths` in every command payload
  - Fallback: extracts root path from command params (e.g., `/Users/fer/Projects` from `/Users/fer/Projects/myapp/file.ts`)

- `routes/nodes.ts` - Node registration now stores `allowedPaths` in workspace config:
  - Accepts optional `allowedPaths` array from Desktop during registration
  - Merges new paths with existing config (additive, no overwrite)
  - Logs path updates for debugging

**Desktop (`src-tauri/src/`)**

- `services/skill_executor.rs` - Updated path resolution to handle Cloud workspaces:
  - First tries to load local workspace by ID
  - Falls back to Cloud-provided `allowedPaths` from command payload
  - **New:** Allows absolute paths to serve as ad-hoc workspace roots (bootstrapping)
  - Improved error messages to guide Agent ("Please provide an absolute path")

- `models/neural.rs` - Added `allowedPaths` field to `RainyPayload`

- `services/neural_service.rs` - Updated registration to send `allowedPaths`

- `commands/neural.rs` - Added `allowedPaths` parameter to `register_node` command

**Frontend (`src/`)**

- `services/tauri.ts` - Updated `registerNode` function signature
- `hooks/useNeuralService.ts` - Updated to pass empty paths (Cloud provides per-command)
- `components/neural/NeuralPanel.tsx` - Updated registration call

### Technical

- Resolves "Workspace not found locally and no allowed_paths in command" error
- Cloud workspace IDs now work with Desktop without requiring local workspace files
- Path extraction fallback enables file operations without prior workspace configuration

---

## [0.5.5] - 2026-02-02 - Neural Link UI

### Added - Frontend Pairing Interface

**Components (`src/components/neural/`)**

- `NeuralPanel.tsx` - Primary UI for Neural Link management:
  - Connection status card with real-time status indicators
  - Node ID display with copy-to-clipboard functionality
  - Reconnect button for error/offline states
  - Security Approvals section for Airlock requests
  - Pending approval cards with Approve/Deny buttons

**Hooks (`src/hooks/`)**

- `useNeuralService.ts` - React hook for Neural System integration:
  - Automatic node registration on mount
  - Heartbeat loop for connection maintenance
  - Airlock event listener for approval requests
  - `connect()`, `respond()` methods exposed

**API Bindings (`src/services/tauri.ts`)**

- Neural System types: `AirlockLevel`, `ApprovalRequest`, `SkillManifest`
- Commands: `registerNode`, `sendHeartbeat`, `respondToAirlock`

### Changed

- `Sidebar.tsx` - Added "Neural Link" navigation item with Network icon
- `App.tsx` - Added route handling for `neural-link` section

---

## [0.5.4] - 2026-01-29 - Introduction of Distributed Neural System & Airlock

### Added - Desktop Nerve Center (Tauri)

**Rust Backend (`src-tauri/src/`)**

- `services/neural_service.rs` - **NeuralService** for Cloud Cortex integration:
  - RainyRPC protocol implementation (polling-based)
  - `register_node` - Handshake with Cloud Cortex
  - `heartbeat` - Send status and fetch pending commands
  - `start_command` / `complete_command` - Lifecycle management

- `services/airlock.rs` - **Airlock Security** implementation:
  - Security firewall for incoming Cloud commands
  - **Level 0 (Safe)**: Auto-approved read-only operations
  - **Level 1 (Sensitive)**: Notification required (Write ops)
  - **Level 2 (Dangerous)**: Explicit approval required (Execute ops)
  - Event-based approval flow with frontend integration

- `models/neural.rs` - Shared data models:
  - `RainyMessage`, `RainyContext`, `RainyPayload`
  - `AirlockLevel`, `CommandStatus`, `DesktopNodeStatus`

- `commands/` - New Tauri commands:
  - **Neural**: `register_node`, `send_heartbeat`, `poll_commands`
  - **Airlock**: `respond_to_airlock`, `get_pending_airlock_approvals`

### Added - Cloud Cortex (Infrastructure)

**Database & API (`Rainy ATM`)**

- Turso Database Schema:
  - `desktop_nodes` - registry of connected devices
  - `command_queue` - persistent command buffer
  - `command_log` - immutable audit trail

- API Endpoints:
  - `POST /v1/nodes/register` - Node registration
  - `POST /v1/nodes/:id/heartbeat` - Status sync & command fetch
  - `POST /v1/nodes/:id/commands/...` - Command status updates

### Technical

- Full end-to-end "Distributed Neural System" architecture
- Secure "Airlock" pattern for remote command execution
- Polling-based bidirectional communication (Cloud <-> Desktop)
- Type-safe implementation across TypeScript (Cloud) and Rust (Desktop)

## [0.5.3] - 2026-01-28 - PHASE 3: Intelligent Router Integration

### Added - PHASE 3: Intelligent Router Commands

**Rust Backend (`src-tauri/src/`)**

- `commands/router.rs` - IntelligentRouter Tauri commands (10 commands):
  - `get_router_config` - Retrieve router configuration (load balancing, fallback, circuit breaker settings)
  - `update_router_config` - Update router configuration dynamically
  - `get_router_stats` - Get router statistics (provider count, healthy providers, open circuits)
  - `complete_with_routing` - Chat completion with intelligent provider selection
  - `stream_with_routing` - Streaming chat with Tauri v2 channels for real-time events
  - `embed_with_routing` - Embeddings with intelligent provider selection
  - `add_provider_to_router` - Add a provider to the intelligent router
  - `remove_provider_from_router` - Remove a provider from the router
  - `get_router_providers` - List all providers in the router
  - `router_has_providers` - Check if router has any providers

- `commands/mod.rs` - Added router module export and re-exports

- `lib.rs` - Updated to:
  - Initialize IntelligentRouter state with `Arc<RwLock<IntelligentRouter>>`
  - Register IntelligentRouterState for Tauri state management
  - Register all 10 router commands

**Frontend (`src/`)**

- `services/tauri.ts` - Added TypeScript types and functions:
  - `RouterConfigDto` - Router configuration interface
  - `RouterStatsDto` - Router statistics interface
  - `RoutedChatRequest` - Chat request with routing options
  - `RoutedEmbeddingRequest` - Embedding request with routing options
  - `StreamingEvent` - Discriminated union for streaming events (started/chunk/finished/error)
  - 10 typed functions for router command invocation

- `hooks/useIntelligentRouter.ts` - New React hook for intelligent routing:
  - Configuration management (get/update)
  - Provider management (add/remove/list)
  - Routed completions (complete/stream/embed)
  - Streaming state management with content accumulation
  - Error handling and loading states

- `hooks/index.ts` - Exported useIntelligentRouter hook

### Technical

- IntelligentRouter now integrated at application level
- Streaming uses Tauri v2 Channel API for efficient real-time events
- Router state is protected by `RwLock` for thread-safe access
- All router components (LoadBalancer, CostOptimizer, FallbackChain, CircuitBreaker) are now accessible

### Notes

- Streaming implementation uses Tauri channels for ordered event delivery
- Router configuration can be updated at runtime without restart
- Provider health and circuit breaker states are tracked per-provider
- Cost optimization considers token pricing and budget limits

## [0.5.2] - 2026-01-28 - PHASE 3: xAI Provider Implementation

### Added - PHASE 3: xAI (Grok) Provider Implementation

**Rust Backend (`src-tauri/src/`)**

- `ai/providers/xai.rs` - xAI provider implementation:
  - Direct integration with xAI API (Grok-3, Grok-3-fast, Grok-2, Grok-2-fast)
  - Support for chat completions and streaming
  - OpenAI-compatible API structure
  - Full error handling with proper status code mapping
  - XAIProviderFactory for provider creation
  - Comprehensive test coverage for provider functionality

- `ai/providers/mod.rs` - Updated exports:
  - Exported xAI provider implementation and factory

- `ai/mod.rs` - Updated module exports:
  - Added exports for xAI provider types and factory

- `commands/ai_providers.rs` - Enhanced provider registration:
  - Support for registering xAI providers
  - Dynamic provider creation based on provider type
  - Proper validation for xAI provider type

### Notes

- xAI API uses OpenAI-compatible endpoints at https://api.x.ai/v1
- Grok-3 is the default model with 131K context window
- Streaming support via Server-Sent Events (SSE)
- Embeddings not supported by xAI (returns UnsupportedCapability error)

## [0.5.1] - 2026-01-28 - PHASE 3: Individual Provider Implementations

### Added - PHASE 3: Individual AI Provider Implementations

**Rust Backend (`src-tauri/src/`)**

- `ai/providers/openai.rs` - OpenAI provider implementation:
  - Direct integration with OpenAI API (GPT-4, GPT-4o, o1 models)
  - Support for chat completions, streaming, and embeddings
  - Full OpenAI API compatibility with customizable base URL
  - Complete error handling with proper status code mapping
  - OpenAIProviderFactory for provider creation

- `ai/providers/anthropic.rs` - Anthropic provider implementation:
  - Direct integration with Anthropic API (Claude 3.5/4, Opus, Sonnet, Haiku)
  - Support for chat completions and streaming
  - Messages API with proper system prompt handling
  - Vision support for Claude 3.5 Sonnet
  - AnthropicProviderFactory for provider creation

- `ai/provider_types.rs` - Enhanced error handling:
  - Added `Configuration` error variant for config-related errors
  - Improved error messages across all error types

- `ai/providers/mod.rs` - Updated exports:
  - Exported new provider implementations (OpenAI, Anthropic)
  - Exported corresponding factory types

- `ai/mod.rs` - Updated module exports:
  - Added exports for new provider types and factories

- `commands/ai_providers.rs` - Enhanced provider registration:
  - Support for registering OpenAI and Anthropic providers
  - Dynamic provider creation based on provider type
  - Proper validation for each provider type

### Notes

- Google Gemini provider already exists at `ai/gemini.rs` with full Gemini 3 support
- Gemini 3 models include: gemini-3-flash-minimal, gemini-3-flash-high, gemini-2.5-flash-lite
- Existing Gemini implementation supports thinking levels and multimodal capabilities

## [0.4.3] - 2026-01-28 - PHASE 3: AI Provider Integration Foundation

### Added - PHASE 3: AI Provider Integration

**Rust Backend (`src-tauri/src/`)**

- `ai/provider_types.rs` - Core types for provider abstraction:
  - ProviderId, ProviderType, ProviderCapabilities, ProviderHealth
  - ProviderConfig, ChatMessage, ChatCompletionRequest/Response
  - TokenUsage, EmbeddingRequest/Response, StreamingChunk
  - AIError enum, ProviderResult type alias, StreamingCallback

- `ai/provider_trait.rs` - AIProvider trait and factory:
  - AIProvider trait with 10 methods (id, provider_type, capabilities, health_check, complete, complete_stream, embed, supports_capability, default_model, available_models, config)
  - AIProviderFactory trait for provider creation
  - ProviderWithStats wrapper for statistics tracking
  - ProviderStats struct with request counts, latency, tokens, last_request

- `ai/provider_registry.rs` - Central provider registry:
  - ProviderRegistry with DashMap for thread-safe access
  - register(), unregister(), get(), get_all(), get_by_type(), get_healthy()
  - set_default(), get_default() for default provider management
  - complete(), complete_stream(), embed() with automatic stats tracking
  - get_stats(), get_all_stats() for statistics retrieval
  - clear(), count() for registry management

- `ai/providers/rainy_sdk.rs` - Rainy SDK provider implementation:
  - RainySDKProvider with capability caching (5-minute TTL)
  - Support for both Rainy API and Cowork modes
  - Automatic capability detection from SDK
  - Health checks via simple chat completion
  - RainySDKProviderFactory for provider creation

- `ai/router/` - Intelligent routing system:
  - `router.rs` - IntelligentRouter with load balancing, cost optimization, capability matching, and fallback
  - `load_balancer.rs` - LoadBalancer with 4 strategies (RoundRobin, LeastConnections, WeightedResponseTime, Random)
  - `cost_optimizer.rs` - CostOptimizer with budget limits and cost estimation
  - `capability_matcher.rs` - CapabilityMatcher for task-based provider selection
  - `fallback_chain.rs` - FallbackChain with circuit breaker and exponential backoff
  - `circuit_breaker.rs` - CircuitBreaker with Open/Closed/HalfOpen states

- `ai/features/` - Enhanced features:
  - `web_search.rs` - WebSearchService with search, search_with_answer, search_results_only
  - `embeddings.rs` - EmbeddingService with cosine_similarity and euclidean_distance
  - `streaming.rs` - StreamingService with chunks_to_text and get_final_chunk
  - `usage_analytics.rs` - UsageAnalytics with ProviderUsage, TotalUsage, UsageStatistics

- `commands/ai_providers.rs` - Tauri commands for provider management (14 commands):
  - list_all_providers, get_provider_info, register_provider, unregister_provider
  - set_default_provider, get_default_provider
  - get_provider_stats, get_all_provider_stats
  - test_provider_connection, get_provider_capabilities
  - complete_chat, generate_embeddings, get_provider_available_models
  - clear_providers, get_provider_count

**Frontend Hooks & Services (`src/`)**

- `hooks/useAIProvider.ts` - Updated for new provider registry commands
- `hooks/useStreaming.ts` - New hook for streaming completions
- `hooks/useUsageAnalytics.ts` - New hook for usage tracking

**Dependencies**

- `rainy-sdk` v0.6.1 - Full integration with rate-limiting, tracing, and cowork features
- `async-trait` - Async trait support for AIProvider
- `dashmap` - Concurrent HashMap for provider registry
- `chrono` - DateTime support for statistics

**Architecture**

- Modular provider abstraction with trait-based design
- Intelligent routing with load balancing and fallback
- Comprehensive usage tracking and analytics
- Thread-safe provider registry with statistics

### Changed

- Updated `ai/mod.rs` to export all PHASE 3 modules
- Updated `commands/mod.rs` to export ai_providers module
- Updated `lib.rs` to add ProviderRegistry state and register new commands

### Technical

- All PHASE 3 foundation components implemented
- Provider abstraction layer complete with trait-based design
- Intelligent router with circuit breaker and fallback chain
- Enhanced features for web search, embeddings, streaming, and usage analytics
- 14 new Tauri commands for provider management
- Full modularization compliance (<400 lines per module)

### Breaking Changes

- None - PHASE 3 is additive, maintains backward compatibility

### Migration Notes

- Existing AIProviderManager remains functional
- New provider registry is opt-in via Tauri commands
- Frontend hooks updated to support new provider management

## [0.4.2] - 2026-01-27 - PHASE 2: Intelligence Layer Complete

### Multi-Agent System

- DirectorAgent for task decomposition and orchestration
- 6 Specialized Agents (Researcher, Executor, Creator, Designer, Developer, Analyst)
- CriticAgent for quality evaluation and improvement suggestions
- GovernorAgent for security policy enforcement and compliance

### Memory System

- ShortTermMemory with RingBuffer (100 entries)
- LongTermMemory with LanceDB integration (structure ready)
- MemoryManager coordinating both memory types
- 9 Tauri commands for memory management

### Reflection & Self-Improvement

- ReflectionEngine for error pattern recognition
- Strategy optimization and learning
- Post-task analysis loop
- 9 Tauri commands for reflection and governance

### Architecture

- Agent trait with 7 methods
- BaseAgent with common functionality
- AgentRegistry for agent lifecycle management
- MessageBus for inter-agent communication
- 23 Tauri commands for multi-agent system
- Full modularization compliance (<400 lines per module)
- Comprehensive unit tests for all components

### Breaking Changes

- None

### Migration Notes

- All PHASE 1 features remain compatible
- Multi-agent system is opt-in via Tauri commands
- Memory system integrates with existing workspace context

## [0.4.1] - 2026-01-27

### Added - PHASE 1: Core Cowork Engine Complete

**Architecture & Planning**

- Comprehensive PHASE 1 implementation plan created and documented
- Multi-agent architecture designed with Director, Researcher, Executor, Creator, Critic, and Governor agents
- Memory system architecture defined (short-term and long-term memory)
- Reflection and self-improvement engine specifications completed-
- Integration points mapped between all system layers

**Workspace Management Enhancements**

- Workspace permission inheritance system designed
- Workspace template system integration planned
- Workspace analytics dashboard specifications created
- Hierarchical permission system with override capabilities

**File System Operations Enhancements**

- Enhanced workspace context integration planned
- File versioning UI components designed
- Batch operations progress tracking specifications
- Comprehensive audit log system requirements defined

**Task Queue System Enhancements**

- Parallel task execution with configurable concurrency designed
- Task queue persistence for crash recovery planned
- Task dependency visualization requirements created
- Automatic retry with exponential backoff specified
- Background task processing with notifications designed

**Multi-Agent System**

- Director Agent architecture for task decomposition and coordination
- Specialized Agents (Researcher, Executor, Creator, Designer, Developer, Analyst) specifications
- Critic Agent for quality evaluation and improvement suggestions
- Governor Agent for security policy enforcement and compliance

**Memory System**

- Short-term memory with context window and recent actions tracking
- Long-term memory with episodic, semantic, and procedural memory
- Vector database integration for semantic search
- Cross-session persistence architecture

**Reflection & Self-Improvement**

- Post-task analysis loop design
- Error pattern recognition system
- Strategy optimization engine
- Prompt and tool auto-refinement mechanisms

**Documentation**

- Detailed implementation plan saved to `plans/phase1-implementation-plan.md`
- Architecture diagrams for system layers and multi-agent coordination
- Migration path with 3-week timeline (Phase 1.1, 1.2, 1.3)
- Success criteria and risk assessment documented
- Testing strategy defined (unit, integration, performance, security)

### Changed

- Project roadmap updated to reflect PHASE 1 completion status
- Development priorities aligned with multi-agent architecture
- Technical stack validated for PHASE 1 requirements

### Technical

- File structure planned for new agent, memory, and reflection modules
- Dependencies identified for vector database, semantic search, and graph visualization
- API endpoints designed for workspace, file operations, task queue, agents, and memory
- Integration points defined between Workspace, File Operations, Task Queue, and Multi-Agent systems

## [0.4.0] - 2026-01-26

### Added - Open Core Business Model by Enosis Labs

**New Version - The System of Rewortk to the new system called Rainy MaTE**

- Updated the system to the new system called Rainy MaTE
- Enhanced the qulty and roadmap
- Added new features
- Biggest changes is incoming, wait for it...

### Changed

- Version bump to 0.4.0 across all config files
- Table of Contents now includes Open Core + Licensing and Business Model sections
- Executive Summary differentiators table updated with new business model columns

## [0.3.2] - 2026-01-19

### Added - AI File Operations Engine

**Rust Backend (`src-tauri/src/`)**

- `services/file_operations.rs` - FileOperationEngine with parallel processing:
  - Move files with conflict resolution (skip/overwrite/rename/ask)
  - Batch rename with pattern templates (`{name}`, `{stem}`, `{ext}`, `{counter}`)
  - Safe delete (moves to trash for recovery)
  - Organize folder by type, date, extension, or content
  - Workspace analysis with optimization suggestions
  - Full undo support with operation history
- `services/ai_agent.rs` - CoworkAgent for autonomous file operations:
  - Natural language instruction parsing via AI
  - Multi-step task planning (TaskPlan with PlannedStep array)
  - Real-time execution with AgentEvent streaming
  - Safety checks for destructive operations

**Tauri Commands (12 new)**

- File Operations: `move_files`, `organize_folder`, `batch_rename`, `safe_delete_files`, `analyze_workspace`, `undo_file_operation`, `list_file_operations`
- Agent Commands: `plan_task`, `execute_agent_task`, `get_agent_plan`, `cancel_agent_plan`, `agent_analyze_workspace`

**Frontend (`src/`)**

- `services/tauri.ts` - TypeScript bindings for all new commands:
  - Types: `FileOpChange`, `TaskPlan`, `PlannedStep`, `WorkspaceAnalysis`, `AgentEvent`, etc.
  - Functions: `moveFiles`, `organizeFolder`, `planTask`, `executeAgentTask`, etc.

**Dependencies**

- `rayon` v1.10 - Parallel processing (available for future optimization)
- `dirs` v5.0 - Cross-platform directories for trash location

### Added - CoworkPanel UI

**Frontend Components**

- `components/cowork/CoworkPanel.tsx` - Chat-style AI agent interface:
  - Message bubbles for user/agent conversations
  - Natural language input with Enter to send
  - Quick actions (Analyze, Organize by type)
  - Plan preview with Execute/Cancel buttons
  - Real-time progress during execution
- `hooks/useCoworkAgent.ts` - React hook for agent state management
- Sidebar integration: "AI Cowork" item in AI Studio section

### Improved - AI Agent Intelligence

**Question vs Command Detection**

- AI now classifies intent as "question" or "command"
- Questions receive direct answers (e.g., "What files are here?" → list of files)
- Commands create executable plans (e.g., "Organize by type" → plan with steps)

**Production Model Strategy**

- **Rainy API (Paid)**: Uses models from SDK's `caps.models` (GPT-4o, GPT-5, Claude, etc.)
- **Gemini BYOK (Free)**: Limited to 3 models:
  - `gemini-3-flash-minimal` - Fast responses, minimal thinking
  - `gemini-3-flash-high` - Deep reasoning for complex tasks
  - `gemini-2.5-flash-lite` - Lightweight, cost-effective
- Automatic fallback: Rainy API → Gemini if request fails
- Model attribution in every response ("_Powered by gpt-4o via Rainy API_")

### Fixed

- State type mismatch in file commands (`FileManager` → `Arc<FileManager>`)
- Empty plan display for questions (now shows direct answers instead)
- Hardcoded Gemini provider (now uses SDK's model list for paid users)

## [0.3.1] - 2026-01-19

### Added - Folder Upload & Project System

**Rust Backend (`src-tauri/src/`)**

- `models/folder.rs` - UserFolder model with persistence:
  - ID, path, name, accessType
  - `addedAt` and `lastAccessed` timestamps for history ordering
- `services/folder_manager.rs` - Folder management service:
  - JSON persistence in app data directory
  - Add/remove/list folders
  - `update_last_accessed()` for recent ordering
  - Automatic sorting by most recent first
- `commands/folder.rs` - Tauri commands:
  - `add_user_folder` - Add folder via picker
  - `list_user_folders` - Get all folders (sorted by recent)
  - `remove_user_folder` - Delete a folder
  - `update_folder_access` - Update last accessed timestamp

**Frontend (`src/`)**

- `hooks/useFolderManager.ts` - React hook for folder operations:
  - Native folder picker via `@tauri-apps/plugin-dialog`
  - Automatic refresh and ordering
- `services/tauri.ts` - UserFolder type and bindings

### Added - Folder UX Enhancements

- **Active Folder Indicator** - Visual highlighting in sidebar when a project is selected
- **Recent Project Ordering** - Folders sorted by `lastAccessed` (most recent first)
- **Workspace Title Header** - "Rainy Cowork in [path]" displayed in main content when a folder is active

**Frontend Changes**

- `components/layout/TahoeLayout.tsx` - Added workspace title header with folder icon
- `components/layout/FloatingSidebar.tsx` - Added `activeFolderId` prop for highlighting
- `App.tsx` - Active folder state tracking, calls `updateFolderAccess` on selection

### Added - Folder Requirement Gate

- **System blocked without folder** - Tasks and AI features require an active folder
- `NoFolderGate` component prompts users to select a folder before using the system
- Clear messaging: "To get started, select a folder where Rainy Cowork will work"

### Technical

- Folders persist in `~/.tauri/com.enosislabs.rainycowork/user_folders.json`
- macOS/Windows folder picker handled via Tauri dialog plugin
- Existing `dialog:allow-open` capability already configured

## [0.3.0] - 2026-01-18

### Added - Phase 3: Web Research

**Rust Backend (`src-tauri/src/`)**

- `services/web_research.rs` - Web Research service:
  - URL fetching with reqwest
  - HTML-to-Markdown conversion (Rust-native via scraper)
  - DashMap caching with 5-minute TTL
  - Error handling with `WebResearchError` enum
- `commands/web.rs` - Tauri commands:
  - `fetch_web_content` - Extract content from URL
  - `get_web_cache_stats` - Cache statistics
  - `clear_web_cache` - Clear cached content

**Frontend (`src/`)**

- `types/web.ts` - WebResearchContent and WebCacheStats types
- `hooks/useWebResearch.ts` - React hook for content extraction

**Dependencies**

- `scraper` v0.23 - HTML parsing
- `url` v2.5 - URL validation
- `regex` v1.11 - Markdown cleanup

**Documentation**

- `ROADMAP.md` - Public roadmap with version milestones

**Tavily Web Search (rainy-api-v2)**

- `services/tavily.ts` - Tavily SDK wrapper:
  - Search with depth, domains, answer options
  - Content extraction from URLs
  - Singleton pattern with environment initialization
- `routes/search.ts` - Search API endpoints:
  - `POST /api/v1/search` - Web search with Zod validation
  - `POST /api/v1/search/extract` - Content extraction
  - Cowork plan `web_research` feature gating

### Added - Phase 3: Document Generation

**Rust Backend (`src-tauri/src/`)**

- `services/document.rs` - Document generation service:
  - Handlebars template engine
  - 4 built-in templates (meeting notes, project report, email, quick note)
  - Markdown → HTML conversion
- `commands/document.rs` - Tauri commands:
  - `list_document_templates` - List all templates
  - `get_template` - Get specific template
  - `generate_document` - Generate from template + context
  - `markdown_to_html` - Convert markdown to HTML

**Frontend (`src/`)**

- `types/document.ts` - TemplateInfo, GeneratedDocument types
- `hooks/useDocument.ts` - React hook for document generation

**Dependencies**

- `handlebars` v6 - Template rendering

### Added - Phase 3: Image Processing

**Rust Backend (`src-tauri/src/`)**

- `services/image.rs` - Image processing service:
  - EXIF metadata extraction (camera, date, GPS, settings)
  - Thumbnail generation (base64 PNG)
  - Image dimensions and format detection
- `commands/image.rs` - Tauri commands:
  - `get_image_metadata` - Full metadata + EXIF
  - `generate_thumbnail` - Resized preview image
  - `get_image_dimensions` - Quick width/height
  - `is_image_supported` - Format check

**Frontend (`src/`)**

- `types/image.ts` - ImageMetadata, ExifData, ThumbnailResult types
- `hooks/useImage.ts` - React hook for image operations

**Dependencies**

- `image` v0.25 - Image processing
- `kamadak-exif` v0.5 - EXIF parsing
- `base64` v0.22 - Thumbnail encoding

### Added - Cowork Plan Integration

**Rust Backend (`src-tauri/src/`)**

- `rainy-sdk` v0.4.2 integration for Cowork services
- `provider.rs` - Updated AIProviderManager with plan-based model access
- `commands/ai.rs` - New `get_cowork_status` command returning plan info, usage tracking, and feature availability
- `CoworkStatus` struct with plan, usage, models, and features
- Caching system for Cowork capabilities (5-minute TTL)

**Frontend (`src/`)**

- `services/tauri.ts` - Added `CoworkStatus`, `CoworkUsage`, `CoworkFeatures` types
- `hooks/useCoworkStatus.ts` - New hook for plan status with computed helpers:
  - `hasPaidPlan`, `plan`, `planName`, `isValid`
  - `usagePercent`, `remainingUses`, `isOverLimit`
  - `canUseWebResearch`, `canUseDocumentExport`, `canUseImageAnalysis`
- `components/settings/SettingsPanel.tsx` - New **Subscription** tab:
  - Plan display with status badge
  - Usage progress bar (color-coded)
  - Remaining uses and reset date
  - Feature availability checkmarks
  - Upgrade button for users on Free plan

### Changed

- `Cargo.toml` - Updated `rainy-sdk` from 0.4.1 to 0.4.2
- Replaced "premium" terminology with plan-based language throughout codebase
- AIProviderManager now uses `is_paid()` instead of `is_premium()`

### Technical

- SDK types: `CoworkTier` → `CoworkPlan` (Free/GoPlus/Plus/Pro/ProPlus)
- SDK types: `CoworkLimits` → `CoworkUsage` with usage tracking fields
- Backward compatibility aliases for deprecated types

## [0.2.0] - 2026-01-17

### Added - Phase 2: Core AI Features Foundation

**Rust Backend (`src-tauri/src/`)**

- `models/mod.rs` - Data models: Task, FileChange, Workspace, TaskEvent, FileVersion
- `commands/` - Tauri commands for tasks, AI, and file operations (18 commands total)
- `services/task_manager.rs` - TaskManager with DashMap, async execution, progress channels
- `services/file_manager.rs` - FileManager with workspace-based versioning (`.rainy-versions/`)
- `ai/provider.rs` - AIProvider trait abstraction and AIProviderManager
- `ai/rainy_api.rs` - Rainy API provider (Enosis Labs) with OpenAI-compatible format
- `ai/gemini.rs` - Google Gemini provider for direct user API keys
- `ai/keychain.rs` - macOS Keychain integration via `security-framework`

**Frontend Hooks & Services (`src/`)**

- `services/tauri.ts` - Typed wrappers for all Tauri commands with Channel support
- `hooks/useTauriTask.ts` - Task management hook with event-driven updates
- `hooks/useAIProvider.ts` - AI provider management with Keychain integration

**Dependencies Added**

- Rust: tokio, reqwest, dashmap, uuid, chrono, thiserror, security-framework, tracing
- Tauri plugins: fs, dialog, notification
- Frontend: @tauri-apps/plugin-fs, plugin-dialog, plugin-notification

### Changed

- Updated `Cargo.toml` with Phase 2 dependencies
- Updated `capabilities/default.json` with fs, dialog, notification permissions
- Rewrote `lib.rs` to wire all modules and register 18 commands

## [0.1.1] - 2026-01-17

### Changed

- **macOS Tahoe-style UI redesign** - Premium floating elements with glassmorphism
- **Floating sidebar** - Rounded corners (24px), drop shadow, collapsible sections
- **Glass surface main content** - Backdrop blur, subtle borders
- **Overlay title bar** - Traffic light spacer on macOS, seamless integration
- **Window transparency** - Enabled in Tauri config for glass effects
- **Improved color palette** - Rose/pink tinted light theme, deep charcoal dark theme
- **Window drag regions** - Proper `-webkit-app-region: drag` for window movement
- **Responsive design** - Works across different screen sizes

### Added

- `FloatingSidebar.tsx` - New collapsible sidebar with Tasks, Favorites, Locations, Settings
- `TahoeLayout.tsx` - New layout component with floating elements
- OS detection for Windows vs macOS controls
- Premium hover elevation effects
- Smooth animations for component appearance

### Technical

- Tauri config: `titleBarStyle: "overlay"`, `transparent: true`
- CSS variables for floating shadows and glass effects
- Custom scrollbar styling matching macOS

### Added

- **Initial Tauri + React + HeroUI v3 foundation**
- **Layout Components**
  - `Header.tsx` - App header with theme toggle (light/dark mode), settings button, user avatar
  - `Sidebar.tsx` - Collapsible navigation sidebar with folders, tasks, history, and settings sections
  - `MainLayout.tsx` - Responsive grid layout combining header, sidebar, and main content
- **Task Components**
  - `TaskInput.tsx` - Natural language task input with HeroUI TextArea, AI provider selector (OpenAI, Anthropic, Ollama), and Start Task button
  - `TaskCard.tsx` - Task display card with progress bar, status icons, pause/stop/view actions
- **File Components**
  - `FileTable.tsx` - Recent file changes display with operation icons (create, modify, delete, move, rename)
- **Type Definitions**
  - `types/index.ts` - TypeScript interfaces for Task, AIProvider, FileChange, Folder, AppSettings
- **Styling**
  - `global.css` - macOS-themed design tokens with OKLCH colors, system fonts (SF Pro), custom animations
  - Dark/light mode with system preference detection
  - Custom scrollbar styling for macOS native feel
- **Configuration**
  - Updated `index.html` with proper title, meta tags, and system font configuration
  - Updated `main.tsx` with correct CSS imports (no HeroUI Provider needed in v3)

### Technical Details

- HeroUI v3 Beta (v3.0.0-beta.3) with compound component patterns
- Tailwind CSS v4 integration
- Tauri 2.0 for native macOS app
- React 19 + TypeScript
- lucide-react for icons

### Known Limitations

- AI provider integration is UI-only (no API calls yet)
- File system operations not connected to Tauri backend
- Toast notifications deferred to v0.2.0
