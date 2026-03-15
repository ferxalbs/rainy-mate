# Rainy MaTE (v0.5.96)

**High-Performance Agentic Desktop Runtime** built on the Tauri 2.0 framework.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)](https://github.com/enosislabs/rainy-cowork/actions)
[![Tauri](https://img.shields.io/badge/Tauri-2.0+-blue.svg)](https://tauri.app/)
[![Rust](https://img.shields.io/badge/Rust-1.78+-black.svg)](https://www.rust-lang.org/)

## System Overview

Rainy MaTE is a native desktop runtime designed for executing autonomous AI agents with high performance reliability. Unlike web-based solutions, it leverages system-level capabilities directly through a secure Rust backend, ensuring minimal latency and complete data sovereignty.

### Core Architecture

The system operates on a dual-runtime architecture:

1.  **Rust Backend (System Layer)**: Handles heavy computation, file I/O, networking, and the core agentic loop (`AgentRuntime`). Utilizes `Tokio` for asynchronous operations and `Rayon` for parallel data processing.
2.  **Web Frontend (Presentation Layer)**: A lightweight React 19 interface rendered via `WebView` constraints, communicating with the backend exclusively through the secure Tauri IPC bridge.

> **Performance Note**: The runtime is compiled to native machine code. It does not require a local Python environment or Docker container.

---

## Technical Specifications

### 1. Agent Runtime (v2)

The `AgentRuntime` implements a robust **ReAct (Reasoning + Acting)** loop designed for stability:

- **Workflow Engine**: Orchestrates the `Think -> Act -> Observe` cycle.
- **Context Management**: Implements a sliding window context manager (`ContextWindow`) with token-aware truncation to maintain coherence within LLM constraints.
- **Memory Persistence**: All interactions are transactionally stored in a comprehensive SQLite database with Write-Ahead Logging (WAL) enabled for concurrency.

### 2. Intelligent Router

The `IntelligentRouter` dynamically balances AI inference loads:

- **Circuit Breaker**: Detects provider failures and automatically reroutes requests.
- **Load Balancer**: Distributes traffic across configured providers (OpenAI, Anthropic, Google, xAI) based on latency and error rates.
- **Cost Optimization**: Selects models based on task complexity and budget constraints.

### 3. Security Model (Airlock)

Rainy MaTE enforces a capability-based security model known as **Airlock**:

| Level  | Designation | Description                                             | Example Operations                      |
| :----- | :---------- | :------------------------------------------------------ | :-------------------------------------- |
| **L0** | Safe        | Read-only operations. Auto-approved.                    | `read_file`, `list_files`, `web_search` |
| **L1** | Sensitive   | State-modifying operations. Requires user notification. | `write_file`, `browse_url`              |
| **L2** | Creating    | Critical system operations. Requires explicit approval. | `execute_command`, `delete_file`        |

This model ensures that autonomous agents cannot perform destructive actions without operator consent.

---

## Capabilities & Tooling

The runtime exposes a standardized set of native tools to the agent:

- **FileSystem**: `read`, `write`, `list`, `search`, `delete`, `move` (Scoped to allowed directories).
- **System Shell**: Secure command execution (Allowlist: `npm`, `cargo`, `git`, `ls`, `grep`, `echo`, `cat`).
- **Browser Automation**: Headless navigation, DOM extraction, and interaction via `ferrum` driver.
- **Network**: HTTP/HTTPS requests, `web_search` (Tavily).
- **Data Processing**: Text extraction, format conversion.

---

## Integration: Cloud Cortex (Rainy ATM)

Rainy MaTE seamlessly integrates with the **Rainy ATM** (Cloud Cortex) protocol for distributed operations:

- **Unified Lane Queue**: Guaranteed message delivery for remote commands (Telegram/Discord).
- **Heartbeat Sync**: Real-time status reporting and command polling.
- **Audit Trail**: Immutable logging of all executed commands and policy changes.

---

## Installation & Build

### Prerequisites

- **Rust**: v1.78+ (`rustup update stable`)
- **Node.js**: v18+ (`LTS recommended`)
- **Package Manager**: `pnpm` (Strictly enforced)
- **Build Tools**: XCode Command Line Tools (macOS) or `build-essential` (Linux)

### Build Instructions

1.  **Clone Repository**

    ```bash
    git clone https://github.com/enosislabs/rainy-cowork.git
    cd rainy-cowork
    ```

2.  **Install Dependencies**

    ```bash
    pnpm install
    ```

3.  **Development Mode** (Hot Reload)

    ```bash
    pnpm tauri dev
    ```

4.  **Production Build** (Optimized Release)
    ```bash
    pnpm tauri build
    ```

---

## Configuration

The application requires valid API keys for AI providers. These are securely stored in the OS Keychain/Keyring.

**Supported Providers:**

- OpenAI (`OPENAI_API_KEY`)
- Google Gemini (`GOOGLE_API_KEY`)
- Anthropic (`ANTHROPIC_API_KEY`)
- Groq (`GROQ_API_KEY`)
- xAI (`XAI_API_KEY`)
- Rainy SDK (`RAINY_API_KEY`)

---

## License

This software is licensed under the **MIT License**.

See [LICENSE](LICENSE) for the full text.

**Note**: Usage of the cloud-hosted **Rainy ATM** infrastructure is subject to the [Enosis Labs Terms of Service](https://enosislabs.com/terms).

---

## 📚 Documentation

Comprehensive documentation is available at:

- **[Rainy MaTE Docs](https://rainy-mate-docs.vercel.app/)** — Main documentation
- **[API Reference](https://rainy-mate-docs.vercel.app/docs/)** — Detailed API docs
- **[Architecture](https://rainy-mate-docs.vercel.app/docs/architecture)** — System architecture
- **[Features](https://rainy-mate-docs.vercel.app/docs/features)** — Feature overview
- **[Contributing](https://rainy-mate-docs.vercel.app/docs/contributing)** — Contribution guide

---

## 🔐 Security

### Airlock Security Levels

Rainy MaTE implements a three-tier security system for agent operations:

| Level | Name          | Description          | Approval Required |
| ----- | ------------- | -------------------- | ----------------- |
| **0** | **Safe**      | Read-only operations | Auto-approved     |
| **1** | **Sensitive** | Write operations     | Notification      |
| **2** | **Dangerous** | Execute/Delete       | Explicit approval |

### Permission Policies

Enterprise-grade permission management with:

- **Workspace-Specific Policies** — Granular access control
- **Audit Trail** — Immutable policy change history
- **SLO Monitoring** — Service level objective tracking
- **Alert Management** — Retention, acknowledgment, and audit

### Security Best Practices

- API keys stored in OS keychain
- Local-first data by default
- Sandboxed AI operations
- Explicit user permissions for sensitive operations

---

## 🤝 Contributing

We welcome contributions from the community! Rainy MaTE is built by developers, for developers.

### Ways to Contribute

- 🐛 **Bug Reports** — Help us identify and fix issues
- ✨ **Feature Requests** — Suggest new capabilities
- 💻 **Code Contributions** — Submit pull requests
- 📚 **Documentation** — Improve guides and examples
- 🎨 **UI/UX** — Enhance the user experience
- 🧪 **Testing** — Help ensure quality and reliability

### Contribution Guidelines

1. **Fork** the repository
2. **Create** a feature branch: `git checkout -b feature/amazing-feature`
3. **Commit** your changes: `git commit -m 'feat(area): add amazing feature'`
4. **Push** to the branch: `git push origin feature/amazing-feature`
5. **Open** a Pull Request

See our [Contributing Guide](https://rainy-mate-docs.vercel.app/docs/contributing) for detailed information.

---

## 📄 License & Legal

This project is licensed under the **MIT License** with additional terms for AI services.

### Legal Documentation

- **[LICENSE](LICENSE)** — MIT License terms
- **[TERMS_OF_USE.md](TERMS_OF_USE.md)** — Complete terms and conditions
- **[PRIVACY_POLICY.md](PRIVACY_POLICY.md)** — Data and privacy handling
- **[SECURITY.md](SECURITY.md)** — Security practices and reporting

### Enosis Labs Integration

When using Enosis Labs AI services, you must also comply with:

- [Enosis Labs Terms of Service](https://enosislabs.vercel.app/terms)
- [Enosis Labs Privacy Policy](https://enosislabs.vercel.app/privacy)

---

## 🌟 Acknowledgments

Rainy MaTE is inspired by the agentic AI revolution and built on the shoulders of giants:

- **Tauri** — For the amazing cross-platform framework
- **React** — For the powerful UI library
- **Rust** — For performance and safety
- **HeroUI** — For beautiful, accessible components
- **OpenAI, Google, Anthropic, xAI** — For advancing AI capabilities
- **The Open Source Community** — For making this possible

Special thanks to all contributors who help make Rainy MaTE better every day! 🎉

---

## 📞 Support

- 📖 **[Documentation](https://rainy-mate-docs.vercel.app/)** — Comprehensive guides
- 💬 **[Discussions](https://github.com/enosislabs/rainy-cowork/discussions)** — Community Q&A
- 🐛 **[Issues](https://github.com/enosislabs/rainy-cowork/issues)** — Bug reports
- 📧 **Email** — Direct support for complex issues

---

<div align="center">

**Built with ❤️ for the open source community**

[⭐ Star on GitHub](https://github.com/enosislabs/rainy-cowork) • [📖 Documentation](https://rainy-mate-docs.vercel.app/) • [💬 Community](https://github.com/enosislabs/rainy-cowork/discussions)

**Rainy MaTE** — _The Open-Source AI Desktop Agent Platform_

</div>
