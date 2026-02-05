# 🌧️ Rainy MaTE - Open Source AI Desktop Agent

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Tauri](https://img.shields.io/badge/Tauri-2.0+-blue.svg)](https://tauri.app/)
[![React](https://img.shields.io/badge/React-19+-61dafb.svg)](https://reactjs.org/)
[![Rust](https://img.shields.io/badge/Rust-1.70+-000000.svg)](https://www.rust-lang.org/)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.9+-3178c6.svg)](https://www.typescriptlang.org/)
[![HeroUI](https://img.shields.io/badge/HeroUI-3.0+-ff6b6b.svg)](https://heroui.com/)

**The open-source alternative to Claude Cowork or OpenClaw (formerly Clawdbot and Moltbot)** - Transform your desktop into an intelligent AI coworker that handles file management, document generation, web research, and task automation with complete privacy and control.

> 🚀 **Cross-platform AI agent** built with Tauri, React, and Rust  
> 🔒 **Privacy-first** - Your data stays on your device  
> 🎯 **Multi-provider AI** - OpenAI, Gemini, Groq, Cerebras, and more  
> ⚡ **Real-time collaboration** - AI that works alongside you

## 🎯 Why Rainy MaTE?

**The Problem**: Proprietary AI agents like Claude Cowork are expensive ($20-200/month), macOS-only, and keep your data in the cloud.

**The Solution**: Rainy MaTE is a free, open-source, cross-platform AI desktop agent that gives you:

- ✅ **Complete Privacy** - Your files never leave your device
- ✅ **Cross-Platform** - Windows, macOS, and Linux support
- ✅ **Multi-AI Provider** - Choose from OpenAI, Gemini, Groq, Cerebras, and more
- ✅ **Zero Subscription** - Use your own API keys, pay only for what you use
- ✅ **Full Control** - Customize, extend, and modify as needed
- ✅ **Open Source** - Transparent, auditable, community-driven

## ✨ Key Features

### 🤖 **Intelligent AI Agent**
- **Multi-step task execution** with autonomous planning and execution
- **Context-aware conversations** that remember your workflow
- **Smart file operations** - organize, rename, move, and process files intelligently
- **Advanced reasoning** with Gemini 3 thinking capabilities and thought signatures

### 📁 **Advanced File Management**
- **Bulk file operations** - organize thousands of files in seconds
- **Smart categorization** - AI-powered file sorting and tagging
- **Content extraction** - Extract text, metadata, and insights from documents
- **Batch processing** - Apply operations across multiple files simultaneously

### 🌐 **Web Research & Content**
- **Tavily-powered web search** - Real-time information retrieval
- **Content extraction** - Convert web pages to clean markdown
- **Research automation** - Gather, analyze, and synthesize information
- **Citation management** - Automatic source tracking and referencing

### 📄 **Document Generation**
- **Template-based creation** - Generate reports, summaries, and documents
- **AI-assisted writing** - Content creation with multiple AI providers
- **Format conversion** - Transform between document formats
- **Export options** - PDF, Markdown, HTML, and more

### 🎨 **Modern UI/UX**
- **HeroUI components** - Beautiful, accessible interface
- **Theme system** - Multiple themes with smooth transitions
- **Responsive design** - Optimized for all screen sizes
- **Real-time feedback** - Live progress indicators and status updates

## 🚀 Quick Start

### Prerequisites

- [Node.js](https://nodejs.org/) 18+ 
- [Rust](https://www.rust-lang.org/) 1.70+
- [Tauri CLI](https://tauri.app/v1/guides/getting-started/prerequisites)

### Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/rainy-cowork.git
cd rainy-cowork

# Install dependencies
npm install

# Start development server
npm run tauri dev
```

### First Run Setup

1. **Launch the application**
2. **Configure AI providers** - Add your API keys in Settings
3. **Grant folder permissions** - Allow the AI to access specific directories
4. **Start your first task** - Try "Organize my Downloads folder"

### Example Tasks

```bash
# File Organization
"Organize my Downloads folder by file type and date"

# Document Creation  
"Create a project summary from these meeting notes"

# Web Research
"Research the latest trends in AI development and create a report"

# Data Processing
"Extract data from these invoices and create a spreadsheet"
```

## 🏗️ Architecture

Rainy Cowork is built with a modern, modular architecture:

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   React Frontend │    │   Tauri Bridge   │    │  Rust Backend   │
│                 │◄──►│                  │◄──►│                 │
│ • HeroUI        │    │ • IPC Commands   │    │ • AI Providers  │
│ • TypeScript    │    │ • File System    │    │ • File Ops      │
│ • Framer Motion │    │ • Notifications  │    │ • Web Research  │
└─────────────────┘    └──────────────────┘    └─────────────────┘
                                │
                                ▼
                       ┌──────────────────┐
                       │   Rainy API v2   │
                       │                  │
                       │ • Multi-Provider │
                       │ • Billing System │
                       │ • Rate Limiting  │
                       └──────────────────┘
```

### Core Components

- **Frontend**: React 19 + TypeScript + HeroUI for modern, responsive UI
- **Backend**: Rust + Tauri for secure, performant native operations  
- **AI Integration**: Rainy SDK for unified multi-provider AI access
- **API Gateway**: Hono + Bun powered API with intelligent routing
- **Database**: Neon PostgreSQL with Drizzle ORM for data persistence

## 🔧 Configuration

### AI Provider Setup

Configure your preferred AI providers in the settings panel:

| Provider | Models | Features |
|----------|--------|----------|
| **OpenAI** | GPT-4o, GPT-5, O3, O4-mini | Advanced reasoning, tool calling |
| **Google Gemini** | Gemini 3 Pro, Gemini 2.5 Flash | Thinking capabilities, multimodal |
| **Groq** | Llama 3.1, Llama 3.3 | Ultra-fast inference |
| **Cerebras** | Llama 3.1 8B | High-performance processing |
| **Enosis Labs** | Astronomer series | Specialized AI models |

### Environment Variables

```bash
# AI Provider API Keys
OPENAI_API_KEY=your_openai_key
GOOGLE_API_KEY=your_gemini_key
GROQ_API_KEY=your_groq_key

# Rainy API Configuration
RAINY_API_KEY=your_rainy_api_key
RAINY_API_URL=https://api.rainy.com

# Optional: Web Search
TAVILY_API_KEY=your_tavily_key
```

### Folder Permissions

Grant specific folder access to enable AI operations:

- **Downloads** - For file organization tasks
- **Documents** - For document processing and creation
- **Desktop** - For quick access to current work
- **Custom Folders** - Project-specific directories

## 📊 Comparison with Alternatives

| Feature | Rainy Cowork | Claude Cowork | Cursor | GitHub Copilot |
|---------|--------------|---------------|--------|----------------|
| **Price** | Free (API costs only) | $20-200/month | $20/month | $10/month |
| **Platform** | Windows, macOS, Linux | macOS only | Cross-platform | IDE-dependent |
| **Privacy** | Local execution | Cloud-based | Cloud-based | Cloud-based |
| **AI Providers** | Multiple (OpenAI, Gemini, etc.) | Claude only | GPT-4 | GitHub models |
| **File Operations** | ✅ Full access | ✅ Sandboxed | ❌ Limited | ❌ Code only |
| **Web Research** | ✅ Tavily integration | ❌ Limited | ❌ No | ❌ No |
| **Open Source** | ✅ MIT License | ❌ Proprietary | ❌ Proprietary | ❌ Proprietary |
| **Customization** | ✅ Full control | ❌ Limited | ❌ Limited | ❌ Limited |

## 🛣️ Roadmap

### v0.4.0 - Enhanced AI Capabilities (Q2 2026)
- [ ] Advanced thinking modes with Gemini 3
- [ ] Multi-modal processing (images, audio, video)
- [ ] Custom AI agent creation
- [ ] Plugin ecosystem foundation

### v0.5.0 - Collaboration Features (Q3 2026)
- [ ] Team workspaces
- [ ] Shared AI agents
- [ ] Real-time collaboration
- [ ] Cloud sync (optional)

### v1.0.0 - Production Ready (Q4 2026)
- [ ] Enterprise features
- [ ] Advanced security controls
- [ ] Performance optimizations
- [ ] Comprehensive documentation

See our [detailed roadmap](ROADMAP.md) for more information.

## 🤝 Contributing

We welcome contributions from the community! Rainy Cowork is built by developers, for developers.

### Ways to Contribute

- 🐛 **Bug Reports** - Help us identify and fix issues
- ✨ **Feature Requests** - Suggest new capabilities
- 💻 **Code Contributions** - Submit pull requests
- 📚 **Documentation** - Improve guides and examples
- 🎨 **UI/UX** - Enhance the user experience
- 🧪 **Testing** - Help ensure quality and reliability

### Development Setup

```bash
# Fork and clone the repository
git clone https://github.com/yourusername/rainy-cowork.git
cd rainy-cowork

# Install dependencies
npm install

# Start development environment
npm run tauri dev

# Run tests
npm test
cargo test
```

### Contribution Guidelines

1. **Fork** the repository
2. **Create** a feature branch: `git checkout -b feature/amazing-feature`
3. **Commit** your changes: `git commit -m 'Add amazing feature'`
4. **Push** to the branch: `git push origin feature/amazing-feature`
5. **Open** a Pull Request

See our [Contributing Guide](CONTRIBUTING.md) for detailed information.

## 📄 License & Legal

This project is licensed under the **MIT License** with additional terms for AI services - see the [LICENSE](LICENSE) file for details.

### Legal Documentation
- **[Terms of Use](TERMS_OF_USE.md)** - Complete terms and conditions
- **[Privacy Policy](PRIVACY_POLICY.md)** - How we handle your data and privacy
- **[Security Policy](SECURITY.md)** - Security practices and vulnerability reporting

### Enosis Labs Integration
When using Enosis Labs AI services, you must also comply with:
- [Enosis Labs Terms of Service](https://enosislabs.vercel.app/terms)
- [Enosis Labs Privacy Policy](https://enosislabs.vercel.app/privacy)

### What this means:
- ✅ **Commercial use** - Use in commercial projects
- ✅ **Modification** - Modify and adapt the code
- ✅ **Distribution** - Share and distribute freely
- ✅ **Private use** - Use for personal projects
- ✅ **Patent use** - Use any patents in the project
- ⚠️ **AI Service Compliance** - Must follow AI provider terms when using their services

## 🌟 Community & Support

### Get Help
- 📖 **Documentation** - Comprehensive guides and API docs
- 💬 **Discussions** - Community Q&A and feature discussions
- 🐛 **Issues** - Bug reports and feature requests
- 📧 **Email** - Direct support for complex issues

### Stay Updated
- ⭐ **Star** this repository to show support
- 👀 **Watch** for updates and new releases
- 🍴 **Fork** to contribute or customize
- 📢 **Share** with your network

## 🙏 Acknowledgments

Rainy Cowork is inspired by the agentic AI revolution and built on the shoulders of giants:

- **Tauri** - For the amazing cross-platform framework
- **React** - For the powerful UI library
- **Rust** - For performance and safety
- **HeroUI** - For beautiful, accessible components
- **OpenAI, Google, Anthropic** - For advancing AI capabilities
- **The Open Source Community** - For making this possible

Special thanks to all contributors who help make Rainy Cowork better every day! 🎉

---

<div align="center">

**Built with ❤️ for the open source community**

[⭐ Star on GitHub](https://github.com/yourusername/rainy-cowork) • [📖 Documentation](https://docs.rainy-cowork.com) • [💬 Community](https://github.com/yourusername/rainy-cowork/discussions) • [🐛 Report Bug](https://github.com/yourusername/rainy-cowork/issues)

**Keywords**: AI desktop agent, open source AI, file automation, document generation, web research, cross-platform AI, privacy-first AI, Tauri app, React TypeScript, Rust backend, multi-provider AI, Claude Cowork alternative, agentic AI, productivity automation, intelligent file management

</div>
