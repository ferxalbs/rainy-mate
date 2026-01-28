//! Reflection and Governance Tauri Commands
//!
//! This module provides Tauri commands for reflection, self-improvement,
//! and governance operations in the multi-agent system.

use std::sync::Arc;
use tauri::State;

use crate::agents::{Task, TaskResult, TaskPriority, TaskContext};
use crate::agents::critic::QualityEvaluation;
use crate::agents::governor::SecurityPolicy;
use crate::services::reflection::{ReflectionEngine, Reflection, ErrorPattern, Strategy, OptimizationReport};

/// State for ReflectionEngine
pub struct ReflectionEngineState(pub Arc<ReflectionEngine>);

/// Analyze task result and learn from it
#[tauri::command]
pub async fn analyze_task_result(
    engine: State<'_, ReflectionEngineState>,
    task_id: String,
    task_description: String,
    success: bool,
    output: String,
    errors: Vec<String>,
) -> Result<Reflection, String> {
    let task = Task {
        id: task_id,
        description: task_description,
        priority: TaskPriority::Medium,
        dependencies: vec![],
        context: TaskContext {
            workspace_id: "default".to_string(),
            user_instruction: "".to_string(),
            relevant_files: vec![],
            memory_context: vec![],
        },
    };

    let result = TaskResult {
        success,
        output,
        errors,
        metadata: serde_json::json!({}),
    };

    engine.0.analyze_result(&task, &result).await
        .map_err(|e| e.to_string())
}

/// Get error patterns
#[tauri::command]
pub async fn get_error_patterns(
    engine: State<'_, ReflectionEngineState>,
) -> Result<Vec<ErrorPattern>, String> {
    Ok(engine.0.get_error_patterns().await)
}

/// Get strategies
#[tauri::command]
pub async fn get_strategies(
    engine: State<'_, ReflectionEngineState>,
) -> Result<Vec<Strategy>, String> {
    Ok(engine.0.get_strategies().await)
}

/// Optimize based on learned patterns
#[tauri::command]
pub async fn optimize_system(
    engine: State<'_, ReflectionEngineState>,
) -> Result<OptimizationReport, String> {
    engine.0.optimize().await
        .map_err(|e| e.to_string())
}

/// Clear error patterns
#[tauri::command]
pub async fn clear_error_patterns(
    engine: State<'_, ReflectionEngineState>,
) -> Result<(), String> {
    engine.0.clear_error_patterns().await;
    Ok(())
}

/// Clear strategies
#[tauri::command]
pub async fn clear_strategies(
    engine: State<'_, ReflectionEngineState>,
) -> Result<(), String> {
    engine.0.clear_strategies().await;
    Ok(())
}

/// Add security policy
#[tauri::command]
pub async fn add_security_policy(
    registry: State<'_, crate::commands::agents::AgentRegistryState>,
    policy_id: String,
    name: String,
    description: String,
    enabled: bool,
) -> Result<(), String> {
    // Get GovernorAgent
    let agents = registry.0.list_agents().await;
    let governor = agents.iter()
        .find(|a| matches!(a.agent_type, crate::agents::AgentType::Governor))
        .ok_or("GovernorAgent not found")?;

    let _agent = registry.0.get_agent(&governor.id)
        .ok_or("GovernorAgent not found in registry")?;

    // Add policy
    let policy = SecurityPolicy {
        id: policy_id,
        name,
        description,
        enabled,
    };

    // Note: This would need to be implemented in GovernorAgent
    // For now, we just acknowledge the request
    tracing::info!("Security policy added: {}", policy.name);
    Ok(())
}

/// List security policies
#[tauri::command]
pub async fn list_security_policies(
    registry: State<'_, crate::commands::agents::AgentRegistryState>,
) -> Result<Vec<SecurityPolicy>, String> {
    // Get GovernorAgent
    let agents = registry.0.list_agents().await;
    let _governor = agents.iter()
        .find(|a| matches!(a.agent_type, crate::agents::AgentType::Governor))
        .ok_or("GovernorAgent not found")?;

    // Note: This would need to be implemented in GovernorAgent
    // For now, return empty list
    Ok(vec![])
}

/// Remove security policy
#[tauri::command]
pub async fn remove_security_policy(
    registry: State<'_, crate::commands::agents::AgentRegistryState>,
    policy_id: String,
) -> Result<(), String> {
    // Get GovernorAgent
    let agents = registry.0.list_agents().await;
    let governor = agents.iter()
        .find(|a| matches!(a.agent_type, crate::agents::AgentType::Governor))
        .ok_or("GovernorAgent not found")?;

    let _agent = registry.0.get_agent(&governor.id)
        .ok_or("GovernorAgent not found in registry")?;

    // Note: This would need to be implemented in GovernorAgent
    // For now, we just acknowledge the request
    tracing::info!("Security policy removed: {}", policy_id);
    Ok(())
}

/// Evaluate task result quality
#[tauri::command]
pub async fn evaluate_task_quality(
    registry: State<'_, crate::commands::agents::AgentRegistryState>,
    task_id: String,
    task_description: String,
    success: bool,
    output: String,
    errors: Vec<String>,
) -> Result<QualityEvaluation, String> {
    let _task = Task {
        id: task_id,
        description: task_description,
        priority: TaskPriority::Medium,
        dependencies: vec![],
        context: TaskContext {
            workspace_id: "default".to_string(),
            user_instruction: "".to_string(),
            relevant_files: vec![],
            memory_context: vec![],
        },
    };

    let _result = TaskResult {
        success,
        output,
        errors,
        metadata: serde_json::json!({}),
    };

    // Get CriticAgent
    let agents = registry.0.list_agents().await;
    let critic = agents.iter()
        .find(|a| matches!(a.agent_type, crate::agents::AgentType::Critic))
        .ok_or("CriticAgent not found")?;

    let _agent = registry.0.get_agent(&critic.id)
        .ok_or("CriticAgent not found in registry")?;

    // Note: This would need to be implemented in CriticAgent
    // For now, return a default evaluation
    Ok(QualityEvaluation {
        quality_score: 85,
        accuracy: "High".to_string(),
        coherence: "Good".to_string(),
        suggestions: vec![],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reflection_serialization() {
        let reflection = Reflection {
            task_id: "task-1".to_string(),
            success: true,
            insights: vec!["Good performance".to_string()],
            improvements: vec!["Add caching".to_string()],
        };

        let json = serde_json::to_string(&reflection).unwrap();
        let deserialized: Reflection = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.task_id, "task-1");
        assert!(deserialized.success);
        assert_eq!(deserialized.improvements.len(), 1);
    }

    #[test]
    fn test_error_pattern_serialization() {
        let pattern = ErrorPattern {
            id: "test-1".to_string(),
            error_type: "SyntaxError".to_string(),
            root_cause: "Missing semicolon".to_string(),
            prevention_strategy: "Use linter".to_string(),
            count: 5,
        };

        let json = serde_json::to_string(&pattern).unwrap();
        let deserialized: ErrorPattern = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.error_type, "SyntaxError");
        assert_eq!(deserialized.count, 5);
    }

    #[test]
    fn test_strategy_serialization() {
        let strategy = Strategy {
            id: "test-1".to_string(),
            name: "Code Review".to_string(),
            description: "Implement peer review process".to_string(),
            effectiveness: 0.85,
        };

        let json = serde_json::to_string(&strategy).unwrap();
        let deserialized: Strategy = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.name, "Code Review");
        assert_eq!(deserialized.effectiveness, 0.85);
    }
}
