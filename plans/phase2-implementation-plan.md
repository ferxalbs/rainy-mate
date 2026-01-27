# PHASE 2 Implementation Plan: Intelligence Layer

**Version:** 2.0.0  
**Status:** Planning  
**Created:** 2026-01-27  
**Target Timeline:** Weeks 6-9  
**Owner:** Architecture Team

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Architecture Design](#2-architecture-design)
3. [Implementation Modules](#3-implementation-modules)
4. [Backend Implementation Details](#4-backend-implementation-details)
5. [Frontend Implementation Details](#5-frontend-implementation-details)
6. [Testing Strategy](#6-testing-strategy)
7. [Migration & Compatibility](#7-migration--compatibility)
8. [Dependencies](#8-dependencies)
9. [Risks & Mitigation](#9-risks--mitigation)
10. [Success Metrics](#10-success-metrics)
11. [Appendices](#11-appendices)

---

## 1. Executive Summary

### 1.1 PHASE 2 Scope and Objectives

PHASE 2 introduces the Intelligence Layer, transforming the Rainy Cowork application from a single-agent file operation system into a sophisticated multi-agent collaborative platform. This phase builds upon the solid foundation established in PHASE 1, which delivered workspace management, file operations, task queuing, and AI provider integration.

**Primary Objectives:**

- **Multi-Agent Orchestration:** Implement a Director Agent capable of task decomposition and coordinating multiple specialized agents working in parallel
- **Intelligent Memory:** Deploy a dual-tier memory system combining short-term context awareness with long-term persistent knowledge
- **Self-Improvement:** Enable the system to reflect on performance, recognize error patterns, and optimize strategies autonomously
- **Quality Assurance:** Integrate Critic and Governor agents to ensure output quality and security compliance

### 1.2 Success Criteria

| Metric | Target | Measurement Method |
|--------|--------|-------------------|
| Task Decomposition Accuracy | >90% | DirectorAgent correctly breaking complex tasks |
| Agent Coordination Latency | <100ms | Time for Director to assign subtasks |
| Memory Retrieval Accuracy | >85% | Relevant context recall in tests |
| Self-Improvement Rate | >20% | Reduction in repeated errors per quarter |
| Security Block Rate | 100% | GovernorAgent blocking all dangerous operations |

### 1.3 Timeline Overview

```
Week 6  │ PHASE 2.1: Agent Foundation
        │ - AgentRegistry, BaseAgent, DirectorAgent, MessageBus
────────┼────────────────────────────────────────────────────────
Week 7  │ PHASE 2.2: Specialized Agents
        │ - Researcher, Executor, Creator, Designer, Developer, Analyst
────────┼────────────────────────────────────────────────────────
Week 8  │ PHASE 2.3: Memory System
        │ - ShortTermMemory, LongTermMemory, Embeddings
────────┼────────────────────────────────────────────────────────
Week 9  │ PHASE 2.4: Reflection & Governance
        │ - CriticAgent, GovernorAgent, ReflectionEngine
```

---

## 2. Architecture Design

### 2.1 Multi-Agent Hierarchy

```
                        ┌─────────────────┐
                        │   User Interface │
                        └────────┬────────┘
                                 │
                        ┌────────▼────────┐
                        │   Tauri Backend  │
                        └────────┬────────┘
                                 │
              ┌──────────────────┼──────────────────┐
              │                  │                  │
     ┌────────▼────────┐ ┌───────▼───────┐ ┌───────▼───────┐
     │   DirectorAgent │ │ CriticAgent   │ │ GovernorAgent │
     │  (Orchestrator) │ │  (Quality)    │ │  (Security)   │
     └────────┬────────┘ └───────────────┘ └───────────────┘
              │
     ┌────────┼────────┬────────┬────────┬────────┬────────┐
     │        │        │        │        │        │        │
┌────▼──┐ ┌───▼──┐ ┌───▼──┐ ┌───▼──┐ ┌───▼──┐ ┌───▼──┐
│Resear-│ │Executor│ │Creator│ │Designer│ │Developer│ │Analyst│
│cher   │ │Agent  │ │Agent  │ │Agent  │ │Agent  │ │Agent │
└───────┘ └───────┘ └───────┘ └───────┘ └───────┘ └───────┘
     │        │        │        │        │        │
     └────────┴────────┴────────┴────────┴────────┘
                           │
              ┌────────────┼────────────┐
              │            │            │
     ┌────────▼────────┐ ┌─▼────────┐ ┌─▼────────┐
     │  ShortTermMemory│ │LongTerm  │ │Reflection│
     │   (RingBuffer)  │ │Memory    │ │Engine    │
     │                 │ │(LanceDB) │ │          │
     └─────────────────┘ └──────────┘ └──────────┘
```

### 2.2 Data Flow Architecture

**Request Flow:**

```
1. User Input → Frontend → Tauri Command
2. Tauri Command → DirectorAgent.analyze()
3. DirectorAgent → decomposes task
4. DirectorAgent → assigns subtasks to specialized agents
5. Specialized Agents → execute with memory context
6. Results → DirectorAgent aggregates
7. DirectorAgent → CriticAgent evaluates
8. GovernorAgent → validates security
9. Response → Frontend displays
```

**Memory Flow:**

```
Short-term Memory (RingBuffer):
- Current session context
- Recent actions (last 100)
- Active task state
- Conversation history

Long-term Memory (LanceDB):
- Episodic: Past task outcomes
- Semantic: Document embeddings
- Procedural: Learned strategies
- User preferences

Retrieval Pipeline:
Query → Embed → Search → Rank → Retrieve → Inject → Agent Context
```

### 2.3 Agent Communication Protocols

**Message Types:**

| Message Type | Purpose | Direction |
|-------------|---------|-----------|
| TaskAssign | Assign work to agent | Director → Specialist |
| TaskResult | Return execution result | Specialist → Director |
| TaskStatus | Report progress | Specialist → Director |
| QueryMemory | Request context | Any → Memory System |
| StoreMemory | Persist learning | Any → Memory System |
| SecurityCheck | Request validation | Any → GovernorAgent |
| QualityReview | Request evaluation | Any → CriticAgent |

---

## 3. Implementation Modules

### 3.1 PHASE 2.1: Agent Foundation (Week 6)

**Objective:** Establish the core agent infrastructure

#### 3.1.1 Agent Trait Definition

```rust
// src-tauri/src/agents/traits.rs

/// Core agent capability trait
#[async_trait::async_trait]
pub trait Agent: Send + Sync {
    /// Unique agent identifier
    fn id(&self) -> AgentId;
    
    /// Agent type for routing
    fn agent_type(&self) -> AgentType;
    
    /// Human-readable name
    fn name(&self) -> String;
    
    /// Execute a task
    async fn execute(&self, context: &AgentContext) -> Result<AgentResult, AgentError>;
    
    /// Get current capabilities
    fn capabilities(&self) -> Vec<Capability>;
}

/// Specialized agent additional behavior
trait SpecializedAgent: Agent {
    /// Domain-specific capabilities
    fn domain(&self) -> AgentDomain;
    
    /// Priority for task assignment (1-10, higher = more specialized)
    fn specialization_level(&self) -> u8;
}
```

#### 3.1.2 BaseAgent Implementation

```rust
// src-tauri/src/agents/base_agent.rs

/// Common functionality for all agents
#[derive(Clone)]
pub struct BaseAgent {
    id: AgentId,
    agent_type: AgentType,
    name: String,
    message_bus: Arc<MessageBus>,
    short_term_memory: Arc<RwLock<ShortTermMemory>>,
    metrics: AgentMetrics,
}

impl BaseAgent {
    pub fn new(id: AgentId, agent_type: AgentType, name: String) -> Self;
    
    pub async fn send_message(&self, recipient: AgentId, message: AgentMessage);
    
    pub async fn broadcast(&self, message: AgentMessage);
    
    pub fn record_metric(&self, metric: MetricType, value: f64);
    
    pub async fn retrieve_context(&self, query: &str) -> Option<MemoryContext>;
}
}
```

#### 3.1.3 AgentRegistry Service

```rust
// src-tauri/src/agents/registry.rs

/// Central registry for all agents
#[derive(Default)]
pub struct AgentRegistry {
    agents: HashMap<AgentId, Arc<dyn Agent>>,
    by_type: HashMap<AgentType, Vec<AgentId>>,
    message_bus: Arc<MessageBus>,
    metrics: RegistryMetrics,
}

impl AgentRegistry {
    pub fn register(&mut self, agent: Arc<dyn Agent>) -> Result<(), RegistryError>;
    
    pub fn unregister(&mut self, id: AgentId);
    
    pub fn get(&self, id: AgentId) -> Option<&Arc<dyn Agent>>;
    
    pub fn find_by_type(&self, agent_type: AgentType) -> Vec<&Arc<dyn Agent>>;
    
    pub async fn assign_task(&self, task: &Task) -> Result<AgentId, AssignmentError>;
    
    pub fn health_check(&self) -> HealthReport;
}
```

#### 3.1.4 MessageBus for Inter-Agent Communication

```rust
// src-tauri/src/agents/message_bus.rs

/// Async message passing between agents
pub struct MessageBus {
    tx: broadcast::Sender<AgentEvent>,
    subscriptions: Arc<RwLock<HashMap<AgentId, Vec<Receiver>>>>,
    dead_letter_queue: Arc<Mutex<Vec<DeadLetter>>>,
}

impl MessageBus {
    pub fn new() -> Self;
    
    pub async fn publish(&self, event: AgentEvent);
    
    pub fn subscribe(&self, agent_id: AgentId) -> Receiver;
    
    pub async fn send(&self, to: AgentId, message: AgentMessage) -> Result<(), SendError>;
    
    pub async fn request_response(&self, to: AgentId, message: AgentMessage, timeout: Duration) -> Result<AgentMessage, TimeoutError>;
}
```

#### 3.1.5 DirectorAgent Implementation

```rust
// src-tauri/src/agents/director.rs

/// Orchestrates task decomposition and agent coordination
pub struct DirectorAgent {
    base: BaseAgent,
    task_analyzer: TaskAnalyzer,
    dependency_resolver: DependencyResolver,
    parallelizer: TaskParallelizer,
    progress_tracker: ProgressTracker,
}

impl DirectorAgent {
    pub async fn analyze_task(&self, request: &UserRequest) -> Result<TaskPlan, AnalysisError>;
    
    pub async fn decompose(&self, task: &Task) -> Result<Vec<SubTask>, DecompositionError>;
    
    pub async fn assign_subtasks(&self, subtasks: Vec<SubTask>) -> Result<AssignmentResult, AssignmentError>;
    
    pub async fn coordinate_parallel(&self, subtasks: Vec<SubTask>) -> Result<Vec<TaskResult>, CoordinationError>;
    
    pub async fn aggregate_results(&self, results: Vec<TaskResult>) -> Result<AggregatedResult, AggregationError>;
    
    pub async fn monitor_progress(&self, task_id: TaskId) -> impl Stream<Item = ProgressUpdate>;
}
```

#### 3.1.6 Deliverables (Week 6)

| File | Description | Lines Target |
|------|-------------|--------------|
| `src-tauri/src/agents/mod.rs` | Module exports | <100 |
| `src-tauri/src/agents/traits.rs` | Agent trait definitions | <200 |
| `src-tauri/src/agents/base_agent.rs` | BaseAgent implementation | <300 |
| `src-tauri/src/agents/registry.rs` | AgentRegistry service | <350 |
| `src-tauri/src/agents/message_bus.rs` | MessageBus implementation | <300 |
| `src-tauri/src/agents/director.rs` | DirectorAgent implementation | <400 |
| `src-tauri/src/commands/agents.rs` | Tauri commands | <250 |
| `src-tauri/src/agents/README.md` | Module documentation | N/A |

---

### 3.2 PHASE 2.2: Specialized Agents (Week 7)

**Objective:** Implement 6 specialized agents for domain-specific tasks

#### 3.2.1 ResearcherAgent

```rust
// src-tauri/src/agents/researcher.rs

pub struct ResearcherAgent {
    base: BaseAgent,
    web_search: WebSearchClient,
    document_parser: DocumentParser,
    source_evaluator: SourceEvaluator,
    citation_manager: CitationManager,
}

impl ResearcherAgent {
    pub async fn research(&self, topic: &str, depth: ResearchDepth) -> Result<ResearchResult, ResearchError>;
    
    pub async fn search_web(&self, query: &str, options: SearchOptions) -> Result<Vec<SearchResult>, SearchError>;
    
    pub async fn evaluate_sources(&self, sources: &[Source]) -> Vec<SourceCredibility>;
    
    pub async fn synthesize_findings(&self, findings: Vec<Finding>) -> ResearchSynthesis;
}
```

#### 3.2.2 ExecutorAgent

```rust
// src-tauri/src/agents/executor.rs

pub struct ExecutorAgent {
    base: BaseAgent,
    task_queue: Arc<TaskQueue>,
    operation_engine: Arc<FileOperationEngine>,
    command_runner: CommandRunner,
    progress_reporter: ProgressReporter,
}

impl ExecutorAgent {
    pub async fn execute_plan(&self, plan: &ExecutionPlan) -> Result<ExecutionResult, ExecutionError>;
    
    pub async fn run_file_operations(&self, operations: Vec<FileOperation>) -> Result<OpResult, OpError>;
    
    pub async fn run_shell_command(&self, command: &str) -> Result<CommandOutput, CommandError>;
    
    pub async fn handle_rollback(&self, operations: Vec<OpId>) -> Result<(), RollbackError>;
}
```

#### 3.2.3 CreatorAgent

```rust
// src-tauri/src/agents/creator.rs

pub struct CreatorAgent {
    base: BaseAgent,
    template_engine: TemplateEngine,
    content_generator: ContentGenerator,
    style_analyzer: StyleAnalyzer,
    format_converter: FormatConverter,
}

impl CreatorAgent {
    pub async fn create_document(&self, spec: &DocumentSpec) -> Result<CreatedDocument, CreationError>;
    
    pub async fn generate_from_template(&self, template_id: TemplateId, data: &Value) -> Result<GeneratedContent, GenerationError>;
    
    pub async fn apply_style(&self, content: &str, style: &StyleSpec) -> Result<StyledContent, StyleError>;
    
    pub async fn create_presentation(&self, outline: &PresentationOutline) -> Result<Presentation, PresentationError>;
}
```

#### 3.2.4 DesignerAgent

```rust
// src-tauri/src/agents/designer.rs

pub struct DesignerAgent {
    base: BaseAgent,
    layout_engine: LayoutEngine,
    color_manager: ColorManager,
    typography_engine: TypographyEngine,
    asset_manager: AssetManager,
}

impl DesignerAgent {
    pub async fn design_layout(&self, spec: &DesignSpec) -> Result<LayoutDesign, DesignError>;
    
    pub async fn generate_ui(&self, requirements: &UiRequirements) -> Result<UiDesign, UiError>;
    
    pub async fn create_visualization(&self, data_spec: &DataVisualizationSpec) -> Result<Visualization, VizError>;
    
    pub async fn optimize_assets(&self, assets: &[AssetPath]) -> Result<Vec<OptimizedAsset>, OptimizationError>;
}
```

#### 3.2.5 DeveloperAgent

```rust
// src-tauri/src/agents/developer.rs

pub struct DeveloperAgent {
    base: BaseAgent,
    code_analyzer: CodeAnalyzer,
    refactoring_engine: RefactoringEngine,
    test_generator: TestGenerator,
    documentation_generator: DocsGenerator,
    linter: Linter,
}

impl DeveloperAgent {
    pub async fn analyze_codebase(&self, path: &Path) -> Result<CodeAnalysis, AnalysisError>;
    
    pub async fn refactor_code(&self, target: &RefactorTarget, strategy: RefactorStrategy) -> Result<RefactorResult, RefactorError>;
    
    pub async fn generate_tests(&self, target: &CodeTarget, coverage: CoverageTarget) -> Result<GeneratedTests, TestError>;
    
    pub async fn generate_docs(&self, target: &CodeTarget) -> Result<GeneratedDocs, DocsError>;
    
    pub async fn fix_bugs(&self, bugs: &[BugReport]) -> Result<Vec<BugFix>, FixError>;
}
```

#### 3.2.6 AnalystAgent

```rust
// src-tauri/src/agents/analyst.rs

pub struct AnalystAgent {
    base: BaseAgent,
    data_processor: DataProcessor,
    statistics_engine: StatisticsEngine,
    visualization_generator: VizGenerator,
    report_generator: ReportGenerator,
}

impl AnalystAgent {
    pub async fn analyze_data(&self, source: &DataSource, analysis_type: AnalysisType) -> Result<AnalysisResult, AnalysisError>;
    
    pub async fn generate_statistics(&self, dataset: &Dataset) -> Result<Statistics, StatsError>;
    
    pub async fn create_report(&self, analysis: &AnalysisResult, format: ReportFormat) -> Result<Report, ReportError>;
    
    pub async fn detect_anomalies(&self, data: &[DataPoint]) -> Result<Vec<Anomaly>, AnomalyError>;
}
```

#### 3.2.7 Deliverables (Week 7)

| File | Description | Lines Target |
|------|-------------|--------------|
| `src-tauri/src/agents/researcher.rs` | ResearcherAgent | <400 |
| `src-tauri/src/agents/executor.rs` | ExecutorAgent | <350 |
| `src-tauri/src/agents/creator.rs` | CreatorAgent | <350 |
| `src-tauri/src/agents/designer.rs` | DesignerAgent | <350 |
| `src-tauri/src/agents/developer.rs` | DeveloperAgent | <400 |
| `src-tauri/src/agents/analyst.rs` | AnalystAgent | <350 |
| `src-tauri/src/agents/mod.rs` | Updated exports | <150 |

---

### 3.3 PHASE 2.3: Memory System (Week 8)

**Objective:** Implement dual-tier memory architecture

#### 3.3.1 ShortTermMemory (RingBuffer)

```rust
// src-tauri/src/services/memory/short_term.rs

/// In-memory ring buffer for recent context
pub struct ShortTermMemory {
    buffer: CircularBuffer<MemoryEntry>,
    max_entries: usize,
    current_session: SessionId,
    entry_index: AtomicU64,
}

impl ShortTermMemory {
    pub fn new(capacity: usize) -> Self;
    
    pub async fn record(&self, entry: MemoryEntry);
    
    pub async fn retrieve_recent(&self, count: usize) -> Vec<MemoryEntry>;
    
    pub async fn search(&self, query: &str) -> Vec<MemoryEntry>;
    
    pub async fn clear_session(&self);
    
    pub fn capacity(&self) -> usize;
    
    pub fn utilization(&self) -> f64;
}
```

#### 3.3.2 LongTermMemory (LanceDB)

```rust
// src-tauri/src/services/memory/long_term.rs

/// Persistent memory with embeddings
pub struct LongTermMemory {
    db: Arc<LanceDB>,
    embedding_model: Arc<dyn EmbeddingModel>,
    vector_index: Arc<VectorIndex>,
    episodic_store: Arc<EpisodicStore>,
    semantic_store: Arc<SemanticStore>,
    procedural_store: Arc<ProceduralStore>,
}

impl LongTermMemory {
    pub async fn new(config: MemoryConfig) -> Result<Self, MemoryError>;
    
    pub async fn store_episode(&self, episode: Episode) -> Result<EpisodeId, StoreError>;
    
    pub async fn store_semantic(&self, content: &str, metadata: &SemanticMetadata) -> Result<SemanticId, StoreError>;
    
    pub async fn store_procedure(&self, procedure: Procedure) -> Result<ProcedureId, StoreError>;
    
    pub async fn retrieve(&self, query: &str, options: RetrievalOptions) -> Result<RetrievedContext, RetrievalError>;
    
    pub async fn search_embeddings(&self, query: &str, limit: usize) -> Result<Vec<EmbeddingMatch>, SearchError>;
    
    pub async fn get_episodes_by_outcome(&self, outcome: OutcomeType) -> Result<Vec<Episode>, QueryError>;
}
```

#### 3.3.3 Memory Service Module

```rust
// src-tauri/src/services/memory/mod.rs

/// Memory system module
pub mod short_term;
pub mod long_term;

use self::{short_term::ShortTermMemory, long_term::LongTermMemory};

#[derive(Clone)]
pub struct MemorySystem {
    short_term: Arc<ShortTermMemory>,
    long_term: Arc<LongTermMemory>,
    embedding: Arc<dyn EmbeddingModel>,
    config: MemoryConfig,
}

impl MemorySystem {
    pub async fn new(config: MemoryConfig) -> Result<Self, MemoryError>;
    
    pub async fn remember(&self, entry: &MemoryEntry);
    
    pub async fn recall(&self, query: &str, context: RecallContext) -> Result<RecallResult, RecallError>;
    
    pub async fn learn(&self, lesson: &LearnedLesson);
    
    pub async fn forget(&self, criteria: ForgetCriteria);
    
    pub async fn consolidate(&self) -> Result<ConsolidationReport, ConsolidationError>;
}
```

#### 3.3.4 Deliverables (Week 8)

| File | Description | Lines Target |
|------|-------------|--------------|
| `src-tauri/src/services/memory/mod.rs` | Module exports | <100 |
| `src-tauri/src/services/memory/short_term.rs` | RingBuffer implementation | <300 |
| `src-tauri/src/services/memory/long_term.rs` | LanceDB integration | <400 |
| `src-tauri/src/services/memory/embedding.rs` | Embedding service | <250 |
| `src-tauri/src/services/memory/README.md` | Module documentation | N/A |

---

### 3.4 PHASE 2.4: Reflection & Governance (Week 9)

**Objective:** Implement quality assurance and security

#### 3.4.1 CriticAgent

```rust
// src-tauri/src/agents/critic.rs

pub struct CriticAgent {
    base: BaseAgent,
    quality_evaluator: QualityEvaluator,
    accuracy_checker: AccuracyChecker,
    consistency_verifier: ConsistencyVerifier,
    improvement_suggester: ImprovementSuggester,
}

impl CriticAgent {
    pub async fn evaluate_result(&self, result: &AgentResult) -> Result<QualityAssessment, EvaluationError>;
    
    pub async fn check_accuracy(&self, content: &str, expected: &AccuracyCriteria) -> Result<AccuracyReport, AccuracyError>;
    
    pub async fn verify_consistency(&self, content: &str) -> Result<ConsistencyReport, ConsistencyError>;
    
    pub async fn suggest_improvements(&self, assessment: &QualityAssessment) -> Vec<ImprovementSuggestion>;
    
    pub async fn rate_task_performance(&self, task: &Task, result: &AgentResult) -> PerformanceRating;
}
```

#### 3.4.2 GovernorAgent

```rust
// src-tauri/src/agents/governor.rs

pub struct GovernorAgent {
    base: BaseAgent,
    policy_engine: PolicyEngine,
    security_scanner: SecurityScanner,
    operation_validator: OperationValidator,
    audit_logger: AuditLogger,
}

impl GovernorAgent {
    pub async fn validate_operation(&self, operation: &Operation) -> Result<ValidationResult, ValidationError>;
    
    pub async fn check_dangerous_patterns(&self, command: &str) -> Result<SecurityCheckResult, SecurityError>;
    
    pub async fn enforce_policy(&self, context: &PolicyContext) -> Result<PolicyDecision, PolicyError>;
    
    pub async fn log_audit(&self, event: &AuditEvent);
    
    pub async fn block_operation(&self, operation: &Operation, reason: &str) -> Result<BlockedOperation, BlockError>;
    
    pub fn get_security_policy(&self) -> SecurityPolicy;
}
```

#### 3.4.3 Reflection Engine

```rust
// src-tauri/src/services/reflection.rs

/// Self-improvement through reflection
pub struct ReflectionEngine {
    episode_store: Arc<EpisodeStore>,
    pattern_detector: PatternDetector,
    strategy_optimizer: StrategyOptimizer,
    improvement_tracker: ImprovementTracker,
}

impl ReflectionEngine {
    pub async fn analyze_task_completion(&self, task: &Task, result: &TaskResult) -> Result<ReflectionAnalysis, AnalysisError>;
    
    pub async fn detect_error_patterns(&self) -> Result<Vec<ErrorPattern>, DetectionError>;
    
    pub async fn optimize_strategy(&self, strategy_id: StrategyId) -> Result<OptimizedStrategy, OptimizationError>;
    
    pub async fn generate_insights(&self, analysis: &ReflectionAnalysis) -> Vec<Insight>;
    
    pub async fn update_agent_behavior(&self, agent_id: AgentId, improvements: &[BehaviorImprovement]) -> Result<(), UpdateError>;
}
```

#### 3.4.4 Deliverables (Week 9)

| File | Description | Lines Target |
|------|-------------|--------------|
| `src-tauri/src/agents/critic.rs` | CriticAgent implementation | <350 |
| `src-tauri/src/agents/governor.rs` | GovernorAgent implementation | <400 |
| `src-tauri/src/services/reflection.rs` | Reflection engine | <350 |
| `src-tauri/src/services/mod.rs` | Updated exports | <100 |

---

## 4. Backend Implementation Details

### 4.1 New Rust Services

| Service | Module | Responsibility | Dependencies |
|---------|--------|---------------|--------------|
| AgentRegistry | `agents/registry.rs` | Agent lifecycle management | MessageBus, Metrics |
| MemorySystem | `services/memory/mod.rs` | Unified memory interface | ShortTerm, LongTerm |
| ReflectionEngine | `services/reflection.rs` | Self-improvement | EpisodeStore, PatternDetector |
| TaskAnalyzer | `agents/analyzer.rs` | Task decomposition | None |

### 4.2 New Tauri Commands

```rust
// src-tauri/src/commands/agents.rs

#[tauri::command]
pub async fn register_agent(
    agent_type: AgentType,
    name: String,
    state: State<'_, AgentState>,
) -> Result<AgentRegistration, CommandError>;

#[tauri::command]
pub async fn assign_task(
    task: TaskSpec,
    preferred_agents: Option<Vec<AgentType>>,
    state: State<'_, AgentState>,
) -> Result<TaskAssignment, CommandError>;

#[tauri::command]
pub async fn get_agent_status(
    agent_id: Option<AgentId>,
    state: State<'_, AgentState>,
) -> Result<AgentStatusReport, CommandError>;

#[tauri::command]
pub async fn query_memory(
    query: String,
    options: RetrievalOptions,
    state: State<'_, MemoryState>,
) -> Result<MemoryQueryResult, CommandError>;

// src-tauri/src/commands/memory.rs

#[tauri::command]
pub async fn store_memory(
    entry: MemoryEntry,
    memory_type: MemoryType,
    state: State<'_, MemoryState>,
) -> Result<StorageConfirmation, CommandError>;

#[tauri::command]
pub async fn recall_context(
    context_size: usize,
    state: State<'_, MemoryState>,
) -> Result<ContextBundle, CommandError>;
```

### 4.3 Data Models

```rust
// src-tauri/src/types/agent.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentId(Uuid);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentType {
    Director,
    Critic,
    Governor,
    Researcher,
    Executor,
    Creator,
    Designer,
    Developer,
    Analyst,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentContext {
    pub task_id: TaskId,
    pub user_id: Option<UserId>,
    pub workspace_id: WorkspaceId,
    pub memory_context: MemoryContext,
    pub constraints: TaskConstraints,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPlan {
    pub id: TaskId,
    pub original_request: String,
    pub subtasks: Vec<SubTask>,
    pub dependencies: Vec<TaskDependency>,
    pub estimated_duration: Duration,
    pub parallelizable: bool,
}

// src-tauri/src/types/memory.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: MemoryId,
    pub timestamp: DateTime<Utc>,
    pub content: String,
    pub entry_type: MemoryType,
    pub importance: ImportanceScore,
    pub metadata: MemoryMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryType {
    ShortTerm { session_id: SessionId },
    Episodic { episode_id: EpisodeId },
    Semantic { document_id: DocumentId },
    Procedural { procedure_id: ProcedureId },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryContext {
    pub recent_entries: Vec<MemoryEntry>,
    pub relevant_memories: Vec<MemoryEntry>,
    pub learned_lessons: Vec<Lesson>,
}
```

---

## 5. Frontend Implementation Details

### 5.1 New TypeScript Types

```typescript
// src/types/agent.ts

export type AgentId = string;
export type AgentType = 
  | 'director' 
  | 'critic' 
  | 'governor' 
  | 'researcher' 
  | 'executor' 
  | 'creator' 
  | 'designer' 
  | 'developer' 
  | 'analyst';

export interface Agent {
  id: AgentId;
  type: AgentType;
  name: string;
  status: AgentStatus;
  capabilities: string[];
  currentTask?: TaskId;
  metrics: AgentMetrics;
}

export type AgentStatus = 'idle' | 'busy' | 'blocked' | 'error';

export interface TaskPlan {
  id: TaskId;
  subtasks: SubTask[];
  estimatedDuration: number;
  parallelizable: boolean;
}

export interface SubTask {
  id: SubTaskId;
  description: string;
  assignedAgent: AgentType;
  status: SubTaskStatus;
  dependencies: SubTaskId[];
  progress: number;
  result?: TaskResult;
}

export interface AgentEvent {
  type: 'task_assigned' | 'task_completed' | 'progress_update' | 'error' | 'blocked';
  agentId: AgentId;
  timestamp: Date;
  payload: unknown;
}

// src/types/memory.ts

export interface MemoryEntry {
  id: string;
  content: string;
  type: 'short_term' | 'episodic' | 'semantic' | 'procedural';
  importance: number;
  timestamp: Date;
  metadata: MemoryMetadata;
}

export interface MemoryQuery {
  query: string;
  limit: number;
  filters?: MemoryFilter[];
}

export interface MemoryFilter {
  type?: MemoryType[];
  dateRange?: { start: Date; end: Date };
  importance?: { min: number; max: number };
}
```

### 5.2 New React Components

| Component | File | Purpose |
|-----------|------|---------|
| AgentRegistry | `components/agents/AgentRegistry.tsx` | Display available agents |
| MultiAgentPanel | `components/agents/MultiAgentPanel.tsx` | Multi-agent chat interface |
| AgentActivityFeed | `components/agents/ActivityFeed.tsx` | Real-time agent events |
| AgentCard | `components/agents/AgentCard.tsx` | Agent status card |
| TaskDecomposition | `components/agents/TaskDecomposition.tsx` | Show task breakdown |
| MemoryVisualizer | `components/agents/MemoryVisualizer.tsx` | Memory usage display |

### 5.3 New Hooks

```typescript
// src/hooks/useMultiAgent.ts

export function useMultiAgent() {
  const [agents, setAgents] = useState<Agent[]>([]);
  const [activeTask, setActiveTask] = useState<TaskPlan | null>(null);
  const [events, setEvents] = useState<AgentEvent[]>([]);
  
  const registerAgent = useCallback(async (type: AgentType, name: string) => {
    return await tauri.invoke('register_agent', { type, name });
  }, []);
  
  const assignTask = useCallback(async (task: TaskSpec) => {
    return await tauri.invoke('assign_task', { task });
  }, []);
  
  const subscribeToEvents = useCallback((agentId: AgentId) => {
    return tauri.listen(`agent-event-${agentId}`, (event) => {
      setEvents(prev => [...prev, event.payload as AgentEvent]);
    });
  }, []);
  
  return {
    agents,
    activeTask,
    events,
    registerAgent,
    assignTask,
    subscribeToEvents,
  };
}

// src/hooks/useAgentRegistry.ts

export function useAgentRegistry() {
  const [agents, setAgents] = useState<Agent[]>([]);
  const [metrics, setMetrics] = useState<RegistryMetrics | null>(null);
  
  const refresh = useCallback(async () => {
    const [agentList, status] = await Promise.all([
      tauri.invoke<Agent[]>('list_agents'),
      tauri.invoke<RegistryMetrics>('get_registry_status'),
    ]);
    setAgents(agentList);
    setMetrics(status);
  }, []);
  
  useEffect(() => {
    refresh();
    const interval = setInterval(refresh, 5000);
    return () => clearInterval(interval);
  }, [refresh]);
  
  return { agents, metrics, refresh };
}
```

### 5.4 Updated Services

```typescript
// src/services/tauri.ts

// Existing commands preserved
export async function coworkChat(message: string) { ... }

// New multi-agent commands
export async function multiAgentChat(message: string, options?: MultiAgentOptions) {
  return invokeWithResult('multi_agent_chat', { message, options });
}

export async function getAgentStatus(agentId?: string) {
  return invokeWithResult('get_agent_status', { agentId });
}

export async function queryMemory(query: string, options?: MemoryQueryOptions) {
  return invokeWithResult('query_memory', { query, options });
}

export async function storeMemory(entry: MemoryEntry, type: MemoryType) {
  return invokeWithResult('store_memory', { entry, type });
}

export async function getReflectionInsights(taskId: string) {
  return invokeWithResult('get_reflection_insights', { taskId });
}
```

---

## 6. Testing Strategy

### 6.1 Unit Tests (per module)

```rust
// src-tauri/tests/agents/director_tests.rs

#[cfg(test)]
mod director_tests {
    use super::*;
    use rstest::rstest;
    
    #[rstest]
    #[case::simple_task("Write a README", 1)]
    #[case::complex_task("Create a full-stack web app with auth", 5)]
    fn test_task_decomposition(
        #[case] request: &str,
        #[case] expected_subtasks: usize,
    ) {
        let rt = Runtime::new().unwrap();
        let result = rt.block_on(async {
            let director = create_test_director();
            director.analyze_task(&request).await
        });
        
        assert!(result.is_ok());
        let plan = result.unwrap();
        assert_eq!(plan.subtasks.len(), expected_subtasks);
    }
    
    #[tokio::test]
    async fn test_parallel_coordination() {
        let director = create_test_director();
        let subtasks = create_independent_subtasks(4);
        
        let results = director.coordinate_parallel(subtasks).await;
        
        assert!(results.is_ok());
        assert_eq!(results.unwrap().len(), 4);
    }
}

// src-tauri/tests/memory/memory_tests.rs

#[cfg(test)]
mod memory_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_short_term_ring_buffer() {
        let memory = ShortTermMemory::new(10);
        
        for i in 0..15 {
            memory.record(create_test_entry(i)).await;
        }
        
        let recent = memory.retrieve_recent(10).await;
        assert_eq!(recent.len(), 10);
        // Should contain entries 5-14
        assert!(recent.iter().all(|e| e.id >= 5));
    }
    
    #[tokio::test]
    async fn test_long_term_embedding_search() {
        let memory = create_test_long_term_memory();
        
        // Store some documents
        memory.store_semantic("Rust async programming", &Metadata::default()).await;
        memory.store_semantic("TypeScript generics", &Metadata::default()).await;
        
        // Search
        let results = memory.search_embeddings("async await", 5).await;
        
        assert!(!results.is_empty());
        assert!(results[0].score > 0.7);
    }
}
```

### 6.2 Integration Tests

```rust
// src-tauri/tests/integration/multi_agent_tests.rs

#[tokio::test]
async fn test_full_task_lifecycle() {
    // Setup
    let registry = AgentRegistry::new();
    let memory = create_test_memory();
    let director = DirectorAgent::new(&registry, &memory);
    
    // Execute complex task
    let task = create_complex_task();
    let result = director.execute(task).await;
    
    // Verify
    assert!(result.is_ok());
    verify_all_subtasks_completed(&result);
    verify_memory_updated(&memory);
    verify_audit_logged();
}

#[tokio::test]
async fn test_agent_coordination() {
    // Test that multiple agents can work in parallel
    // and DirectorAgent properly handles dependencies
}
```

### 6.3 Frontend Tests

```typescript
// src/components/agents/AgentCard.test.tsx

import { render, screen, fireEvent } from '@testing-library/react';
import { AgentCard } from './AgentCard';

describe('AgentCard', () => {
  it('shows agent status correctly', () => {
    render(<AgentCard agent={mockAgent('busy')} />);
    expect(screen.getByText('Busy')).toBeInTheDocument();
  });
  
  it('handles click for details', () => {
    const onClick = jest.fn();
    render(<AgentCard agent={mockAgent()} onClick={onClick} />);
    fireEvent.click(screen.getByRole('button'));
    expect(onClick).toHaveBeenCalled();
  });
});

// src/hooks/useMultiAgent.test.ts

import { renderHook, waitFor } from '@testing-library/react';
import { useMultiAgent } from './useMultiAgent';

describe('useMultiAgent', () => {
  it('initializes with empty agents', () => {
    const { result } = renderHook(() => useMultiAgent());
    expect(result.current.agents).toEqual([]);
  });
  
  it('registers agent successfully', async () => {
    const { result } = renderHook(() => useMultiAgent());
    
    await waitFor(() => {
      expect(result.current.registerAgent).toBeDefined();
    });
  });
});
```

---

## 7. Migration & Compatibility

### 7.1 Backward Compatibility

The existing `CoworkAgent` will be preserved and wrapped by the new multi-agent system:

```rust
// Legacy CoworkAgent wrapped for compatibility
pub struct LegacyCoworkAgentAdapter {
    inner: CoworkAgent,
    message_bus: Arc<MessageBus>,
}

#[async_trait::async_trait]
impl Agent for LegacyCoworkAgentAdapter {
    fn agent_type(&self) -> AgentType {
        AgentType::Executor // Mapped to Executor role
    }
    
    async fn execute(&self, context: &AgentContext) -> Result<AgentResult, AgentError> {
        // Convert to legacy format
        let legacy_context = convert_context(context);
        let result = self.inner.execute(legacy_context).await?;
        // Convert back
        Ok(convert_result(result))
    }
}
```

### 7.2 Gradual Migration Path

```
Phase 2.1 (Week 6)     │ Phase 2.2 (Week 7)     │ Phase 2.3 (Week 8)
───────────────────────┼────────────────────────┼─────────────────────
├── AgentRegistry      │ ├── ResearcherAgent    │ ├── ShortTermMemory
├── BaseAgent          │ ├── ExecutorAgent      │ ├── LongTermMemory
├── DirectorAgent      │ ├── CreatorAgent       │ ├── EmbeddingService
├── MessageBus         │ ├── DesignerAgent      │ └── MemorySystem
└── Legacy Adapter     │ ├── DeveloperAgent     │
                       │ ├── AnalystAgent       │
                       │ └── Legacy Adapter     │
```

### 7.3 Feature Flags

```rust
// src-tauri/src/config/features.rs

#[derive(Clone)]
pub struct FeatureFlags {
    pub multi_agent: bool,
    pub memory_system: bool,
    pub reflection: bool,
    pub critic_agent: bool,
    pub governor_agent: bool,
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            multi_agent: false, // Enable in 2.2
            memory_system: false, // Enable in 2.3
            reflection: false, // Enable in 2.4
            critic_agent: false, // Enable in 2.4
            governor_agent: false, // Enable in 2.4
        }
    }
}
```

---

## 8. Dependencies

### 8.1 New Rust Crates

| Crate | Version | Purpose |
|-------|---------|---------|
| `lance` | ^0.20 | Vector database for embeddings |
| `rig-core` | ^0.3 | RAG and embedding utilities |
| `async-trait` | ^0.1 | Trait objects for agents |
| `tokio-stream` | ^0.1 | Stream utilities for progress |
| `uuid` | ^1.10 | Agent/Task ID generation |

### 8.2 Updated Cargo.toml

```toml
[dependencies]
# Existing
tauri = { version = "2", features = ["shell-open"] }
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["full"] }

# New in PHASE 2
lance = "0.20"
rig-core = "0.3"
async-trait = "0.1"
tokio-stream = "0.1"
uuid = { version = "1.10", features = ["v4", "fast-rng"] }

[dev-dependencies]
rstest = "0.23"
wiremock = "0.11"
```

### 8.3 New npm Packages

| Package | Version | Purpose |
|---------|---------|---------|
| `@types/uuid` | ^10 | TypeScript UUID types |

### 8.4 Build System Updates

```bash
# Required system dependencies for LanceDB
# Linux
sudo apt-get install libclang-dev libssl-dev pkg-config

# macOS
xcode-select --install

# Windows (via vcpkg)
vcpkg install openssl:x64-windows-static
```

---

## 9. Risks & Mitigation

### 9.1 Technical Risks

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| LanceDB integration complexity | High | Medium | Start with SQLite fallback, migrate to LanceDB post-validation |
| Agent coordination deadlock | High | Low | Implement timeout and deadlock detection |
| Memory bloat | Medium | Medium | Implement aggressive eviction policies |
| Embedding model performance | Medium | Medium | Use lightweight models, cache results |

### 9.2 Performance Considerations

| Concern | Target | Mitigation |
|---------|--------|------------|
| Agent message latency | <100ms | Use broadcast channel with bounded buffers |
| Memory retrieval | <50ms | Pre-compute embeddings, cache hot entries |
| Task decomposition | <200ms | Parallel analysis strategies |
| Database queries | <100ms | Indexed vector search, connection pooling |

### 9.3 Security Considerations

| Threat | Mitigation |
|--------|--------------------|
| Malicious agent messages | Validate all inter-agent messages |
| Memory injection | Sanitize memory entries before storage |
| Unauthorized task access | Use workspace context for authorization |
| Dangerous operations | GovernorAgent blocks all file operations |

---

## 10. Success Metrics

### 10.1 Performance Targets

| Metric | Target | Measurement |
|--------|--------|-------------|
| Task decomposition time | <200ms | Average over 1000 tasks |
| Agent assignment accuracy | >90% | DirectorAgent correctness |
| Memory retrieval relevance | >85% | User feedback scores |
| System uptime | >99.5% | Per month availability |
| Error recovery time | <5min | Mean time to recovery |

### 10.2 Quality Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Code review pass rate | >90% | First-pass reviews |
| Test coverage | >80% | Per module coverage |
| Documentation completeness | 100% | Required docs present |
| Circular dependency violations | 0 | CI checks |

### 10.3 User Experience Goals

| Goal | Metric | Target |
|------|--------|--------|
| Task completion rate | % tasks completed | >85% |
| Agent switching | % using multi-agent | >50% after 1 month |
| Memory utilization | % recalling context | >70% find useful |

---

## 11. Appendices

### 11.1 Module Dependency Graph

```
agents/
├── traits.rs ─────────────►
├── base_agent.rs ─────────► [traits, message_bus, memory]
├── registry.rs ───────────► [agents, metrics]
├── director.rs ───────────► [base, analyzer, parallelizer]
├── researcher.rs ─────────► [base, web_search]
├── executor.rs ───────────► [base, task_queue]
├── creator.rs ────────────► [base, templates]
├── designer.rs ───────────► [base, layout_engine]
├── developer.rs ──────────► [base, code_analyzer]
├── analyst.rs ────────────► [base, statistics]
├── critic.governor.rs ────► [base, policies]
└── README.md

services/memory/
├── short_term.rs ─────────► [RingBuffer]
├── long_term.rs ──────────► [LanceDB, embeddings]
└── README.md
```

### 11.2 File Change Summary

| Phase | Files Created | Files Modified |
|-------|---------------|----------------|
| 2.1 | 7 | 2 (`mod.rs`, `lib.rs`) |
| 2.2 | 6 | 1 (`mod.rs`) |
| 2.3 | 4 | 2 (`mod.rs`, `lib.rs`) |
| 2.4 | 3 | 2 (`mod.rs`, `lib.rs`) |
| **Total** | **20** | **7** |

### 11.3 Definition of Done

For each module, DONE means:
- [ ] Code implementation complete
- [ ] Unit tests passing (>80% coverage)
- [ ] Integration tests passing
- [ ] Documentation written (README, inline docs)
- [ ] No linting errors
- [ ] No circular dependencies
- [ ] Code reviewed and approved
- [ ] Feature flag configured
- [ ] Migration plan documented

---

## Document Version History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2026-01-27 | Architecture Team | Initial draft |

---

*This document follows the modularization rules defined in [.kilocode/rules/priorize-the-modularization.md](../.kilocode/rules/priorize-the-modularization.md). Each module implements single responsibility, exports explicit interfaces, includes tests, and maintains no circular dependencies.*
