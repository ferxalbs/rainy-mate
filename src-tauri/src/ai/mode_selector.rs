// Mode Selector
#![allow(dead_code)]
// Intelligent routing between Rainy API and Cowork modes

use crate::ai::unified_model_registry::{ModelContext, ProviderSource};

/// Processing mode for AI requests
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ProcessingMode {
    /// Fast, direct AI access via Rainy API
    FastChat,
    /// Streaming responses for real-time interaction
    Streaming,
    /// High-level complex operations via Cowork
    DeepProcessing,
}

/// Use case for AI request
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UseCase {
    /// Quick question or simple query
    QuickQuestion,
    /// Streaming response needed
    StreamingResponse,
    /// File analysis or editing
    FileOperation,
    /// Batch processing of multiple files
    BatchProcessing,
    /// Code review or generation
    CodeReview,
    /// Research with web search
    WebResearch,
}

/// Task complexity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskComplexity {
    Low = 1,
    Medium = 2,
    High = 3,
}

/// Mode Selection Logic
// @TODO: Full implementation pending

/// Mode selector for intelligent routing
pub struct ModeSelector;

impl ModeSelector {
    /// Select processing mode based on API key, use case, and complexity
    ///
    /// Priority:
    /// 1. Rainy API mode for fast, direct access (preferred)
    /// 2. Cowork mode only for complex, multi-step operations
    pub fn select_mode(
        _api_key: &str,
        use_case: UseCase,
        _complexity: TaskComplexity,
    ) -> ProcessingMode {
        Self::select_for_rainy_api_key(use_case)
    }

    /// Select mode for regular Rainy API key
    fn select_for_rainy_api_key(use_case: UseCase) -> ProcessingMode {
        match use_case {
            UseCase::StreamingResponse => ProcessingMode::Streaming,
            _ => ProcessingMode::FastChat,
        }
    }

    /// Determine if Cowork mode should be used
    pub fn should_use_cowork(
        api_key: &str,
        use_case: UseCase,
        complexity: TaskComplexity,
    ) -> bool {
        Self::select_mode(api_key, use_case, complexity) == ProcessingMode::DeepProcessing
    }

    /// Get recommended provider source for given context
    pub fn recommended_provider(_api_key: &str, context: ModelContext) -> ProviderSource {
        match context {
            ModelContext::Chat => ProviderSource::RainyApi,
            ModelContext::Processing => ProviderSource::RainyApi,
        }
    }

    /// Estimate task complexity from description
    pub fn estimate_complexity(description: &str) -> TaskComplexity {
        let desc_lower = description.to_lowercase();
        let mut score = 0;

        // Keywords indicating complexity
        let high_complexity_keywords = [
            "batch",
            "multiple files",
            "refactor",
            "rewrite",
            "analyze entire",
            "comprehensive",
            "complex",
            "advanced",
            "architecture",
            "system",
        ];

        let medium_complexity_keywords = [
            "edit", "modify", "update", "fix", "improve", "optimize", "review", "check",
        ];

        for keyword in &high_complexity_keywords {
            if desc_lower.contains(keyword) {
                score += 2;
            }
        }

        for keyword in &medium_complexity_keywords {
            if desc_lower.contains(keyword) {
                score += 1;
            }
        }

        // Check for file count
        if desc_lower.contains("file") {
            let file_count = desc_lower.matches("file").count();
            if file_count > 3 {
                score += 2;
            } else if file_count > 1 {
                score += 1;
            }
        }

        match score {
            0..=1 => TaskComplexity::Low,
            2..=3 => TaskComplexity::Medium,
            _ => TaskComplexity::High,
        }
    }

    /// Detect use case from request
    pub fn detect_use_case(description: &str, requires_streaming: bool) -> UseCase {
        if requires_streaming {
            return UseCase::StreamingResponse;
        }

        let desc_lower = description.to_lowercase();

        if desc_lower.contains("batch") || desc_lower.contains("multiple") {
            return UseCase::BatchProcessing;
        }

        if desc_lower.contains("code") || desc_lower.contains("review") {
            return UseCase::CodeReview;
        }

        if desc_lower.contains("research") || desc_lower.contains("search") {
            return UseCase::WebResearch;
        }

        if desc_lower.contains("file") || desc_lower.contains("edit") {
            return UseCase::FileOperation;
        }

        UseCase::QuickQuestion
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_mode_rainy_api_key() {
        let mode = ModeSelector::select_mode(
            "ra-20250125143052Ab3Cd9Ef2Gh5Ik8Lm4Np7Qr",
            UseCase::QuickQuestion,
            TaskComplexity::Low,
        );
        assert_eq!(mode, ProcessingMode::FastChat);
    }

    #[test]
    fn test_select_mode_current_rainy_key_simple() {
        let mode = ModeSelector::select_mode(
            "ra-20250125143052Ab3Cd9Ef2Gh5Ik8Lm4Np7Qr",
            UseCase::QuickQuestion,
            TaskComplexity::Low,
        );
        assert_eq!(mode, ProcessingMode::FastChat);
    }

    #[test]
    fn test_select_mode_current_rainy_key_complex_request() {
        let mode = ModeSelector::select_mode(
            "ra-20250125143052Ab3Cd9Ef2Gh5Ik8Lm4Np7Qr",
            UseCase::BatchProcessing,
            TaskComplexity::High,
        );
        assert_eq!(mode, ProcessingMode::FastChat);
    }

    #[test]
    fn test_select_mode_streaming() {
        let mode = ModeSelector::select_mode(
            "ra-20250125143052Ab3Cd9Ef2Gh5Ik8Lm4Np7Qr",
            UseCase::StreamingResponse,
            TaskComplexity::Low,
        );
        assert_eq!(mode, ProcessingMode::Streaming);
    }

    #[test]
    fn test_estimate_complexity_low() {
        let complexity = ModeSelector::estimate_complexity("What is the weather?");
        assert_eq!(complexity, TaskComplexity::Low);
    }

    #[test]
    fn test_estimate_complexity_medium() {
        let complexity = ModeSelector::estimate_complexity("Edit the file to fix the bug");
        assert_eq!(complexity, TaskComplexity::Medium);
    }

    #[test]
    fn test_estimate_complexity_high() {
        let complexity =
            ModeSelector::estimate_complexity("Batch refactor multiple files in the system");
        assert_eq!(complexity, TaskComplexity::High);
    }
}
