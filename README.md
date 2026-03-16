# Rainy MaTE

<div align="center">
  <img src="./whale.png" alt="Rainy MaTE logo" width="120" />
  <h1 style="margin-bottom: 0.35rem;">Rainy MaTE</h1>
  <p style="margin-top: 0; font-size: 1.05rem;">
    Native agent runtime for desktop, built with <strong>Tauri 2</strong>, <strong>Rust</strong>, and <strong>React 19</strong>.
  </p>
  <p>
    <img alt="Version" src="https://img.shields.io/badge/version-0.5.96-0f766e" />
    <img alt="Status" src="https://img.shields.io/badge/status-beta-f59e0b" />
    <img alt="Runtime" src="https://img.shields.io/badge/runtime-tauri_2-2563eb" />
    <img alt="Engine" src="https://img.shields.io/badge/engine-rust-111827" />
  </p>
</div>

<div style="margin: 1.5rem 0; padding: 1rem 1.1rem; border: 1px solid #f59e0b; border-radius: 14px; background: linear-gradient(135deg, rgba(245,158,11,0.16), rgba(127,29,29,0.10));">
  <h2 style="margin: 0 0 0.55rem 0;">Warning: MaTE is in active development and currently in BETA</h2>
  <p style="margin: 0 0 0.65rem 0;">
    MaTE is <strong>not the final product</strong>. Behavior, interfaces, and internal systems may change without notice while the platform is still being hardened. Unexpected regressions, unfinished flows, and sharp edges are possible.
  </p>
  <p style="margin: 0 0 0.65rem 0;">
    This may cost us some public in the short term, but that is the price of building the system properly instead of pretending it is already stable.
  </p>
  <p style="margin: 0;">
    <strong>Spanish note:</strong> MaTE está en desarrollo activo y en <strong>BETA</strong>. Puede haber cambios no esperados y este no es el producto final.
  </p>
</div>

## What MaTE Is

Rainy MaTE is a desktop-first agent runtime. The UI lives in React, but the real system runs in Rust: agent orchestration, tool execution, memory, security policy, browser control, workspace access, and cloud bridge logic are implemented in `src-tauri/`.

It is designed around one architectural rule:

> TypeScript is the view layer. Rust owns the logic.

That separation is enforced across the project so the desktop app remains fast, inspectable, and security-aware.

## Current State

- Active desktop version: `0.5.96`
- Desktop stack: `Tauri 2` + `Rust` + `React 19` + `Vite`
- Cloud/backend stack: `Bun` in [`rainy-atm/`](./rainy-atm)
- Package manager: `pnpm` only
- Development status: BETA / active internal iteration

Recent system work reflected in the codebase:

- `THE FORGE` added workflow recording and specialist-agent generation.
- ATM security was hardened with policy-hash verification, safer WebSocket auth flow, and audit fixes.
- Memory internals were optimized, while the stable embedder surface remains locked to `gemini-embedding-001` (`3072d`) in the current desktop UI.

## What Exists Today

### Agent Runtime

MaTE ships a native ReAct-style runtime in [`src-tauri/src/ai/agent/`](./src-tauri/src/ai/agent):

- `Think -> Act -> Observe` workflow
- Tool-call execution through `SkillExecutor`
- Streaming and non-streaming model paths
- History management and long-chat continuity
- Memory retrieval injection before model calls
- Supervisor mode for multi-agent specialist orchestration

The runtime also includes explicit truthfulness rules: tool output is treated as the source of truth, and failures must be reported rather than fabricated.

### Native Tooling Surface

Registered built-in tools live in [`src-tauri/src/services/skill_executor/registry.rs`](./src-tauri/src/services/skill_executor/registry.rs), with policy enforced in [`src-tauri/src/services/tool_policy.rs`](./src-tauri/src/services/tool_policy.rs).

Current built-in categories:

- Filesystem: read, list, inspect, search, write, append, move, delete, document ingestion
- Shell/Git: command execution with allowlist, git status/diff/log/show/branch wrappers
- Web/HTTP: web search, page reading, JSON/text fetch, JSON POST
- Browser automation: open URL, tabs, click, type, submit, go back, snapshot, screenshot, link extraction

Every registered tool must have an explicit Airlock policy entry. Unknown tools are denied by default.

### Airlock Security Model

MaTE uses a three-level execution gate enforced in Rust:

| Level | Meaning | Typical behavior |
| --- | --- | --- |
| `L0` | Safe | Auto-approved, read-only or observational |
| `L1` | Sensitive | State-changing, notification-gated |
| `L2` | Dangerous | Explicit approval required |

This is not only a UI convention. Policy is checked in multiple layers, including workflow validation, executor checks, and cloud/desktop policy reconciliation.

### Memory System

The memory stack combines short-term runtime memory and an encrypted long-term vault:

- Long-term store: AES-256-GCM encrypted `libSQL/SQLite`
- Retrieval: hybrid semantic + lexical search
- Injection: bounded retrieval context before model calls
- Stable embedding surface today: `gemini-embedding-001` at `3072` dimensions

The repo contains internal work for dual embedding profiles, but the stable end-user desktop configuration is still pinned to the locked Gemini profile in the settings UI.

### THE FORGE

`THE FORGE` is the in-product workflow-to-agent factory:

- Record a workflow
- Capture steps, tools, decisions, errors, retries
- Generate a draft specialist agent
- Validate it before save/activation
- Export/share through ATM flows

Primary desktop entry points for this live in:

- [`src-tauri/src/commands/workflow_factory.rs`](./src-tauri/src/commands/workflow_factory.rs)
- [`src-tauri/src/services/workflow_recorder.rs`](./src-tauri/src/services/workflow_recorder.rs)
- [`src-tauri/src/services/agent_library.rs`](./src-tauri/src/services/agent_library.rs)

### Rainy ATM Integration

`Rainy ATM`, located in [`rainy-atm/`](./rainy-atm), is a central part of the system architecture and acts as the connector between mobile and desktop.

ATM currently covers:

- mobile-to-desktop command transport
- dynamic tool registry behavior
- workspace/channel routing
- node registration and heartbeat
- fleet controls and command polling
- audit and policy handling

Its runtime uses `Bun`, not Node.js.

## Architecture

```text
rainy-mate/
├── src/                  React UI only
├── src-tauri/            Native Rust engine
│   ├── src/ai/           Providers, router, agent runtime, supervisor flow
│   ├── src/commands/     Thin Tauri invoke layer
│   ├── src/services/     Business logic, tools, memory, security, ATM bridge
│   └── migrations/       Database and vault migrations
├── rainy-atm/            Bun connector layer between mobile and desktop
└── docs/                 Supporting documentation
```

Key service areas in the Rust backend:

- `ai/agent/`: runtime, workflow, memory wiring, supervisor orchestration
- `services/skill_executor/`: native tool handlers and tool registry
- `services/airlock.rs`: approval and execution gating
- `services/memory/` and `services/memory_vault/`: retrieval and encrypted persistence
- `services/workspace.rs`: workspace lifecycle and scope enforcement
- `services/command_poller.rs`: cloud command execution loop
- `services/neural_service.rs`: desktop node registration and heartbeat

## Startup Model

The application boot sequence matters. `lib.rs` initializes core services first, then completes app-handle-dependent services in `setup()`.

In practical terms:

1. Providers, task manager, file/document/image services
2. Workspace, router, ATM client, neural service, browser controller
3. Skill executor, command poller, socket client, workflow recorder, agent library
4. `setup()` services such as updater, folder manager, file-op init, memory manager, Airlock, database, agent manager, and poller startup

This ordering is intentional. Services that need app data directories or `AppHandle` are initialized later.

## Supported Provider Surface

The codebase includes provider integrations and routing infrastructure for:

- OpenAI
- Google Gemini
- Anthropic
- xAI
- Groq
- Rainy SDK
- additional model-catalog routing infrastructure in the unified registry

Exact model availability can change over time and depends on provider configuration, credentials, and routing rules.

## Development

### Prerequisites

- Rust stable
- Node.js
- `pnpm`
- Tauri desktop prerequisites for your platform
- `Bun` only if you will work inside `rainy-atm/`

### Install

```bash
git clone https://github.com/ferxalbs/rainy-mate.git
cd rainy-mate
pnpm install
```

### Run

```bash
pnpm run tauri dev
```

### Required Validation Gates

These are the baseline gates used by the project:

```bash
cd src-tauri && cargo check -q
cd src-tauri && cargo test
pnpm exec tsc --noEmit
```

For this README update, the current repository passed:

- `cargo check -q`
- `pnpm exec tsc --noEmit`

`cargo check` still emits some warnings for currently unused code paths, but it completes successfully.

## Documentation Pointers

- [`AGENTS.md`](./AGENTS.md): architecture, rules, startup order, module map, tool policy model
- [`CHANGELOG.md`](./CHANGELOG.md): canonical historical record
- [`ROADMAP.md`](./ROADMAP.md): planned work
- [`FEATURES.md`](./FEATURES.md): feature inventory
- [`SECURITY.md`](./SECURITY.md): security notes
- [`CONTRIBUTING.md`](./CONTRIBUTING.md): contributor workflow

## Contribution Rules

If you plan to contribute, read [`AGENTS.md`](./AGENTS.md) first. The project has strict rules:

- business logic in Rust, not React
- `pnpm` only
- modularized code paths
- explicit tool policies
- no undocumented dead code
- validation gates before claiming work is done

## License

This repository is licensed under the [MIT License](./LICENSE).
