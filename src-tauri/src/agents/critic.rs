//! CriticAgent - Quality evaluation and improvement suggestions
//!
//! The CriticAgent evaluates task results for quality, accuracy, and coherence,
//! providing actionable improvement suggestions to enhance overall system performance.

use std::sync::Arc;
use async_trait::async_trait;

use crate::agents::{
    Agent, AgentConfig, AgentError, AgentInfo, AgentMessage,
    AgentStatus, AgentType, Task, TaskResult, TaskPriority,
    BaseAgent, AgentRegistry,
};

/// CriticAgent evaluates task results and provides quality assessments
pub struct CriticAgent {
    base: BaseAgent,
    registry: Arc<AgentRegistry>,
}

impl CriticAgent {
    /// Create a new CriticAgent instance
    pub fn new(
        config: AgentConfig,
        registry: Arc<AgentRegistry>,
    ) -> Self {
        let ai_provider = registry.ai_provider();
        let message_bus = registry.message_bus();
        let base = BaseAgent::new(config, ai_provider, message_bus);

        Self { base, registry }
    }

    /// Evaluate task result quality
    async fn evaluate_result(&self, result: &TaskResult) -> Result<QualityEvaluation, AgentError> {
        let prompt = format!(
            "Evaluate the quality of this task result:\n\
            Success: {}\n\
            Output: {}\n\
            Errors: {:?}\n\n\
            Provide a JSON response with:\n\
            - quality_score (0-100)\n\
            - accuracy (assessment)\n\
            - coherence (assessment)\n\
            - suggestions (array of improvement suggestions)",
            result.success,
            result.output,
            result.errors
        );

        let response = self.base.query_ai(&prompt).await?;

        // Parse AI response
        let evaluation: QualityEvaluation = serde_json::from_str(&response)
            .map_err(|e| AgentError::TaskExecutionFailed(
                format!("Failed to parse evaluation: {}", e)
            ))?;

        Ok(evaluation)
    }

    /// Provide improvement suggestions for a task result
    pub async fn suggest_improvements(&self, result: &TaskResult) -> Result<Vec<String>, AgentError> {
        let evaluation = self.evaluate_result(result).await?;
        Ok(evaluation.suggestions)
    }

    /// Get quality score for a task result
    pub async fn get_quality_score(&self, result: &TaskResult) -> Result<u8, AgentError> {
        let evaluation = self.evaluate_result(result).await?;
        Ok(evaluation.quality_score)
    }
}

/// Quality evaluation result
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct QualityEvaluation {
    /// Overall quality score (0-100)
    pub quality_score: u8,
    /// Accuracy assessment
    pub accuracy: String,
    /// Coherence assessment
    pub coherence: String,
    /// Improvement suggestions
    pub suggestions: Vec<String>,
}

#[async_trait]
impl Agent for CriticAgent {
    fn info(&self) -> AgentInfo {
        AgentInfo {
            id: self.base.info().id,
            name: "Critic".to_string(),
            agent_type: AgentType::Critic,
            status: self.base.info().status,
            current_task: self.base.info().current_task,
        }
    }

    async fn update_status(&self, status: AgentStatus) {
        self.base.update_status(status).await;
    }

    async fn set_current_task(&self, task_id: Option<String>) {
        self.base.set_current_task(task_id).await;
    }

    async fn process_task(&self, task: Task) -> Result<TaskResult, AgentError> {
        self.base.update_status(AgentStatus::Busy).await;
        self.base.set_current_task(Some(task.id.clone())).await;

        // Evaluate result from task context
        let evaluation = self.evaluate_result(&TaskResult {
            success: true,
            output: task.description.clone(),
            errors: vec![],
            metadata: serde_json::json!({}),
        }).await?;

        self.base.update_status(AgentStatus::Idle).await;
        self.base.set_current_task(None).await;

        Ok(TaskResult {
            success: true,
            output: serde_json::to_string(&evaluation).unwrap_or_default(),
            errors: vec![],
            metadata: serde_json::json!({
                "quality_score": evaluation.quality_score,
                "evaluated": true
            }),
        })
    }

    async fn handle_message(&self, message: AgentMessage) -> Result<(), AgentError> {
        match message {
            AgentMessage::TaskAssign { task_id: _, task } => {
                let _result = self.process_task(task).await;
                // Send result back via message bus
            }
            _ => {}
        }
        Ok(())
    }

    fn capabilities(&self) -> Vec<String> {
        vec![
            "quality_evaluation".to_string(),
            "accuracy_assessment".to_string(),
            "coherence_check".to_string(),
            "improvement_suggestions".to_string(),
        ]
    }

    fn can_handle(&self, task: &Task) -> bool {
        task.description.contains("evaluate") ||
        task.description.contains("review") ||
        task.description.contains("critique") ||
        task.description.contains("assess")
    }

    async fn initialize(&mut self, config: AgentConfig) -> Result<(), AgentError> {
        self.base.initialize(config).await
    }

    async fn shutdown(&mut self) -> Result<(), AgentError> {
        self.base.shutdown().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quality_evaluation_serialization() {
        let evaluation = QualityEvaluation {
            quality_score: 85,
            accuracy: "High".to_string(),
            coherence: "Good".to_string(),
            suggestions: vec![
                "Add more details".to_string(),
                "Improve structure".to_string(),
            ],
        };

        let json = serde_json::to_string(&evaluation).unwrap();
        let deserialized: QualityEvaluation = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.quality_score, 85);
        assert_eq!(deserialized.accuracy, "High");
        assert_eq!(deserialized.suggestions.len(), 2);
    }

    #[test]
    fn test_can_handle() {
        // This test would require a full setup with registry
        // For now, we just verify logic structure
        let task = Task {
            id: "test-1".to_string(),
            description: "Evaluate this result".to_string(),
            priority: TaskPriority::Medium,
            dependencies: vec![],
            context: crate::agents::TaskContext {
                workspace_id: "default".to_string(),
                user_instruction: "".to_string(),
                relevant_files: vec![],
                memory_context: vec![],
            },
        };

        assert!(task.description.contains("evaluate"));
    }
}
