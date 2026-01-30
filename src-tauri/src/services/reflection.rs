//! ReflectionEngine - Self-improvement and optimization
//!
//! The ReflectionEngine analyzes task results, learns from errors and successes,
//! and provides optimization strategies for continuous system improvement.

use std::sync::Arc;
use crate::agents::{Task, TaskResult, AgentError};
use crate::models::ProviderType;

/// ReflectionEngine for self-improvement and optimization
pub struct ReflectionEngine {
    ai_provider: Arc<crate::ai::provider::AIProviderManager>,
    error_patterns: Arc<tokio::sync::RwLock<Vec<ErrorPattern>>>,
    strategies: Arc<tokio::sync::RwLock<Vec<Strategy>>>,
}

impl ReflectionEngine {
    /// Create a new ReflectionEngine instance
    pub fn new(ai_provider: Arc<crate::ai::provider::AIProviderManager>) -> Self {
        Self {
            ai_provider,
            error_patterns: Arc::new(tokio::sync::RwLock::new(vec![])),
            strategies: Arc::new(tokio::sync::RwLock::new(vec![])),
        }
    }

    /// Analyze task result and learn from it
    pub async fn analyze_result(&self, task: &Task, result: &TaskResult) -> Result<Reflection, AgentError> {
        if !result.success {
            // Analyze error
            self.analyze_error(task, result).await?;
        } else {
            // Analyze success
            self.analyze_success(task, result).await?;
        }

        Ok(Reflection {
            task_id: task.id.clone(),
            success: result.success,
            insights: vec![],
            improvements: vec![],
        })
    }

    /// Analyze error and identify patterns
    async fn analyze_error(&self, task: &Task, result: &TaskResult) -> Result<(), AgentError> {
        let prompt = format!(
            "Analyze this task error and identify patterns:\n\
            Task: {}\n\
            Errors: {:?}\n\n\
            Identify:\n\
            - Error type\n\
            - Root cause\n\
            - Prevention strategy\n\n\
            Return a JSON response with:\n\
            - error_type (string)\n\
            - root_cause (string)\n\
            - prevention_strategy (string)",
            task.description,
            result.errors
        );

        let response = self.ai_provider
            .execute_prompt(&ProviderType::RainyApi, "gpt-4", &prompt, |_, _| {}, None::<fn(String)>)
            .await
            .map_err(|e| AgentError::TaskExecutionFailed(e.to_string()))?;

        // Parse and store error pattern
        let pattern: ErrorPattern = serde_json::from_str(&response)
            .unwrap_or_else(|_| ErrorPattern {
                id: uuid::Uuid::new_v4().to_string(),
                error_type: "unknown".to_string(),
                root_cause: "unknown".to_string(),
                prevention_strategy: "unknown".to_string(),
                count: 1,
            });

        let mut patterns = self.error_patterns.write().await;
        patterns.push(pattern);

        Ok(())
    }

    /// Analyze success and identify improvements
    async fn analyze_success(&self, task: &Task, result: &TaskResult) -> Result<(), AgentError> {
        let prompt = format!(
            "Analyze this successful task and suggest improvements:\n\
            Task: {}\n\
            Output: {}\n\n\
            Suggest:\n\
            - Performance improvements\n\
            - Quality enhancements\n\
            - Efficiency gains\n\n\
            Return a JSON response with:\n\
            - name (string)\n\
            - description (string)\n\
            - effectiveness (number 0-1)",
            task.description,
            result.output
        );

        let response = self.ai_provider
            .execute_prompt(&ProviderType::RainyApi, "gpt-4", &prompt, |_, _| {}, None::<fn(String)>)
            .await
            .map_err(|e: String| AgentError::TaskExecutionFailed(e))?;

        // Parse and store strategy
        let strategy: Strategy = serde_json::from_str(&response)
            .unwrap_or_else(|_| Strategy {
                id: uuid::Uuid::new_v4().to_string(),
                name: "unknown".to_string(),
                description: "unknown".to_string(),
                effectiveness: 0.5,
            });

        let mut strategies = self.strategies.write().await;
        strategies.push(strategy);

        Ok(())
    }

    /// Get all error patterns
    pub async fn get_error_patterns(&self) -> Vec<ErrorPattern> {
        let patterns = self.error_patterns.read().await;
        patterns.clone()
    }

    /// Get all strategies
    pub async fn get_strategies(&self) -> Vec<Strategy> {
        let strategies = self.strategies.read().await;
        strategies.clone()
    }

    /// Optimize based on learned patterns
    pub async fn optimize(&self) -> Result<OptimizationReport, AgentError> {
        let patterns = self.get_error_patterns().await;
        let strategies = self.get_strategies().await;

        Ok(OptimizationReport {
            error_patterns_count: patterns.len(),
            strategies_count: strategies.len(),
            recommendations: vec![
                "Review error patterns for common issues".to_string(),
                "Implement top strategies for improvement".to_string(),
            ],
        })
    }

    /// Clear all error patterns
    pub async fn clear_error_patterns(&self) {
        let mut patterns = self.error_patterns.write().await;
        patterns.clear();
    }

    /// Clear all strategies
    pub async fn clear_strategies(&self) {
        let mut strategies = self.strategies.write().await;
        strategies.clear();
    }
}

/// Reflection result from analyzing a task
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Reflection {
    /// Task ID that was analyzed
    pub task_id: String,
    /// Whether the task was successful
    pub success: bool,
    /// Insights gained from analysis
    pub insights: Vec<String>,
    /// Suggested improvements
    pub improvements: Vec<String>,
}

/// Error pattern identified from analysis
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ErrorPattern {
    /// Unique pattern identifier
    pub id: String,
    /// Type of error
    pub error_type: String,
    /// Root cause of the error
    pub root_cause: String,
    /// Strategy to prevent this error
    pub prevention_strategy: String,
    /// Number of times this pattern has occurred
    pub count: usize,
}

/// Strategy for improvement
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Strategy {
    /// Unique strategy identifier
    pub id: String,
    /// Strategy name
    pub name: String,
    /// Strategy description
    pub description: String,
    /// Effectiveness score (0-1)
    pub effectiveness: f64,
}

/// Optimization report
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OptimizationReport {
    /// Number of error patterns identified
    pub error_patterns_count: usize,
    /// Number of strategies generated
    pub strategies_count: usize,
    /// Recommendations for improvement
    pub recommendations: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
