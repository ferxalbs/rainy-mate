# Changelog

All notable changes to Rainy Cowork will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
