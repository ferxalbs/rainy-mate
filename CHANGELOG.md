# CHANGELOG

All notable changes to Rainy MaTE will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased] - 2026-03-17 - INTELLIGENT MEMORY DISTILLATION & CHAT PERFORMANCE AND MEMORY STRATEGY DISPATCH & AGENT RUNTIME HARDENING

### Added

- **Memory distillation pipeline** — replaces raw conversation dumping with LLM-based fact extraction:
  - `MemoryDistiller` (`src-tauri/src/services/memory_vault/distiller.rs`) extracts structured facts from conversation turns via cheap LLM call (temperature 0.1, 1024 tokens)
  - `DedupGate` (`src-tauri/src/services/memory_vault/dedup.rs`) checks embedding similarity before storage — Skip (distance <0.05), Update (<0.15), or Insert
  - `DistillationBuffer` in `AgentMemory` batches 5 turns before flushing; flushes remaining at end of each agent run
  - Fast-path skip: read-only tool results (`read_file`, `list_files`, `git_status`, etc.) never enter the pipeline
- **Memory categories** — `MemoryCategory` enum: `Preference`, `Correction`, `Fact`, `Procedure`, `Observation` stored in encrypted metadata as `_category`
- **Importance scoring** — each distilled memory gets a 0.0–1.0 importance score stored as `_importance`; floors enforced (Preference ≥0.7, Correction ≥0.8)
- **Category-aware retrieval scoring** — hybrid and vector-only search formulas now include `_importance` (12%) and `category_boost` (10–15%) weights; preferences and corrections rank highest
- **Memory Explorer UI enhancements** — category badge (colored per type), importance bar (thin progress), and category filter dropdown in `MemoryExplorerPanel.tsx`

### Fixed

- **Agent chat 58% CPU on scroll** — complete rendering pipeline overhaul:
  - `stopAgentRun`/`retryAgentRun` no longer depend on `messages` array (uses `messagesRef` pattern) — stabilizes callback references so `React.memo` on `MessageBubble` actually works
  - Replaced all Framer Motion infinite animations (`motion.div`) with CSS `@keyframes` (`neural-bar`, `shimmer`) — eliminates JS animation frame overhead
  - Hoisted `MarkdownRenderer` component overrides, remark/rehype plugin arrays to module scope — zero allocation per render
  - `NeuralStatus` wrapped in `React.memo` to prevent cascade re-renders during streaming
  - Scroll updates coalesced via `requestAnimationFrame` instead of firing on every streaming chunk
  - Removed `transition-all duration-300` from message scroll container — eliminates layout thrashing
  - `latestTelemetry` computed via `useMemo` (reverse loop) instead of `.reverse().find()` on every render
  - Simplified `MessageBubble` memo comparator from 7 checks to 3 (message, isExecuting, workspaceId)
  - Added transcript windowing via `VirtualTranscript` + `useVirtualTranscript` so only visible message rows stay mounted while preserving prepend-history scroll position and bottom anchoring during streaming
  - Lazy-mounted runtime trace bodies and paged trace rows behind explicit expansion, preventing collapsed supervisor/sub-agent traces from keeping large hidden DOM trees alive
  - Removed the chat markdown `content-visibility` placeholder path inside virtualized rows and increased transcript overscan to eliminate the transient ghost gap that appeared on mount/screen switches with large markdown tables
- **User message markdown rendering and theme consistency**:
  - User-authored chat messages now render markdown with the same parser as assistant responses, including tables, fenced code blocks, blockquotes, links, and inline code
  - Restored the original `bg-primary text-primary-foreground` user bubble styling and made the user-markdown tone inherit that existing bubble palette instead of introducing hard-coded gradients or theme-specific color overrides
  - Tightened user markdown contrast inside the original bubble so pasted markdown remains legible across the app's existing theme system without breaking the prior visual pattern
- **React Compiler compatibility fixes** (react-doctor 95→99/100):
  - Replaced `Date.now()` in render paths with `performance.now()` via refs (`ThoughtDisplay`, `MessageBubble.formatDuration`)
  - Extracted `EmptyStatePrompts` and `TelemetryBar` as `React.memo` sub-components from `AgentChatPanel`
  - Eliminated inline `renderComposer()` function — renders `<ChatComposer>` directly
  - All event handlers wrapped in `useCallback` with stable deps
  - Replaced array index keys with content-based keys in `MessageBubble` step/error lists
  - Added module-scope `EMPTY_REASONING` constant to prevent per-render array allocation
  - **ThoughtDisplay**: extracted `useElapsedTimer` hook — no refs during render, no synchronous setState in effects; derived `isExpanded` from `isStreaming` prop instead of cascading effect
  - **AgentChatPanel**: replaced reasoning effort `useEffect` → `useMemo` derivation — zero cascading renders
  - **App.tsx**: auto-select folder uses `.then()` callbacks instead of synchronous setState in effect
  - **AppSidebar**: consolidated 6 `useState` → `useReducer` for updater state; extracted default `[]` to module-scope `EMPTY_FOLDERS` constant

### Added

- **Memory strategy dispatch** — `AgentSpec.memory_config.strategy` now controls runtime RAG retrieval (was display-only):
  - `"simple_buffer"` — reads in-process ring buffer only (zero vault I/O, fastest, ephemeral agents)
  - `"vector"` — ANN/Exact vault vector search only, no lexical merge (high-precision)
  - `"hybrid"` — existing behavior: vault vector + lexical, merged by scoring (default)
  - Added `SemanticRetrievalMode::SimpleBuffer` variant to `src-tauri/src/services/memory/types.rs`
  - `search_semantic_detailed()` accepts new `strategy: &str` param; `search()` convenience wrapper defaults to `"hybrid"` (`src-tauri/src/services/memory/memory_manager.rs`)
- **`retention_days` pruning** — memory entries older than the configured retention window are now actively pruned:
  - Per-workspace prune on each agent run (`prune_expired(workspace_id, retention_days)`)
  - Global startup sweep (`prune_global_expired(365)`) spawned during `MemoryManager::init()` as a safety net
  - `delete_workspace_entries_older_than` / `delete_all_entries_older_than` with cascade to embedding vectors (`src-tauri/src/services/memory_vault/repository.rs`)
  - `prune_workspace_expired` / `prune_global_expired` added to `MemoryVaultService` (`src-tauri/src/services/memory_vault/service.rs`)
- **Workspace scoping via persistence flags** — `AgentRuntime.effective_workspace_id()` computes a compound vault key based on:
  - `per_connector_isolation: true` → appends `connector_id`
  - `session_scope: "per_user"` → appends `user_id`; `"per_channel"` → appends `connector_id` when isolation is off
  - `cross_session: false` → agent skips writing memories to vault entirely
- **Per-agent rate limiting** — `max_requests_per_minute` enforced in `AgentRuntime` via a sliding window `VecDeque<Instant>`; returns `RateLimitExceeded` error immediately when limit is hit (`src-tauri/src/ai/agent/runtime.rs`)
- **connector_id / user_id end-to-end propagation** — connector context flows from ATM webhooks → `command_queue` payload → desktop poller → `RainyPayload` → `RuntimeOptions` → `effective_workspace_id()`:
  - `RainyPayload` gains `connector_id` and `user_id` fields with `#[derive(Default)]` (`src-tauri/src/models/neural.rs`)
  - `RuntimeOptions` gains `connector_id` and `user_id` fields (`src-tauri/src/ai/agent/runtime.rs`)
  - `command_poller.rs` reads and forwards both fields to `RuntimeOptions` (`src-tauri/src/services/command_poller.rs`)
- **5 new integration/unit tests**:
  - `test_simple_buffer_returns_ring_buffer_only` — verifies no vault I/O on `simple_buffer` strategy
  - `test_prune_expired_removes_old_entries` — verifies retention pruning removes stale entries
  - `test_simple_buffer_no_cross_workspace_leak` — verifies workspace isolation in ring buffer path
  - `test_rate_limiter_blocks_after_limit` — verifies RPM gate fires at threshold
  - `test_rate_limiter_allows_after_window_expires` — verifies sliding window resets correctly
- **Supervisor/UI projection helpers** — extracted focused runtime/UI support modules to reduce hot-path duplication:
  - Rust frontend event projector in `src-tauri/src/commands/agent_frontend_events.rs`
  - frontend message update helper in `src/hooks/agent-chat/messageState.ts`

### Fixed

- **Dead code from CodeRabbit auto-fix** — `KeychainManager::is_supported()` was injected by CodeRabbit bot (commit `7adca1f`) without any call site; removed entirely since per-method `#[cfg]` branches already handle platform detection (`src-tauri/src/ai/keychain.rs`)
- **Durable local Airlock approvals** — local agent tool/memory approvals no longer disappear on timeout by default:
  - `QueuedCommand` gained explicit `approval_timeout_secs` control in `src-tauri/src/models/neural.rs`
  - local workflow/runtime-generated approvals now use indefinite blocking semantics (`Some(0)`) in `src-tauri/src/ai/agent/workflow.rs` and `src-tauri/src/ai/agent/runtime.rs`
  - `src-tauri/src/services/airlock.rs` now represents non-expiring approvals cleanly and only times out when explicitly configured
- **Supervisor verifier lane drift** — verifier is no longer left visually stranded in `pending` when executor completed without write-like actions:
  - `src-tauri/src/ai/agent/supervisor.rs`
- **Chat bubble overflow/regression path** — long markdown tables/code blocks are now contained instead of stretching the transcript layout:
  - `src/components/agent-chat/MarkdownRenderer.tsx`
  - `src/components/agent-chat/MessageBubble.tsx`
  - `src/components/agent-chat/AgentChatPanel.tsx`

### Changed

- `max_tokens_per_day` removed from `AirlockRateLimits` everywhere — field was unimplementable and misleading:
  - `src-tauri/src/ai/specs/manifest.rs`
  - `src/types/airlock.ts`
  - `src/components/agents/builder/specDefaults.ts`
  - `src/components/agents/builder/airlock/RateLimitsSection.tsx` (removed "Tokens / Day" UI field)
- All 4 `@TODO` markers in `manifest.rs` resolved — `strategy`, `retention_days`, `persistence`, and `rate_limits` are now fully implemented
- **Supervisor runtime and chat rendering hardening** — reduced CPU churn and event spam across local multi-agent runs:
  - Rust now coalesces frontend-facing agent events before emitting them from `src-tauri/src/commands/agent.rs`
  - `src/hooks/useAgentChat.ts` batches runtime event application per animation frame instead of issuing a React state write per backend event
  - `src/components/agent-chat/MarkdownRenderer.tsx` disables syntax highlighting while streaming and uses render containment for large completed responses
  - `src/components/agent-chat/MessageBubble.tsx` and `src/components/neural/AirlockEvents.tsx` now surface safer layout/approval states
- **Supervisor mode product wiring** — runtime metadata, lane state, builder controls, and telemetry continue to be exposed end-to-end through:
  - `src-tauri/src/ai/agent/events.rs`
  - `src-tauri/src/ai/agent/protocol.rs`
  - `src-tauri/src/ai/agent/runtime_registry.rs`
  - `src-tauri/src/ai/agent/specialist.rs`
  - `src-tauri/src/services/command_poller.rs`
  - `src-tauri/src/services/neural_service.rs`
  - `src/components/agents/builder/AgentBuilder.tsx`
  - `src/components/agents/builder/RuntimePanel.tsx`
  - `src/components/agents/builder/specDefaults.ts`
  - `src/services/tauri.ts`
  - `src/types/agent.ts`
  - `src/types/neural.ts`

### Validation

- `cargo check -q` → pass (zero warnings)
- `cargo test -q memory --lib` → 21 tests pass
- `cargo test -q agent --lib` → 49 tests pass
- `pnpm exec tsc --noEmit` → pass
- `bun test` (rainy-atm/v0.1.16) → 50 tests pass
- `cargo test -q airlock --lib` → 11 tests pass
- `cargo test -q workflow --lib` → 9 tests pass
- `cargo test -q supervisor --lib` → 9 tests pass
- `cargo test -q agent --lib` → 52 tests pass
- `pnpm exec tsc --noEmit` → pass (chat transcript virtualization + user markdown bubble updates)
- `cargo check -q` → pass (chat transcript virtualization + user markdown bubble updates)
- `npx -y react-doctor@latest . --verbose --diff` → 100/100 (no issues found after final chat rendering adjustments)

### Fixed

- **CRITICAL** — Connector credentials were stored encrypted but read without decryption, causing all connectors (Telegram, WhatsApp) to receive ciphertext as bot tokens. Added `decrypt()` call in `ConnectorRegistry.getConnector()`. (`rainy-atm` v0.1.15)
- **SECURITY** — WebSocket auth accepted credentials in URL query params (leaked in server logs). Removed `authFromQuery` path; all connections now receive `{type:"auth_required"}` on connect and must send an `identify` message within 5 seconds or be closed with code 4001. (`rainy-atm`)
- **SECURITY** — Telegram webhook secret verification used string equality, vulnerable to timing attacks. Replaced with `timingSafeEqual()` from Node `crypto`. (`rainy-atm`)
- **SECURITY** — `tool_access_policy` received from cloud was applied without cryptographic verification. Rust `command_poller` now computes SHA-256 of the canonical JSON and rejects commands where the hash does not match the provided `tool_access_policy_hash`. (`src-tauri`)
- Tool execution audit write failures were silently swallowed. Failures now increment `rainy:audit:failures` in Redis for observability. (`rainy-atm`)

### Added

- Per-session rate limiting in `UnifiedLaneQueue` — 20 requests/minute per `{channel}:{workspaceId}:{sessionPeer}`, enforced after the existing workspace-level limit. (`rainy-atm`)
- WhatsApp lane queue (`whatsapp-lane-queue.ts`) and agent runtime (`whatsapp-agent-runtime.ts`) mirroring the Telegram/Discord pattern with Redis keys `rainy:wa:queue:` / `rainy:wa:lock:`. WhatsApp messages no longer silently fail after webhook ingestion. (`rainy-atm`)
- Telegram `callback_query` updates (inline keyboard presses) are now parsed by `TelegramConnector.parseWebhook()` into a `NormalizedMessage` with `text: cq.data` and `callbackQueryId` in metadata. Previously these were silently dropped. (`rainy-atm`)

### Changed

- `socket_client.rs` annotated with `@RESERVED` to document intentional HTTP-polling-first design while WS auth migration is pending. (`src-tauri`)
- **Memory system performance optimizations** — reduced latency and resource usage across the RAG loop (`src-tauri`):
  - `EmbedderService` is now cached in a `OnceLock` and pre-warmed during `MemoryManager::init()`, eliminating repeated Keychain + settings initialization on every search.
  - `touch_access` updates are now batched into a single transaction via `touch_access_batch()` in the vault repository, replacing N individual write queries per search result.
  - New memory entries are stored immediately (non-blocking write path), then embedded asynchronously via a background `tokio::spawn` task that upserts the embedding vector once available.
  - Semantic search now enforces a 4-second timeout on the Gemini embedding API call; timeouts fall back to lexical search with a descriptive reason logged in `SemanticSearchResult`.
  - Re-embedding backfill (`run_reembed_backfill`) now fetches unembedded IDs in pages of 500 using `LIMIT/OFFSET`, preventing unbounded memory usage on large vaults.

### Changed

- Renamed "Codex" to "MaTE" in the chat input placeholder to align with the application's actual name.
- Maintained version **0.5.96** as the active development release (postponed 0.5.97).
- Applied premium glassmorphism styling (`bg-background/20-30`, `backdrop-blur-md`, `border-white/10`) and refined hover states (`hover:bg-white/10`) across the application to align with **Section 20** design standards:
  - **AppSidebar**: Polished "Add First" button, all `NavItem` components, and footer action buttons.
  - **WasmSkillsPage**: Updated "Browse", "Install Local", and "Install From ATM" buttons.
  - **AgentBuilder**: Refined "Save Draft" button, Max Tokens input, and slider thumbs with `bg-white`, `shadow-lg`, and hover scale effects.
  - **AgentStorePage**: Polished "New Agent", "Refresh List", header actions, and agent list item hover states.
  - **ApiKeysTab**: Updated "Verify", "Lock in Vault", and key action buttons.
  - **Sidebar**: Redesigned for a more slender profile (`68px` collapsed width), reduced top margins (`mt-6`) for traffic light alignment, and downsized buttons/icons (`size-9`/`size-7`) for a more elegant aesthetic.
  - **Popovers**: Refined background opacities (`bg-background/20-30`) and blurs in Workspace Selector and Reasoning Effort dropdowns to avoid solid black appearance.

- Refactored `AgentChatPanel.tsx` for stability, fixing syntax errors and duplicate declarations.
- Aligned Chat UI with reference design: added growing dynamic input area, redesigned "Let's build" suggestion cards, and improved adaptive blurs (`backdrop-blur-2xl`).
- Replaced nested `Button` components in `TooltipTrigger` with `render` props and `buttonVariants` to resolve Base UI `asChild` TypeScript errors.
- Cleaned up the chat footer row for local model and reasoning effort selection.
- Refactored **Settings Page** to a premium macOS-inspired sidebar layout using Shadcn/UI:
  - Migrated all settings tabs (**Models, API Keys, Appearance, Permissions, Profile**) from HeroUI to Shadcn/UI.
  - Rebuilt `ThemeSelector` with high-fidelity glass cards and custom mode toggles.
  - Implemented `enableCompactMode` in `ThemeProvider` with persistent local storage.
  - Enhanced **API Keys** management with credential masking, one-click visibility, and real-time validation feedback.
  - Improved mobile responsiveness with a condensed horizontal tab navigation for small screens.
- Hardened Fleet Command Center sensitive-action UX with explicit HeroUI confirmation dialogs (typed phrase + acknowledge checkbox + submitting lock) for kill switch and node retire:
  - `src/components/neural/modules/FleetCommandCenter.tsx`
- Removed owner credential payload dependency from Fleet UI actions (desktop bridge now uses persisted secure owner context instead of passing keys from React):
  - `src/services/tauri.ts`
  - `src-tauri/src/commands/atm.rs`
  - `src-tauri/src/services/atm_client.rs`
- Added fleet node lifecycle hardening and safer operations:
  - idempotent node registration by fingerprint (avoids duplicate ghost nodes)
  - fleet status now includes `effectiveStatus` and retire metadata
  - retire endpoint with owner validation + audit emission + anti-flood protections
  - files:
    - `rainy-atm/src/routes/nodes.ts`
    - `rainy-atm/src/routes/admin-fleet.ts`
    - `rainy-atm/src/services/command-bridge.ts`
    - `rainy-atm/src/db/client.ts`
    - `rainy-atm/src/db/schema.ts`
- Hardened Cloud Build deploy reliability for registry push instability (`blob upload unknown`) with pull/push retries and removal of duplicate implicit image push:
  - `rainy-atm/cloudbuild.yaml`

### Fixed

- Fixed Fleet confirmation modal completion flow: dialog now closes correctly after successful action completion.
  - `src/components/neural/modules/FleetCommandCenter.tsx`
- Fixed Rainy ATM auto-connect regressions by serializing desktop node registration locally and surfacing deploy/transient registration states with clearer recovery messaging:
  - `src-tauri/src/services/neural_service.rs`
  - `src-tauri/src/services/command_poller.rs`
  - `src-tauri/src/services/atm_client.rs`
  - `src/components/neural/NeuralPanel.tsx`
- Fixed ATM startup recovery regression where instances could remain permanently `DB_NOT_READY` after transient DB outages by continuing background init retries until success.
  - `rainy-atm/src/index.ts`
- Fixed retired-node command finalization regression: retired nodes can still report progress/complete for already-running commands, preventing stranded `running` commands.
  - `rainy-atm/src/routes/nodes.ts`
- Fixed fleet last-active-node retirement guard to count both `online` and `busy` healthy nodes.
  - `rainy-atm/src/routes/admin-fleet.ts`

## [0.5.96] - 2026-03-11 - THE FORGE (AGENT FACTORY PRODUCTION)

### Added

- Added THE FORGE foundation command surface and services for local workflow-to-agent generation:
  - `start_workflow_recording`
  - `record_workflow_step`
  - `stop_workflow_recording`
  - `get_workflow_recording`
  - `get_active_workflow_recording`
  - `generate_agent_spec_from_recording`
  - `save_generated_agent`
  - `list_generated_agents`
  - `load_generated_agent`
  - files:
    - `src-tauri/src/services/workflow_recorder.rs`
    - `src-tauri/src/services/agent_library.rs`
    - `src-tauri/src/commands/workflow_factory.rs`
    - `src-tauri/src/lib.rs`
    - `src/services/tauri.ts`
- Added Forge synthesis and persistence regression tests for deterministic allowlist derivation, summary truncation, and local agent-library roundtrip:
  - `src-tauri/src/commands/workflow_factory.rs`
  - `src-tauri/src/services/agent_library.rs`
- Added Forge quality gate + validation command surface to prevent low-value specialist generation:
  - strict generation gate: minimum useful steps + tool-call presence
  - `validate_generated_agent` command for mandatory draft test scoring (`coverage`, `determinism`, `safety`)
  - files:
    - `src-tauri/src/commands/workflow_factory.rs`
    - `src-tauri/src/lib.rs`
    - `src/services/tauri.ts`
- Added Forge auto-capture integration for native runtime tool events during active recording sessions:
  - `src/hooks/useAgentChat.ts`
- Added desktop ATM workspace-sharing command surface for private agent import loops:
  - `list_atm_workspace_shared_agents`
  - `import_atm_workspace_shared_agent`
  - files:
    - `src-tauri/src/commands/atm.rs`
    - `src-tauri/src/services/atm_client.rs`
    - `src-tauri/src/lib.rs`
    - `src/services/tauri.ts`
- Added desktop ATM marketplace command surface to complete Forge publish/import loops:
  - `list_atm_marketplace_agents`
  - `publish_atm_marketplace_agent`
  - `import_atm_marketplace_agent`
  - files:
    - `src-tauri/src/commands/atm.rs`
    - `src-tauri/src/services/atm_client.rs`
    - `src-tauri/src/lib.rs`
    - `src/services/tauri.ts`

- Added default user-editable MCP JSON lifecycle commands and runtime import flow:
  - `get_or_create_default_mcp_json_config`
  - `save_default_mcp_json_config`
  - `import_mcp_servers_from_default_json`
  - files:
    - `src-tauri/src/commands/mcp.rs`
    - `src-tauri/src/lib.rs`
    - `src/services/tauri.ts`
    - `src/components/neural/modules/NeuralMcp.tsx`
- Added persistent long-chat history scope and hydration path for Agent Chat so local runs continue in one durable conversation:
  - `get_default_chat_scope`
  - `get_chat_history_window`
  - files:
    - `src-tauri/src/ai/agent/manager.rs`
    - `src-tauri/src/lib.rs`
    - `src-tauri/src/commands/agent.rs`
    - `src/services/tauri.ts`
    - `src/hooks/useAgentChat.ts`
    - `src/components/agent-chat/AgentChatPanel.tsx`
- Added dual-vector storage foundation for embedding profile migration with dedicated per-model vector rows:
  - `memory_vault_embedding_vectors` table + ANN index
  - files:
    - `src-tauri/src/services/memory_vault/repository.rs`
    - `src-tauri/src/services/memory_vault/service.rs`
    - `src-tauri/src/services/memory_vault/types.rs`
    - `src-tauri/src/services/memory_vault/profiles.rs`
- Added RAG telemetry chips wiring in Agent Chat message metadata:
  - `history`, `retrieval`, `embedding`
  - files:
    - `src/types/agent.ts`
    - `src/hooks/useAgentChat.ts`
    - `src/components/agent-chat/AgentChatPanel.tsx`
    - `src-tauri/src/ai/agent/runtime.rs`
- Added persistent rolling compaction state for long-chat continuity:
  - `chat_compaction_state` table + index
  - new command: `get_chat_compaction_state`
  - files:
    - `src-tauri/migrations/20260311113000_add_chat_compaction_state.sql`
    - `src-tauri/src/ai/agent/manager.rs`
    - `src-tauri/src/lib.rs`
    - `src/services/tauri.ts`
- Added hybrid memory retrieval reranking that merges semantic and lexical candidates and scores by semantic distance, lexical overlap, recency, and access frequency:
  - `src-tauri/src/services/memory/memory_manager.rs`
- Added Gemini batch embedding path (`batchEmbedContents`) for high-throughput document ingestion and memory backfill:
  - `src-tauri/src/services/embedder.rs`
  - `src-tauri/src/services/memory/memory_manager.rs`
  - `src-tauri/src/services/memory_vault/service.rs`
- Added optional Turso remote-replica mode for memory vault (local-first default remains unchanged):
  - env support: `RAINY_MEMORY_TURSO_URL`, `RAINY_MEMORY_TURSO_AUTH_TOKEN`, `RAINY_MEMORY_TURSO_SYNC_SECS`
  - file:
    - `src-tauri/src/services/memory_vault/repository.rs`

### Changed

- Changed fleet policy application semantics to persist full workspace policy state on desktop (not only policy version floor), enabling deterministic policy reuse across command executions:
  - `src-tauri/src/services/fleet_control.rs`
  - `src-tauri/src/services/settings.rs`
- Changed command execution path to inject persisted fleet policy into non-fleet commands when command payload does not include a policy envelope:
  - `src-tauri/src/services/command_poller.rs`
  - `src-tauri/src/services/skill_executor.rs`
- Changed Fleet Command Center UI to expose `currentAirlockPolicy` state and show dispatch acknowledgement summaries for policy push / kill switch:
  - `src/components/neural/modules/FleetCommandCenter.tsx`
  - `src/services/tauri.ts`
  - `src-tauri/src/services/atm_client.rs`
- Changed Agent Chat UI to integrate Forge recording controls (`start`, `stop & generate`) and local generated-agent activation path:
  - `src/components/agent-chat/AgentChatPanel.tsx`
  - `src/services/tauri.ts`
- Changed Forge generation flow to draft-review-before-save with inline editable fields (`name`, `description`, `soul_content`) and explicit save/discard actions:
  - `src/components/agent-chat/AgentChatPanel.tsx`
- Changed Forge synthesis from generic prompt output to structured specialist playbook generation (`goal`, `ordered steps`, `decision rules`, `fallbacks`, `success criteria`) with deterministic allowlist mode:
  - `src-tauri/src/commands/workflow_factory.rs`
- Changed Forge Draft Review UX to enforce `Test Draft` pass before `Save & Activate`, and to show real-time recording quality counters (steps/useful/tools/decisions/errors):
  - `src/components/agent-chat/AgentChatPanel.tsx`
- Changed Forge runtime capture telemetry to include `decision`, `error`, and `retry` signals from native agent events:
  - `src/hooks/useAgentChat.ts`
  - `src-tauri/src/services/workflow_recorder.rs`
- Changed Forge tool-allowlist derivation to sanitize tool labels into a safe canonical subset before embedding in generated specs:
  - `src-tauri/src/commands/workflow_factory.rs`
- Changed Rainy ATM route wiring to mount private workspace-agent sharing APIs:
  - `rainy-atm/src/index.ts`
  - `rainy-atm/src/routes/workspace-agents.ts`
- Changed Rainy ATM root metadata version and marketplace response guards to keep Step 7 routes resilient under malformed persisted payloads:
  - `rainy-atm/src/index.ts`
  - `rainy-atm/src/routes/marketplace.ts`
- Changed Rainy ATM bridge integration surface with modular AgentBridge dispatch primitives for external channel adapters:
  - `rainy-atm/src/integrations/agent-bridge.ts`
- Changed versioning for THE FORGE release:
  - `package.json` -> `0.5.96`
  - `src-tauri/Cargo.toml` -> `0.5.96`
  - `src-tauri/tauri.conf.json` -> `0.5.96`

- Switched MCP transport support to `stdio + http` and retained compatibility for legacy persisted `"sse"` tags via serde alias mapping:
  - `src-tauri/src/services/mcp_service.rs`
- Reworked MCP Neural panel to JSON-first management (visual JSON editor + validate/save/run), removing manual server-creation dependence:
  - `src/components/neural/modules/NeuralMcp.tsx`
- Updated local agent workflow wiring so `run_agent_workflow` accepts optional `chat_scope_id` and defaults to the persistent global scope:
  - `src-tauri/src/commands/agent.rs`
  - `src/services/tauri.ts`
  - `src/hooks/useAgentChat.ts`
- Updated memory retrieval/embedding flow to prioritize `gemini-embedding-2-preview` with safe fallback behavior and model-aware vector selection:
  - `src-tauri/src/services/embedder.rs`
  - `src-tauri/src/services/memory/memory_manager.rs`
  - `src-tauri/src/ai/agent/memory.rs`
  - `src-tauri/src/services/settings.rs`
- Updated agent workflow with automatic long-context compression at `80k` estimated tokens:
  - model-generated rolling summary persisted per `chat_scope_id`
  - keeps recent turns uncompressed and injects durable session summary in history
  - emits `CONTEXT_COMPACTION:{...}` status event for UI visibility
  - files:
    - `src-tauri/src/commands/agent.rs`
    - `src-tauri/src/ai/agent/manager.rs`
- Updated polling cadence to reduce server load:
  - active loop: `2s`
  - idle loop: `10s`
  - files:
    - `src-tauri/src/services/command_poller.rs`
- Updated memory runtime wiring to use one canonical `MemoryManager` path across local and polled agent execution, eliminating split retrieval/store behavior:
  - `src-tauri/src/ai/agent/memory.rs`
  - `src-tauri/src/ai/agent/runtime.rs`
  - `src-tauri/src/ai/agent/workflow.rs`
  - `src-tauri/src/commands/agent.rs`
  - `src-tauri/src/services/command_poller.rs`
  - `src-tauri/src/lib.rs`
  - `src-tauri/src/services/skill_executor.rs`
- Updated memory write flow to transactional atomic upsert for encrypted entries + vector rows:
  - `src-tauri/src/services/memory_vault/repository.rs`
  - `src-tauri/src/services/memory_vault/service.rs`
- Updated memory command search API to enforce workspace-scoped retrieval:
  - `src-tauri/src/commands/memory.rs`

### Fixed

- Fixed fleet kill-switch command completion semantics to return SLA-aware acknowledgement payloads based on active runtime counts within a 5-second budget, instead of unconditional success:
  - `src-tauri/src/services/command_poller.rs`
- Fixed workflow recorder robustness by adding malformed-step validation (allowed kinds, non-empty labels), payload-size limits, step limits, and bounded history retention:
  - `src-tauri/src/services/workflow_recorder.rs`

- Fixed MCP runtime removal lifecycle to consistently detach live connections using sanitized server keys.
- Fixed MCP disconnection/removal lifecycle to explicitly terminate spawned stdio subprocesses (including replacement on reconnect with same key), preventing orphan process leaks and hidden active tools.
  - file:
    - `src-tauri/src/services/mcp_service.rs`
- Fixed noisy heartbeat failure reporting for temporary upstream gateway errors by sanitizing HTML response bodies and classifying 502/503/504 retries as transient:
  - `src-tauri/src/services/neural_service.rs`
  - `src-tauri/src/services/command_poller.rs`
- Fixed status event chatter by consolidating RAG telemetry emission to a single per-run snapshot instead of repeated updates:
  - `src-tauri/src/ai/agent/runtime.rs`
  - `src/hooks/useAgentChat.ts`
- Fixed chat UX visibility gap for context compression by surfacing a best-practice auto-compression indicator in Agent Chat:
  - `src/components/agent-chat/AgentChatPanel.tsx`
  - `src/types/agent.ts`
- Fixed ambiguous `RETRIEVAL: UNKNOWN` chip states by normalizing retrieval fallback to `unavailable` and persisting chat-scoped runtime telemetry for hydration:
  - `src-tauri/migrations/20260311130000_add_chat_runtime_telemetry.sql`
  - `src-tauri/src/ai/agent/manager.rs`
  - `src-tauri/src/commands/agent.rs`
  - `src/hooks/useAgentChat.ts`
  - `src/components/agent-chat/AgentChatPanel.tsx`
- Fixed memory DB directory drift so vault initialization respects `memory_db` path instead of writing to app-data parent.
  - `src-tauri/src/services/memory/memory_manager.rs`
- Fixed global search leakage by removing implicit `"global"` query fallback and requiring workspace-scoped memory search.
  - `src-tauri/src/services/memory/memory_manager.rs`
  - `src-tauri/src/services/skill_executor.rs`
  - `src-tauri/src/commands/memory.rs`
- Fixed partial consistency risk where encrypted memory rows could succeed while vector rows failed silently by enforcing transactional atomicity.
  - `src-tauri/src/services/memory_vault/repository.rs`
  - `src-tauri/src/services/memory_vault/service.rs`
- Fixed incomplete context reset so `memory_vault_embedding_vectors` is also cleared with workspace history reset.
  - `src-tauri/src/ai/agent/manager.rs`
- Fixed full-history reset consistency so workspace memory is cleared through the active `MemoryManager`/vault path (same DB used by runtime writes), avoiding stale RAG context after reset.
  - `src-tauri/src/commands/agent.rs`
  - `src-tauri/src/services/memory/memory_manager.rs`
  - `src-tauri/src/services/memory_vault/service.rs`
  - `src-tauri/src/services/memory_vault/repository.rs`
  - `src-tauri/src/ai/agent/manager.rs`
- Fixed model-specific embedding metadata drift by removing silent fallback from model-bound batch embedding and using strict per-model embedding in backfill paths.
  - `src-tauri/src/services/embedder.rs`
  - `src-tauri/src/services/memory/memory_manager.rs`
- Fixed confidential-memory access-control bypass in semantic retrieval injection by reapplying L2 Airlock filtering before system-prompt memory injection.
  - `src-tauri/src/ai/agent/runtime.rs`
  - `src-tauri/src/services/memory/memory_manager.rs`
  - `src-tauri/src/services/memory/types.rs`
- Fixed dead-code/warning regressions introduced by memory refactor by removing unused methods/constants and cfg-sensitive unused imports.
  - `src-tauri/src/services/embedder.rs`
  - `src-tauri/src/ai/agent/workflow.rs`
  - `src-tauri/src/services/memory_vault/repository.rs`
- Fixed Forge agent-library path traversal risk by enforcing strict slug validation for `AgentSpec.id` before save/load path joins (blocks `../` and any non `[a-zA-Z0-9_-]` filename components):
  - `src-tauri/src/services/agent_library.rs`
- Fixed Forge draft-save crash vectors by requiring save-ready specialist fields (`id`, `soul.name`, `soul_content`, allowlist mode, non-empty allowed tools) at command boundary:
  - `src-tauri/src/commands/workflow_factory.rs`
- Fixed Forge recording UX deadlock by decoupling stop and generate actions so active recordings can always be stopped/canceled even when quality gate is not yet satisfied:
  - `src/components/agent-chat/AgentChatPanel.tsx`

### Validation

- `cd src-tauri && cargo check -q` — passes
- `pnpm exec tsc --noEmit` — passes
- `cd src-tauri && cargo test -q airlock --lib` — passes
- `cd src-tauri && cargo test -q agent --lib` — passes
- `cd src-tauri && cargo test -q manifest_covers_every_registered_tool --lib` — passes
- `cd src-tauri && cargo test -q every_registered_tool_has_explicit_policy_entry --lib` — passes
- `cd src-tauri && cargo test -q workflow_recorder --lib` — passes
- `cd src-tauri && cargo test -q workflow_factory --lib` — passes
- `cd src-tauri && cargo test -q agent_library --lib` — passes
- `cd src-tauri && cargo test -q workflow_recorder --lib` — passes (post recorder guardrails)
- `cd src-tauri && cargo test -q workflow_factory --lib` — passes (post generated-spec helper + tests)
- `cd src-tauri && cargo test -q agent_library --lib` — passes (post persistence roundtrip tests)
- `cd rainy-atm && bunx tsc --noEmit` — passes
- `cd rainy-atm && bun test` — passes (45/45)
- `cd rainy-atm && bun run build` — passes
- `cd src-tauri && cargo check -q` — passes (post workspace-agent import commands)
- `pnpm exec tsc --noEmit` — passes (post workspace-agent import wrappers)
- `cd src-tauri && cargo test -q agent --lib` — passes (post workspace-agent import commands)
- `cd src-tauri && cargo test -q airlock --lib` — passes (post workspace-agent import commands)
- `cd src-tauri && cargo test -q workflow --lib` — passes
- `cd src-tauri && cargo test -q agent --lib` — passes
- `cd src-tauri && cargo test -q memory_vault --lib` — passes
- `cd src-tauri && cargo test -q agent --lib` — passes (post hybrid/batch/Turso memory updates)
- `cd src-tauri && cargo test -q memory_vault --lib` — passes (post hybrid/batch/Turso memory updates)
- `cd src-tauri && cargo test -q memory_vault --lib` — passes (post audit memory reset alignment fix)
- `cd src-tauri && cargo test -q agent --lib` — passes (post audit retrieval injection hardening)
- `cd src-tauri && cargo test -q airlock --lib` — passes (post audit confidential-memory L2 gating fix)
- `cd src-tauri && cargo check -q` — passes (post ATM marketplace desktop command surface)
- `pnpm exec tsc --noEmit` — passes (post ATM marketplace desktop command wrappers)
- `cd rainy-atm && bunx tsc --noEmit` — passes (post marketplace API hardening)
- `cd rainy-atm && bun test` — passes (45/45) (post marketplace API hardening)
- `cd rainy-atm && bun run build` — passes (post marketplace API hardening)
- `cd rainy-atm && bunx tsc --noEmit` — passes (post AgentBridge module)
- `cd rainy-atm && bun test` — passes (47/47) (post AgentBridge module)
- `cd rainy-atm && bun run build` — passes (post AgentBridge module)
- `cd src-tauri && cargo test -q agent_library --lib` — passes (4/4) (post path-traversal fix)
- `cd src-tauri && cargo check -q` — passes (release readiness revalidation)
- `pnpm exec tsc --noEmit` — passes (release readiness revalidation)
- `cd rainy-atm && bunx tsc --noEmit` — passes (release readiness revalidation)
- `cd rainy-atm && bun test` — passes (47/47) (release readiness revalidation)
- `cd rainy-atm && bun run build` — passes (release readiness revalidation)
- `pnpm run build` — passes (release readiness revalidation; non-blocking bundle-size warning from Vite reporter)
- `cd src-tauri && cargo test -q agent_library --lib` — passes (4/4) (release readiness revalidation)
- `cd src-tauri && cargo test -q workflow_factory --lib` — passes (5/5) (post Forge value-upgrade gates + validation)
- `cd src-tauri && cargo check -q` — passes (post Forge value-upgrade gates + validation)
- `pnpm exec tsc --noEmit` — passes (post Forge value-upgrade UX + command wrappers)
- `pnpm run build` — passes (post Forge value-upgrade UX + command wrappers; non-blocking bundle-size warning)
- `cd src-tauri && cargo test -q workflow_recorder --lib` — passes (2/2) (post Forge signal capture expansion)
- `cd src-tauri && cargo test -q agent_library --lib` — passes (4/4) (post Forge polish regression revalidation)
- `pnpm exec tsc --noEmit` — passes (post Forge stop/generate UX decoupling fix)
- `cd src-tauri && cargo check -q` — passes (post Forge stop/generate UX decoupling fix)
- `pnpm run build` — passes (post Forge stop/generate UX decoupling fix; non-blocking bundle-size warning)

## [0.5.95] - 2026-03-10 - NERVE CENTER STEP 6 STABILITY PATCH

### Fixed

- Fixed Tauri startup panic caused by TaskManager state type mismatch (`TaskManager` vs `Arc<TaskManager>`) by aligning managed state and command state extraction:
  - `src-tauri/src/lib.rs`
  - `src-tauri/src/commands/task.rs`
- Fixed Tokio nested-runtime panic (`Cannot start a runtime from within a runtime`) by removing `tauri::async_runtime::block_on(...)` from tool-definition discovery and making tool fetch fully async:
  - `src-tauri/src/services/skill_executor/registry.rs`
  - `src-tauri/src/ai/agent/runtime.rs`
  - `src-tauri/src/ai/agent/workflow.rs`
- Fixed stale workflow test harness constructor drift after `SkillExecutor::new(...)` gained `Arc<McpService>`:
  - `src-tauri/src/ai/agent/workflow.rs`
- Stabilized supervisor orchestration determinism by enforcing dependency-aware specialist sequencing, dependency-context handoff, and assignment-order summary output:
  - `src-tauri/src/ai/agent/supervisor.rs`

### Changed - Versioning

- `package.json` -> `0.5.95`
- `src-tauri/Cargo.toml` -> `0.5.95`
- `src-tauri/tauri.conf.json` -> `0.5.95`

### Validation

- `cd src-tauri && cargo check -q` — passes
- `cd src-tauri && cargo test -q workflow --lib` — passes
- `cd src-tauri && cargo test -q agent --lib` — passes
- `pnpm exec tsc --noEmit` — passes

## [0.5.95] - 2026-03-06 - NERVE CENTER (FLEET COMMAND CENTER)

### Added

- Added cooperative fleet kill-switch runtime cancellation in:
  - `src-tauri/src/ai/agent/workflow.rs`
  - `src-tauri/src/ai/agent/runtime.rs`
  - `src-tauri/src/ai/agent/specialist.rs`
  - `src-tauri/src/ai/agent/supervisor.rs`
- Added immediate websocket kill-signal arming path in `src-tauri/src/lib.rs` so `fleet_kill_switch` broadcasts trigger local cancellation without waiting for command polling order.
- Added richer fleet runtime audit emission from cloud-triggered agent runs in `src-tauri/src/services/command_poller.rs`:
  - tool execution events
  - tool result outcomes
  - airlock denial/blocked decisions
- Added fleet status API policy snapshot fields (`mode`, `enabled`, `version`, `hash`) in `rainy-atm/src/routes/admin-fleet.ts`.

### Changed

- Updated ATM fleet dispatch orchestration in `rainy-atm/src/services/fleet-kill-switch.ts`:
  - added 5-second dispatch acknowledgement tracking
  - return per-command status summary (`queued/pending/approved/running/completed/failed/rejected/timeout`)
  - sync `fleet_dispatch_log` to observed command state.
- Updated node command lifecycle handlers in `rainy-atm/src/routes/nodes.ts` to move `fleet_dispatch_log` through `running` and terminal statuses on `start`/`complete`.
- Updated desktop `CommandPoller` to expose `arm_kill_switch()` and use it for both direct fleet commands and websocket-triggered kill events.
- Updated ATM models to exclusively use `gpt-5-nano` (basic) and `inception/mercury-2` (advanced) via OpenRouter/Rainy-SDK.
- Restricted the ATM agent creation form (`CreateAgentForm.tsx`) to only display and use these models.
- Documented these new backend capabilities in types configuration.

### Fixed

- Fixed graceful termination semantics for active agent sessions under fleet kill-switch conditions by persisting a termination assistant message and allowing normal history/memory persistence path.
- Fixed fleet command audit completeness by emitting blocked Airlock decisions when command-level approvals are denied.

### Changed - Versioning

- `package.json` -> `0.5.95`
- `src-tauri/Cargo.toml` -> `0.5.95`
- `src-tauri/tauri.conf.json` -> `0.5.95`

### Validation

- `cd src-tauri && cargo check -q` — passes
- `cd src-tauri && cargo test -q` — passes (116/116)
- `pnpm exec tsc --noEmit` — passes
- `pnpm run build` — passes
- `cd rainy-atm && bunx tsc --noEmit` — passes
- `cd rainy-atm && bun test` — passes (43/43)
- `cd rainy-atm && bun run build` — passes

## [0.5.94] - 2026-03-04 - THE DIRECTOR

### Added

- Added formal BYOK instability issue record with repro, impact, mitigations, and next milestones:
  - `ISSUES/BYOK_GEMINI_TOOLCALLING_2026-03-05.md`
- Added model label rendering under agent chat bubbles so each assistant response shows the model used.
- Added canonical model catalog module in `src-tauri/src/ai/model_catalog.rs` as a single Rust source of truth for:
  - supported model slugs
  - provider ownership (`rainy_api` vs `gemini_byok`)
  - model capabilities and thinking metadata
- Added strict obsolete-slug guardrails to reject deprecated flash slugs globally (`gemini-2.5-flash`, `gemini-2.5-flash-lite`).
- Added `GeminiProviderFactory` in `src-tauri/src/ai/providers/gemini_adapter.rs` and wired it into provider exports for first-class provider registration.
- Added first-pass Fleet Command Center UI module in `src/components/neural/modules/FleetCommandCenter.tsx` with:
  - node health/status cards
  - fleet policy push action
  - fleet kill switch dispatch action
- Added Fleet tab wiring in neural UI:
  - `src/components/neural/layout/NeuralSidebar.tsx`
  - `src/components/neural/NeuralPanel.tsx`
- Added desktop fleet API command wrappers in:
  - `src-tauri/src/commands/atm.rs`
  - `src/services/tauri.ts`
  - `src-tauri/src/services/atm_client.rs`
- Added modular fleet runtime services in desktop core:
  - `src-tauri/src/services/fleet_control.rs`
  - `src-tauri/src/services/agent_kill_switch.rs`
  - `src-tauri/src/services/audit_emitter.rs`
- Added modular Supervisor runtime building blocks in:
  - `src-tauri/src/ai/agent/events.rs`
  - `src-tauri/src/ai/agent/protocol.rs`
  - `src-tauri/src/ai/agent/runtime_registry.rs`
  - `src-tauri/src/ai/agent/specialist.rs`
  - `src-tauri/src/ai/agent/supervisor.rs`
- Added `runtime` configuration to `AgentSpec` in `src-tauri/src/ai/specs/manifest.rs` with `single` and `supervisor` modes, bounded specialist count, and verifier gating.
- Added first-class specialist roles:
  - `ResearchAgent`
  - `ExecutorAgent`
  - `VerifierAgent`
- Added Supervisor event emission for real-time plan/status visibility:
  - `supervisor_plan_created`
  - `specialist_spawned`
  - `specialist_status_changed`
  - `specialist_completed`
  - `specialist_failed`
  - `supervisor_summary`

### Fixed

- Fixed deterministic provider routing so explicit model prefixes are authoritative:
  - `rainy:` / `rainy-api/` now pin to Rainy provider.
  - `gemini:` now pins to Gemini BYOK provider.
- Fixed Rainy v3/OpenRouter model ownership so namespaced catalog slugs like `openai/gpt-5-nano`, `anthropic/...`, and `google/...` no longer fall through to Gemini BYOK routing.
- Fixed native `run_agent_workflow` provider bootstrap in `src-tauri/src/commands/agent.rs` so Rainy-pinned models synchronously register `rainy_api` before the first ThinkStep instead of failing with generic `No providers available`.
- Fixed Rainy credential resolution in `src-tauri/src/ai/provider.rs` and `src-tauri/src/commands/agent.rs` to ignore residual `ra-cowork` / `cowork_api` legacy paths and accept only current `ra-` Rainy API keys.
- Fixed Rainy `responses` tool-call failures for GPT-5-family models by routing tool-bearing turns through `chat.completions` while keeping `responses` for non-tool turns:
  - `src-tauri/src/ai/providers/rainy_sdk.rs`
  - `rainy-atm/src/services/rainy-runtime.ts`
- Fixed Rainy `chat.completions` tool-choice serialization in `src-tauri/src/ai/providers/rainy_sdk.rs` by omitting `ToolChoice::Auto` / `ToolChoice::None`, avoiding `null` payloads that Rainy rejected with `400 invalid_union`.
- Fixed Gemini BYOK multi-turn tool continuity by serializing assistant tool calls and tool results using Gemini-native parts (`function_call` + `function_response`) instead of flattening tool messages into plain text.
- Fixed empty final-response regressions by adding backend and frontend guardrails when a run ends with empty assistant text.
- Fixed local `run_agent_workflow` Airlock bypass by injecting the initialized `AirlockService` state into `AgentRuntime` instead of running with `None`.
- Restored effective permission gating for tool execution in local native agent loops (Airlock checks now execute for local chat runs too).
- Fixed startup-time `libsql` panic caused by threading initialization race with `sqlx` SQLite pools:
  - `assertion left == right failed` in `libsql::local::database` with poisoned `Once`.
- Added deterministic early `libsql` warm-up in app setup before any `sqlx` pool initialization to enforce safe initialization order.
- Fixed Gemini BYOK request model normalization so Google API receives clean model IDs (for example `gemini-3.1-flash-lite-preview`) instead of prefixed IDs like `gemini:gemini-3.1-flash-lite-preview`.
- Prevented invalid Gemini URLs like `models/gemini:...` that caused `404 NOT_FOUND` responses in runtime chat.
- Normalized incoming model IDs at router command boundaries to avoid prefixed model propagation through runtime.
- Enforced Airlock checks for local tool execution inside agent ActStep before tool dispatch in `src-tauri/src/ai/agent/workflow.rs`.
- Improved specialist runtime status fidelity:
  - emits `waiting_on_airlock` when approval gates are active
  - emits verifier `verifying` phase
  - files: `src-tauri/src/ai/agent/specialist.rs`, `src-tauri/src/ai/agent/supervisor.rs`
- Fixed Supervisor terminal semantics so runs with failed specialist lanes are no longer reported as completed in `src-tauri/src/ai/agent/supervisor.rs`.
- Aligned desktop websocket wake-up handling for both `command_queued` and legacy `new_command` events in `src-tauri/src/lib.rs`.
- Wired fleet audit queue flush on command completion in `src-tauri/src/services/command_poller.rs` via `AuditEmitter -> ATMClient`.
- Closed the ATM-hosted Wasm skill installation trust chain by switching desktop remote installs to verify `ed25519` bundle signatures against an ATM-served public key instead of the workspace platform key.
- Hardened `src-tauri/src/services/neural_service.rs` heartbeat behavior so runtime skill manifests are re-signed and refreshed whenever the manifest hash changes, keeping the node advertisement aligned with installed/enabled third-party skills.
- Aligned Wasm sandbox resource limits in `src-tauri/src/services/wasm_sandbox/mod.rs` with the production Step 4 profile by reducing the per-instance memory ceiling to `50 MB`.
- Fixed keychain-dependent Rust tests to run deterministically in the local test environment by adding a test-only in-memory keychain fallback in `src-tauri/src/ai/keychain.rs`.
- Tightened the built-in tool policy regression in `src-tauri/src/services/skill_executor/registry.rs` so the explicit-policy invariant validates registered built-ins without being polluted by installed third-party skills.

### Changed

- Replaced the legacy Rainy direct HTTP bridge in `src-tauri/src/ai/providers/rainy_sdk.rs` with the native `rainy-sdk 0.6.11` OpenAI chat replay surface:
  - `create_openai_chat_completion`
  - `create_openai_chat_completion_stream`
  - native replay of assistant `tool_calls`, `tool` messages, multimodal content, and provider metadata
- Refactored Rainy transport selection to operate on the full request, not just the model slug:
  - GPT-5 / O3 / O4 models prefer `responses`
  - turns that advertise local tools fall back to `chat.completions`
- Refactored `rainy-atm/src/services/rainy-runtime.ts` to preserve original Rainy/OpenRouter model slugs instead of rewriting or de-prefixing them during execution.
- Rainy provider capabilities now prefer live v3 catalog metadata from `RainyClient::get_models_catalog()` instead of a fixed hardcoded list.
- `src-tauri/src/ai/provider.rs`
  - Rainy model discovery now prefers live v3 catalog entries, then falls back to `list_available_models()`, then local static defaults.
  - added in-memory catalog cache with invalidation on Rainy API key changes to avoid hammering `/api/v1/models/catalog`.
- `src-tauri/src/commands/unified_models.rs`
  - unified model selector now augments static Rainy entries with dynamically discovered Rainy v3 catalog models when an API key is present.
- `src-tauri/src/services/settings.rs`
  - settings model picker now augments static Rainy entries with dynamically discovered Rainy v3 catalog models.
- Updated `src-tauri/Cargo.toml` / `src-tauri/Cargo.lock` to consume `rainy-sdk 0.6.11`.
- Updated `rainy-sdk` dependency to latest available stable release:
  - `src-tauri/Cargo.toml`: `rainy-sdk = 0.6.10`
  - `src-tauri/Cargo.lock`: resolved `rainy-sdk 0.6.10`
- `src-tauri/src/commands/agent.rs`
  - runtime now preserves full `model_id` (including provider prefix) instead of stripping it.
  - native agent runtime sets `streaming_enabled: false` by default.
- `src-tauri/src/ai/router/router.rs`
  - provider selection now honors explicit prefixes before heuristic capability routing.
- `src-tauri/src/ai/agent/runtime.rs`
  - added `RuntimeOptions.streaming_enabled`.
  - ThinkStep wiring now supports explicit streaming opt-in.
  - final response selection now falls back to latest non-empty assistant output.
- `src-tauri/src/ai/agent/workflow.rs`
  - ThinkStep streaming path is now gated by `allow_streaming` and remains non-streaming for tool turns.
  - added post-tool empty-response recovery pass that forces a final plain-text answer with tools disabled.
- `src-tauri/src/ai/providers/gemini_adapter.rs`
  - added Gemini-native `function_response` support.
  - adjusted `functionResponse` history serialization to `role: user` for manual `generateContent` compatibility in BYOK flows.
  - wrapped function responses under `response.result` for Gemini manual orchestration compatibility.
  - preserved and rehydrated Gemini `thought_signature` metadata across assistant tool-call turns.
  - added native `function_calling_config` mapping from internal `tool_choice` to Gemini modes (`ANY/AUTO/NONE`) for deterministic BYOK tool execution.
  - added warning logs for empty Gemini assistant turns with raw parts payload context.
  - improved schema sanitizer for Gemini OpenAPI subset compatibility.
  - tool schema conversion now fail-fast on invalid root schema shapes.
- `src-tauri/src/ai/model_catalog.rs`
  - Gemini BYOK entries now advertise `function_calling: true`.
- `src/hooks/useAgentChat.ts`
  - added UI fallback text to prevent blank assistant bubbles on empty backend result.
- `src-tauri/src/commands/router.rs`
  - routing commands now preserve explicit prefixed model IDs.
- `src-tauri/src/services/command_poller.rs`
  - cloud runtime options now set `streaming_enabled: false` by default.
- `src-tauri/src/commands/agent.rs`
  - `run_agent_workflow` now receives `AirlockServiceState`.
  - runtime construction now passes the real shared Airlock service instance.
- `src/components/agent-chat/MessageBubble.tsx`
  - shows `message.modelUsed.name` below non-user bubbles alongside timestamp.
- `src-tauri/src/lib.rs`
  - setup now performs `libsql::Builder::new_local(\":memory:\").build().await` before DB and service initialization that may touch SQLite from `sqlx`.
- `src-tauri/src/ai/providers/gemini_adapter.rs`
  - `resolve_model_id` now strips provider prefixes before URL construction.
- `src-tauri/src/ai/gemini.rs`
  - legacy Gemini provider now normalizes prefixed model IDs before API mapping.
- `src-tauri/src/commands/router.rs`
  - `complete_with_routing` and `stream_with_routing` now normalize model IDs before dispatch.
- `src-tauri/src/commands/agent.rs`
  - runtime options now use normalized model IDs for agent execution.
- `src-tauri/src/ai/router/router.rs`
  - provider pinning recognizes `gemini:` prefixed model IDs as Gemini BYOK candidates.
- Replaced duplicated static model lists in `src-tauri/src/commands/unified_models.rs` with catalog-backed generation.
- Reworked `src-tauri/src/services/settings.rs` model listing to derive from the new Rust catalog.
- Updated default selected model in settings to `gemini-3-flash-preview`.
- Enabled `provider_type = google` in `src-tauri/src/commands/ai_providers.rs` via the new Gemini factory.
- Bootstrapped `rainy_api` and `gemini_byok` providers from keychain at startup in `src-tauri/src/lib.rs` and auto-added them to the router.
- Removed ad-hoc Gemini provider injection from `src-tauri/src/commands/agent.rs`; provider lifecycle is now centralized.
- Extended router streaming events in `src-tauri/src/commands/router.rs` with `thought` payload emission.
- Updated frontend streaming types/hooks to handle thought chunks:
  - `src/services/tauri.ts`
  - `src/hooks/useStreaming.ts`
  - `src/hooks/useAgentChat.ts`
- Updated `src/hooks/useAIProvider.ts` to auto-register `gemini_byok` (in addition to `rainy_api`) and add it to the router when enabled.
- Aligned Gemini provider capability model lists to current preview slugs:
  - `src-tauri/src/ai/providers/gemini_adapter.rs`
  - `src-tauri/src/ai/gemini.rs`
  - `src-tauri/src/ai/providers/rainy_sdk.rs`
- Updated `gemini-2.5-flash` to `gemini-3-flash-preview` across the Rust engine and frontend hooks/types.
- Updated `gemini-2.5-flash-lite` to `gemini-3.1-flash-lite-preview` across the Rust engine and frontend hooks/types.
- Updated model registry metadata, frontend selection logic, and thinking capability flags for the new preview series.
- `src-tauri/src/ai/agent/runtime.rs` now routes to the Supervisor layer when an agent spec declares `runtime.mode = supervisor`; existing agents continue to use the single-agent path by default.
- `src-tauri/src/commands/agent.rs` and `src-tauri/src/services/command_poller.rs` now inject a shared runtime registry so local and cloud-triggered agent runs can report Supervisor activity consistently.
- `src-tauri/src/models/neural.rs` and `src-tauri/src/services/neural_service.rs` now include `runtimeStats` in node heartbeats, exposing active supervisor runs, specialist counts, and tool usage by role.
- Updated the chat UI to visualize Supervisor execution rails and per-specialist status in:
  - `src/hooks/useAgentChat.ts`
  - `src/components/agent-chat/MessageBubble.tsx`
  - `src/types/agent.ts`
  - `src/hooks/useAgentRuntime.ts`

### Validation

- `cd src-tauri && cargo check -q` — passes (Rainy v3 routing / credential / transport fixes)
- `pnpm exec tsc --noEmit` — passes (frontend + ATM type integrity after Rainy v3 changes)
- `cd rainy-atm && bunx tsc --noEmit` — passes
- `cd rainy-atm && bun run build` — passes
- `cd rainy-atm && bun test` — passes (43/43)
- `cd src-tauri && cargo check -q` - passes
- `cd src-tauri && cargo check -q` — passes
- `cd src-tauri && cargo test -q agent::workflow::tests::test_workflow_execution --lib` - passes
- `cd src-tauri && cargo test -q agent::workflow::tests::test_workflow_execution --lib` — passes
- `cd src-tauri && cargo test -q router::router::tests --lib` - passes
- `cd src-tauri && cargo test -q router::router::tests --lib` — passes
- `cd src-tauri && cargo test -q test_streaming_event_serialization --lib` — passes
- `cd src-tauri && cargo test -q` — passes (116/116)
- `pnpm exec tsc --noEmit` - passes
- `pnpm exec tsc --noEmit` — passes
- `pnpm run build` — passes

## [0.5.93] - QUARANTINE ZONE (WASM Skill Sandbox) - 2026-02-25

### Fixed - 0.5.93 Stabilizations

- Migrated Wasm Skill installers to verify `Ed25519` signatures instead of symmetric HMAC `sha256` for robust public-key cryptography payload verification.
- Hardened Wasmtime execution limits in `src-tauri/src/services/wasm_sandbox/mod.rs` by adding `wasmtime::ResourceLimiter` to clamp max memory per instance to strict limits (e.g., `< 50MB`).
- Fixed process-wide C-library lock panics (`SQLITE_MISUSE`) between `sqlx` and `libsql` statically-linked `libsqlite3-sys` in the `cargo test` suite by explicitly enforcing `libsql` global state initialization before `sqlx` connection pooling across test modules.

### Added - Third-Party Wasm Skill Registry + Installer Foundation

**Rust Backend (`src-tauri/src/`)**

- Added modular third-party skill persistence and metadata registry in `services/third_party_skill_registry.rs`.
- Added `services/skill_installer/` to parse `skill.toml`, verify Wasm SHA-256, enforce built-in-domain collision checks, and persist installed packages in the local app data directory.
- Added `services/wasm_sandbox/` execution host service with concurrency limits, Wasm binary validation, and a deny-first capability model (filesystem/network permissions remain fail-closed until host capability bindings are enabled).
- Added Wasmtime/wasmtime-wasi runtime integration for QUARANTINE ZONE basic WASI execution (JSON stdin envelope with `method` + `params`, captured stdout/stderr, fuel limit, stack limit, bounded stdio).
- Added WASI filesystem capability support via `WasiCtxBuilder::preopened_dir` with manifest permission-mode mapping (`read`, `read_write`) and fail-closed validation for invalid modes/paths.
- Added fail-closed Wasm execution timeout enforcement (`spawn_blocking` + bounded timeout) so sandboxed skills cannot hang the desktop runtime indefinitely.
- Added Wasm module compilation caching (shared Wasmtime engine + cached compiled modules by binary SHA-256) to improve repeat skill execution latency.
- Hardened Step 4 skill lifecycle semantics:
  - installer rejects third-party methods that collide with built-in tool method names
  - skill removal now deletes the installed package directory from disk (not only the registry entry)
- Added host-mediated network capability support for Wasm skills via declarative `networkRequests` prefetch in the WASI input envelope, with strict domain allowlist enforcement and SSRF/private-IP blocking.
- Added Tauri commands for skill management:
  - `list_installed_skills`
  - `install_local_skill`
  - `install_skill_from_atm`
  - `set_installed_skill_enabled`
  - `remove_installed_skill`

### Changed - Runtime Manifest + Skill Routing

**Rust Backend (`src-tauri/src/`)**

- `tool_manifest.rs` now merges installed third-party skill manifests into the runtime-generated node skill manifest (built-ins remain canonical + fail-closed).
- `SkillExecutor` now resolves unknown skill domains against the third-party skill registry and routes them to the Wasm sandbox service (currently fail-closed execution for undeployed ABI).
- `SkillExecutor` third-party calls now execute in the Wasm sandbox for skills that declare no network capabilities; filesystem capabilities are enforced via WASI preopened directories at runtime.
- Network capabilities remain fail-closed until explicit domain-checked host bindings are implemented in the Wasm runtime.
- Added third-party skill pre-execution policy enforcement in `SkillExecutor`:
  - command Airlock level must satisfy installed method minimum level
  - declared filesystem/domain permissions must fit within current command Airlock scopes
- `AirlockService` now recognizes installed third-party skill methods for policy presence/effective level calculation instead of denying them as unknown tools.
- Updated `tool_manifest` regression test to assert complete built-in coverage while allowing dynamic third-party skills.

### Changed - Desktop UI (Neural Settings)

**Frontend (`src/components/neural/modules/NeuralSettings.tsx`)**

- Added a “Wasm Skill Sandbox” management section showing installed skills, trust state, permissions, methods, enable/disable toggles, local install, ATM install, and remove actions.
- Added Tauri service wrappers in `src/services/tauri.ts` for the new skill-management commands.
- Updated neural state/tool display mappings for skill management actions in `src/components/agent-chat/neural-config.ts`.

### Changed - STEP 3 Production Gate Revalidation

- Revalidated Step 3 (`HIVE MIND SEED`) baseline before Step 4 integration:
  - semantic retrieval + context window tests still pass
  - `cargo check`, `tsc`, and ATM build/tests remain green after Step 4 foundation wiring
- Documented that current Step 3 retrieval remains Gemini-embedding-backed with lexical fallback when embedding credentials are unavailable.

### Validation

- `pnpm exec tsc --noEmit` — passes
- `cd src-tauri && cargo check -q` — passes
- `cd src-tauri && cargo test -q third_party_skill_registry::tests --lib` — passes (2/2)
- `cd src-tauri && cargo test -q wasm_sandbox::tests --lib` — passes (2/2)
- `cd src-tauri && cargo test -q manifest_covers_every_registered_tool --lib` — passes
- `cd src-tauri && cargo test -q context_window --lib` — passes
- `cd rainy-atm && bun run build` — passes
- `cd rainy-atm && bun test` — passes (42/42)

## [0.5.92] - 2026-02-21 - HIVE MIND SEED (Vector Knowledge Graph)

### Added - Knowledge Ingestion & Semantic Retrieval

**Rust Backend (`src-tauri/src/`)**

- Added `pdf-extract` (v0.7.0) to parse PDF documents natively.
- Implemented `ingest_document` level 0 tool in `skill_executor/filesystem.rs` to read plaintext, Markdown, and PDF documents.
- Added `ingest_text` and `search_semantic` to `MemoryManager`. The manager now automatically chunks large ingested texts (1500 chars limit), embeds them using `gemini-embedding-001`, and persists them into the AES-256-GCM encrypted `MemoryVaultService`.
- Wired the semantic retrieval into the `AgentRuntime`. The agent now silently searches the encrypted vault using the user's input before the ReAct `ThinkStep` and statically injects the top 5 most relevant semantic memory entries natively into its context window.

**Frontend & Tooling**

- Registered `ingest_document` in `registry.rs` and the frontend so agents can ingest user-provided documents directly into the workspace knowledge graph.

### Changed - STEP 3 Production Hardening (Gemini-Only 3072d)

**Rust Backend (`src-tauri/src/`)**

- Enforced Gemini-only embedding normalization for memory and agent memory embedding paths (`gemini-embedding-001`, `3072` dimensions) to prevent provider drift and dimension mismatches in STEP 3 semantic retrieval.
- Added structured semantic retrieval/injection metadata in `MemoryManager` (ANN vs exact vs lexical fallback) and bounded semantic memory prompt injection using the `ContextWindow` budget.
- Added libSQL ANN retrieval path (`vector_top_k`) in `memory_vault/repository.rs` with safe exact-search fallback (`vector_distance_cos`) if the vector index path is unavailable.
- Added a best-effort libSQL vector index creation path plus migration file `src-tauri/migrations/20260222090000_memory_vault_vector_ann_index.sql`.
- Hardened `ingest_document` with file-size checks and richer ingestion status reporting (chunks ingested/embedded + warnings).

### Validation

- `cd src-tauri && cargo check` — passes
- `cd src-tauri && cargo test context_window --lib` — passes (3/3)
- `cd src-tauri && cargo test memory_vault::repository::tests::test_libsql_direct_vector_api --lib` — passes (1/1)
- `pnpm exec tsc --noEmit` — passes
- `cd src-tauri && cargo test` — compiles and starts test run, but did not complete during this validation window (no failure captured)

## [0.5.91] - 2026-02-20 - DARK ARCHIVE (Memory V3) PT. 3

### Added - Gemini 3072-Dimension Vector Support

**Rust Backend (`src-tauri/src/services/`)**

- Added strict native `F32_BLOB(3072)` bounds to `memory_vault_entries_v3` inside `memory_vault/repository.rs`.
- Implemented global dimensionality runtime safety checks inside `MemoryVaultService::put`.
- Created an automatic background `run_reembed_backfill` process running at startup in `memory_vault/service.rs` to flawlessly re-embed legacy missing-dimension memory contents without downtime.
- Persisted vector provider origin contexts by saving `embedding_model`, `embedding_provider`, and `embedding_dim` locally during creation.

**Frontend (`src/components/settings/`)**

- Removed runtime embedding drop-down mutation toggles in `ModelsTab.tsx`, enforcing `gemini-embedding-001` to safely prevent 1536 parameter dimension-mismatch DB crashes natively.

## [0.5.91] - 2026-02-20 - DARK ARCHIVE (Polish & Hardening) PT. 2

### Added - Native Vector Similarity SQL via libSQL

- `services/memory_vault/repository.rs` completely migrated from standard `sqlx` to native `libsql` connections to unblock and support `vector_distance_cos` SQLite queries.
- Introduced unit testing for DB vector similarity execution over `F32_BLOB(1536)` types locally.

### Changed - Airlock Security Validations

- **AgentState** natively initializes and propagates `Arc<Option<AirlockService>>`.
- Enforced `Sensitive` (Level 1) gate restrictions before persisting/writing Memory Vault entries in `ActStep`.
- Enforced `Dangerous` (Level 2) gate restrictions before resolving `Confidential` Memories in `ThinkStep`.
- Migrated `run_plaintext_migration` schema iterators to execute purely over `libsql::Row` cursors.

### Fixed - Gemini Embedding Provider/Model State

- Fixed embedder provider selection parsing in settings UI so `"gemini"` is no longer truncated to `"g"` during persistence, which caused keychain lookups to fail with `Missing embedding API key for provider: g`.
- Added defensive provider normalization in memory/embedder runtime (`g|google|gemini -> gemini`, `oai|openai -> openai`) before resolving credentials and routing embedding requests.
- Updated Gemini embedding defaults to use `gemini-embedding-001` and added compatibility normalization for deprecated legacy Gemini embedding model IDs.
- Updated settings load path to migrate persisted deprecated Gemini embedding models to `gemini-embedding-001`.
- Updated settings model picker UI to remove deprecated Gemini embedding model options and keep the supported `gemini-embedding-001 (3072 dimensions)` option.

## [0.5.91] - 2026-02-19 - DARK ARCHIVE (Encrypted Memory Vault) - PT. 1

### Added - Encrypted Memory Vault (AES-256-GCM)

**Rust Backend (`src-tauri/src/services/`)**

- Added new modular memory vault service:
  - `src-tauri/src/services/memory_vault/mod.rs`
  - `src-tauri/src/services/memory_vault/types.rs`
  - `src-tauri/src/services/memory_vault/key_provider.rs`
  - `src-tauri/src/services/memory_vault/crypto.rs`
  - `src-tauri/src/services/memory_vault/repository.rs`
  - `src-tauri/src/services/memory_vault/service.rs`
- Vault uses AES-256-GCM envelope encryption at entry level with per-entry nonce.
- Added key-provider abstraction (`VaultKeyProvider`) and initial macOS keychain backend.
- Added DB migration:
  - `src-tauri/migrations/20260219090000_add_memory_vault_entries.sql`

### Changed - Memory Runtime + Command Surface (API-compatible)

**Agent Runtime (`src-tauri/src/ai/agent/`)**

- `memory.rs` now persists/retrieves memory via `MemoryVaultService` (encrypted at rest).
- Legacy `short_term.json` migration path now imports into vault and renames legacy file to `.migrated`.

**Memory Manager (`src-tauri/src/services/memory/`)**

- Reworked `MemoryManager` internals to route long-term storage and retrieval through encrypted vault while keeping existing command-facing methods.
- Preserved existing Tauri command signatures for memory APIs.

**Commands (`src-tauri/src/commands/memory.rs`)**

- Kept external API signatures stable.
- Replaced plaintext knowledge-store JSON dependency for query/index runtime path with vault-backed retrieval/tag resolution.

### Changed - Context Reset Semantics

**Agent Manager (`src-tauri/src/ai/agent/manager.rs`)**

- `clear_chat_history` now clears workspace memory from `memory_vault_entries` (instead of plaintext `memory_entries`).

### Changed - Versioning

- `package.json` -> `0.5.91`
- `src-tauri/Cargo.toml` -> `0.5.91`
- `src-tauri/tauri.conf.json` -> `0.5.91`

### Validation

- `cd src-tauri && cargo check -q` — passes
- `pnpm exec tsc --noEmit` — passes

## [0.5.90] - 2026-02-18 - IRON FLOOR (Foundation Hardening)

### Changed - Dynamic Tool Manifest Source of Truth

**Rust Backend (`src-tauri/src/`)**

- Added `src-tauri/src/services/tool_manifest.rs` to build node `SkillManifest` directly from runtime-registered tools and canonical Rust tool policy.
- `register_node` now generates skills server-side in Rust (no frontend static tool manifest coupling).
- `CommandPoller` auto-registration now uses the same runtime-generated manifest, keeping reconnect/heartbeat capability advertisements accurate.
- `skill_executor/registry.rs` now exposes reusable `registered_tool_definitions()` to avoid duplicated tool catalogs.

**Frontend (`src/`)**

- Removed static runtime registration catalog `src/constants/defaultNeuralSkills.ts`.
- Updated Neural registration call sites to `registerNode(allowedPaths)`:
  - `src/components/neural/NeuralPanel.tsx`
  - `src/hooks/useNeuralService.ts`
  - `src/services/tauri.ts`

### Fixed - Reconnect/Airlock Hardening

**Rust Backend (`src-tauri/src/services/`)**

- `neural_service.rs`:
  - Heartbeat now clears cached `node_id` on `401/404` so reconnect paths re-register cleanly.
  - `disconnect()` now clears local `node_id` after successful server disconnect.
- `airlock.rs`:
  - Added malformed-intent inference tests and empty-intent inference deny-path coverage.
- `tool_manifest.rs`:
  - Added regression test that fails if legacy `src-tauri/src/agents/` directory returns.

### Validation

- `pnpm exec tsc --noEmit` — passes
- `cd src-tauri && cargo check -q` — passes
- `cd src-tauri && cargo test -q manifest_covers_every_registered_tool` — passes
- `cd src-tauri && cargo test -q infer_tool_name_returns_none_when_payload_and_intent_are_empty` — passes

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

- xAI API uses OpenAI-compatible endpoints at <https://api.x.ai/v1>
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
