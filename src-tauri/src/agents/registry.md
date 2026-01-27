# AgentRegistry Module

## Purpose

The AgentRegistry service provides centralized management of all agents in the multi-agent system. It handles agent registration, lifecycle management, task assignment, and coordination between agents.

## Public Interface

### Main Struct

- **`AgentRegistry`**: Central registry for managing all agents

### Key Methods

#### Agent Management
- `new(ai_provider: Arc<AIProviderManager>) -> Self` - Create a new registry
- `register_agent(agent: Arc<dyn Agent>, config: AgentConfig) -> Result<(), AgentError>` - Register a new agent
- `unregister_agent(agent_id: &str) -> Result<(), AgentError>` - Unregister an agent
- `get_agent(agent_id: &str) -> Option<Arc<dyn Agent>>` - Get agent by ID
- `list_agents() -> Vec<AgentInfo>` - List all registered agents

#### Task Management
- `assign_task(task: Task) -> Result<String, AgentError>` - Assign task to appropriate agent
- `cancel_task(task_id: &str) -> Result<(), AgentError>` - Cancel a task
- `get_task_agent(task_id: &str) -> Option<String>` - Get agent assigned to a task

#### Status Monitoring
- `get_agent_status(agent_id: &str) -> Option<AgentStatus>` - Get agent status
- `get_busy_agents() -> Vec<AgentInfo>` - Get all busy agents
- `get_idle_agents() -> Vec<AgentInfo>` - Get all idle agents
- `get_statistics() -> RegistryStatistics` - Get registry statistics

#### Coordination
- `coordinate_agents(task: Task) -> Result<Vec<String>, AgentError>` - Coordinate multiple agents
- `broadcast_message(message: AgentMessage)` - Broadcast message to all agents

#### Accessors
- `message_bus() -> Arc<MessageBus>` - Get message bus reference
- `ai_provider() -> Arc<AIProviderManager>` - Get AI provider reference

### Supporting Types

- **`RegistryStatistics`**: Statistics about the registry
  - `total_agents: usize` - Total number of registered agents
  - `idle_agents: usize` - Number of idle agents
  - `busy_agents: usize` - Number of busy agents
  - `error_agents: usize` - Number of agents in error state
  - `active_tasks: usize` - Number of currently active tasks

## Usage Examples

### Basic Registration

```rust
use rainy_cowork_lib::agents::{AgentRegistry, AgentConfig, BaseAgent};

let ai_provider = Arc::new(AIProviderManager::new());
let registry = AgentRegistry::new(ai_provider);

let config = AgentConfig {
    agent_id: "researcher-1".to_string(),
    workspace_id: "workspace-1".to_string(),
    ai_provider: "gemini".to_string(),
    model: "gemini-2.0-flash".to_string(),
    settings: serde_json::json!({}),
};

let message_bus = registry.message_bus();
let agent = Arc::new(BaseAgent::new(config.clone(), registry.ai_provider(), message_bus));

registry.register_agent(agent, config).await?;
```

### Task Assignment

```rust
use rainy_cowork_lib::agents::{Task, TaskContext, TaskPriority};

let task = Task {
    id: "task-1".to_string(),
    description: "Research the latest AI trends".to_string(),
    priority: TaskPriority::High,
    dependencies: vec![],
    context: TaskContext {
        workspace_id: "workspace-1".to_string(),
        user_instruction: "Research AI trends".to_string(),
        relevant_files: vec![],
        memory_context: vec![],
    },
};

let agent_id = registry.assign_task(task).await?;
println!("Task assigned to agent: {}", agent_id);
```

### Status Monitoring

```rust
// Get all agents
let agents = registry.list_agents().await;

// Get idle agents
let idle_agents = registry.get_idle_agents().await;

// Get busy agents
let busy_agents = registry.get_busy_agents().await;

// Get statistics
let stats = registry.get_statistics();
println!("Total agents: {}", stats.total_agents);
println!("Idle agents: {}", stats.idle_agents);
println!("Busy agents: {}", stats.busy_agents);
```

### Agent Coordination

```rust
// Coordinate multiple agents for a task
let participating_agents = registry.coordinate_agents(task).await?;
println!("Participating agents: {:?}", participating_agents);

// Broadcast message to all agents
use rainy_cowork_lib::agents::AgentMessage;
let message = AgentMessage::QueryMemory {
    query: "What are the latest trends?".to_string(),
};
registry.broadcast_message(message).await;
```

## Architecture

The AgentRegistry uses:

- **DashMap**: Thread-safe concurrent hash maps for agent storage
- **Arc**: Shared ownership for thread-safe agent references
- **tokio::spawn**: Asynchronous task execution
- **MessageBus**: Inter-agent communication

### Data Structures

```rust
pub struct AgentRegistry {
    agents: DashMap<String, Arc<dyn Agent>>,           // Agent storage
    agent_configs: DashMap<String, AgentConfig>,       // Configuration storage
    task_assignments: DashMap<String, String>,         // Task -> Agent mapping
    message_bus: Arc<MessageBus>,                      // Communication
    ai_provider: Arc<AIProviderManager>,              // AI operations
}
```

## Task Assignment Algorithm

1. Iterate through all registered agents
2. Check if agent can handle the task (`can_handle()`)
3. Check if agent is idle (`AgentStatus::Idle`)
4. Assign to first matching agent
5. Update agent status to `Busy`
6. Track task assignment
7. Execute task asynchronously
8. Update status to `Idle` on completion

## Error Handling

The registry returns `AgentError` for various failure scenarios:

- `TaskExecutionFailed`: Task processing errors
- `InvalidConfig`: Configuration errors (e.g., duplicate agent ID)
- `Io`: I/O operation errors
- `Serialization`: JSON serialization errors

## Thread Safety

The AgentRegistry is fully thread-safe:

- All operations use DashMap for concurrent access
- Agents are wrapped in Arc for shared ownership
- Task execution is spawned in separate tokio tasks
- Status updates are atomic

## Performance Considerations

- DashMap provides lock-free concurrent access
- Task execution is non-blocking (async)
- Statistics collection is O(n) where n is number of agents
- Agent lookup is O(1) average case

## Testing

The module includes comprehensive unit tests covering:

- Registry creation
- Agent registration and unregistration
- Duplicate registration prevention
- Agent retrieval and listing
- Task assignment and cancellation
- Status monitoring
- Statistics collection
- Registry cloning

Run tests with:
```bash
cargo test --package rainy-cowork-lib --lib agents::registry
```

## Migration Notes

This is a new module in PHASE 2.1. No migration is required.

## Future Enhancements

Potential improvements:

1. **Task Prioritization**: Implement priority-based task queuing
2. **Load Balancing**: Distribute tasks more evenly across agents
3. **Agent Health Checks**: Monitor agent health and auto-recovery
4. **Task Dependencies**: Handle task dependencies and execution order
5. **Metrics Collection**: Detailed performance metrics
6. **Agent Pooling**: Pre-allocate agent pools for common tasks
7. **Circuit Breakers**: Prevent cascading failures
8. **Task Timeouts**: Automatic task cancellation on timeout
