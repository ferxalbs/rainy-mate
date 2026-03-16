# Rainy MaTE

<p align="center">
  <img src="./whale.png" alt="Rainy MaTE logo" width="120" />
</p>

<p align="center">
  <strong>Native agent runtime for desktop, built with Tauri 2, Rust, and React 19.</strong>
</p>

<p align="center">
  <img alt="Version" src="https://img.shields.io/badge/version-0.5.96-0f766e" />
  <img alt="Status" src="https://img.shields.io/badge/status-beta-f59e0b" />
  <img alt="Runtime" src="https://img.shields.io/badge/runtime-tauri_2-2563eb" />
  <img alt="Engine" src="https://img.shields.io/badge/engine-rust-111827" />
</p>

> [!WARNING]
> **MaTE is in active development and currently in BETA**
>
> MaTE is **not the final product**. Behavior, interfaces, and internal systems may change without notice while the platform is still being hardened. Unexpected regressions, unfinished flows, and sharp edges are possible.
>
> This may cost us some public in the short term, but that is the price of building the system properly instead of pretending it is already stable.

## Project Overview

Rainy MaTE is a high-performance, locally-orchestrated desktop AI agent runtime.

While many AI agent platforms are constructed entirely within interpreted environments like Python or Node.js, MaTE adopts a fundamentally different architectural philosophy: maximum native performance, strict security borders, and absolute layer separation.

> **TypeScript is exclusively the view layer. Rust owns the system logic.**

By pushing all orchestration, memory loops, capability routing, and security policies into a native Rust engine (via Tauri v2), MaTE delivers a fast, highly inspectable, and secure agent experience running natively on your desktop OS instead of inside a web abstraction or cloud container.

## Why Rainy MaTE?

The transition from single-turn LLM chatbots to autonomous, stateful agents requires redefining how software trusts AI. Giving AI access to a local filesystem and shell requires uncompromising safety.

MaTE was built to solve specific challenges with existing agent platforms:

1. **Security Escapes:** Agents executing outside of hardware-level isolation often operate with the full permissions of the user. MaTE sandboxes and gates capabilities before the tool is ever executed.
2. **Context Amnesia:** Long-running conversations collapse as context windows fill. MaTE implements continuous rolling summarization and hybrid vector-lexical memory retrieval to preserve state over days or weeks of runtime.
3. **Execution Latency:** Local file I/O and process execution heavily dictate agent speed. Rust provides predictable, low-overhead system interfaces compared to interpreted language counterparts.

## Core Architecture & Capabilities

### Native Agent Supervisor

MaTE implements a robust ReAct (Think → Act → Observe) agent loop directly on your host machine.

- **Multi-Agent Orchestration:** The Supervisor mode spawns and coordinates specialized micro-agents (e.g., Research, Executor, Verifier) to resolve complex, multi-step workflows.
- **Context Compression:** Built-in rolling summarization preserves active context in long chats. When the context exceeds optimal thresholds (e.g., 80k tokens), MaTE automatically compacts the conversation history while preserving key signals.
- **Deterministic Routing:** First-class dynamic routing for leading LLM providers (Gemini BYOK, OpenAI, Anthropic, xAI), ensuring requests are routed to specific models based on capability (e.g., function calling, reasoning).

### The Airlock Security Model

MaTE enforces a rigid, 3-tier permission gate at the Rust level before any tool or capability is invoked:

- **L0 (Safe):** Read-only observational tasks (e.g., viewing files, reading web pages). Auto-approved silently.
- **L1 (Sensitive):** State-changing but contained actions (e.g., writing explicitly named files). Generates OS notifications.
- **L2 (Dangerous):** Execution of arbitrary code, destructive filesystem actions, or wide-scoping changes. Requires explicit, blocking human UI approval.

*Law: No tool can be registered in the system without an explicit, hardened security policy block. Unregistered tools fail closed.*

### Built-in Tool Arsenal

The runtime ships with a compiled suite of capabilities:

- **Filesystem & Documents:** Read, write, list, search, and parse documents natively (including built-in PDF and Markdown extraction).
- **Shell & Git:** Execute commands (against a strict binary allowlist) and manage version control wrappers natively.
- **Web & Browser:** Headless reading, arbitrary HTTP fetches, or fully visible Chrome DevTools Protocol (CDP) automation (clicking, typing, navigation, screenshots).

### The Quarantine Zone (WASM Extensibility)

MaTE supports third-party capability expansion through WebAssembly. Native sandbox environments built on `Wasmtime` ensure that external skills operate within strict resource constraints (e.g., <50MB RAM limits, bounded timeouts, Ed25519 signature verification) and deny-first filesystem/network access.

### Encrypted Local Knowledge Graph

Your context stays entirely on your machine. Agents build and query a secure local memory vault backed by `libSQL/SQLite` and encrypted point-to-point via AES-256-GCM.

Retrieval is powered by a high-speed hybrid search combining semantic vector distances (currently pinned to `gemini-embedding-001` at 3072d) and lexical frequency matching.

### The Forge (Agent Synthesis)

MaTE records your workflows. **The Forge** is an interactive workflow recorder. Perform a human workflow (clicks, terminal commands, file edits), and MaTE captures the traces, decisions, and fallbacks. It then synthesizes these signals into a fully autonomous, deterministic AI agent capable of repeating that workflow locally or being shared securely.

### Rainy ATM (Cloud Bridge)

The desktop runtime is entirely local, but it isn't isolated. **Rainy ATM** serves as the secure connection bridge that routes communications between your isolated local agents and external environments. It provides:

- Seamless webhook polling and routing from Telegram, Discord, and WhatsApp directly into your local desktop agents.
- Fleet command capabilities, including cryptographic policy verification.

---

## Design Philosophy

**Architectural Law:**
If code calculates state, persists data, executes system commands, or makes access decisions, **it lives in Rust**. React is used strictly to render state and dispatch Tauri IPC commands.

---

## Getting Started

To run Rainy MaTE locally, you will need to set up the development environment.

### Prerequisites

Ensure you have the following installed on your host machine:

- **Rust** (stable toolchain)
- **Node.js** (v20+)
- **pnpm** (Strictly required; `npm` is not supported)
- **Tauri 2 Prerequisites** (macOS: Xcode Command Line Tools)

### Installation & Execution

1. Clone the repository:

   ```bash
   git clone https://github.com/ferxalbs/rainy-mate.git
   cd rainy-mate
   ```

2. Install workspace dependencies using pnpm:

   ```bash
   pnpm install
   ```

3. Boot the desktop application in development mode:

   ```bash
   pnpm run tauri dev
   ```

---

## Contributing Guidelines

We welcome contributions, but we enforce strict architectural rules to maintain system integrity. Before submitting a Pull Request, you **must** read our [Agent Reference Checklist](./AGENTS.md).

### Core Contribution Rules

1. **Rust First:** No business logic in React components. Period.
2. **Explicit Security:** Adding a new agent capability/tool requires an explicit Airlock policy entry, or the build will fail.
3. **No Unmarked Dead Code:** Unused code must be explicitly marked for future use or removed immediately. Avoid generating warnings in the compiler output.

### Pre-Commit Validation

Every PR must pass the exact same gates used by our internal Continuous Integration. Run these locally before committing:

```bash
# 1. Rust compilation and dead-code checks
cd src-tauri && cargo check -q

# 2. Rust test suite validation
cd src-tauri && cargo test

# 3. TypeScript type integrity
pnpm exec tsc --noEmit
```

*Note: The `cargo check` command may emit warnings for unused code paths during active development iterations, but the application must successfully compile without errors for a PR to be merged.*

---

## Documentation

- [**Agent Architecture & Rules (AGENTS.md)**](./AGENTS.md)
- [**Historical Record (CHANGELOG.md)**](./CHANGELOG.md)
- [**Development Roadmap (ROADMAP.md)**](./ROADMAP.md)

Rainy MaTE is licensed under the [MIT License](./LICENSE).
