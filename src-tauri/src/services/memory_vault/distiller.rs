use super::types::{DistilledMemory, MemoryCategory, RawMemoryTurn};
use crate::ai::provider_types::{ChatCompletionRequest, ChatMessage};
use crate::ai::router::IntelligentRouter;
use std::sync::Arc;
use tokio::sync::RwLock;

const READONLY_TOOLS: &[&str] = &[
    "list_files",
    "list_files_detailed",
    "read_file",
    "read_many_files",
    "read_file_chunk",
    "search_files",
    "file_exists",
    "get_file_info",
    "git_status",
    "git_log",
    "git_diff",
    "git_show",
    "git_branch_list",
];

const EXTRACTION_SYSTEM_PROMPT: &str = r#"You are a memory extraction engine. Given conversation turns, extract ONLY facts worth remembering long-term.

Rules:
- Extract concise, standalone facts (1-2 sentences each)
- SKIP: file contents, code listings, search results, raw data
- FOCUS: user preferences, corrections, decisions, project knowledge, procedures
- Each fact must be self-contained (understandable without conversation context)

Output JSON array:
[{"content": "...", "category": "preference|correction|fact|procedure|observation", "importance": 0.0-1.0}]

Categories:
- preference: User style choices, likes/dislikes ("I prefer tabs over spaces")
- correction: User corrections to agent behavior ("Don't use npm, use pnpm")
- fact: Project/domain knowledge ("The API uses JWT auth with RS256")
- procedure: How to do things ("To deploy, run scripts/deploy.sh then verify on staging")
- observation: General context that doesn't fit above

If nothing is worth remembering, return []."#;

const TRIVIAL_TURN_PREFIXES: &[&str] = &[
    "hi",
    "hello",
    "hey",
    "hola",
    "buenas",
    "good morning",
    "good afternoon",
    "good evening",
    "how can i help",
    "how may i help",
    "what can i help",
    "what do you want to set up today",
];

pub struct MemoryDistiller {
    router: Arc<RwLock<IntelligentRouter>>,
}

impl MemoryDistiller {
    pub fn new(router: Arc<RwLock<IntelligentRouter>>) -> Self {
        Self { router }
    }

    pub fn is_readonly_tool(tool_name: &str) -> bool {
        READONLY_TOOLS.contains(&tool_name)
    }

    pub fn is_trivial_conversation_turn(text: &str) -> bool {
        let normalized = text.trim().to_ascii_lowercase();
        if normalized.is_empty() {
            return true;
        }

        if normalized.len() <= 12
            && normalized
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c.is_ascii_whitespace() || ",.!?'".contains(c))
        {
            return true;
        }

        TRIVIAL_TURN_PREFIXES
            .iter()
            .any(|prefix| normalized.starts_with(prefix))
    }

    pub async fn distill(&self, turns: Vec<RawMemoryTurn>) -> Result<Vec<DistilledMemory>, String> {
        if turns.is_empty() {
            return Ok(vec![]);
        }

        let meaningful_turns: Vec<RawMemoryTurn> = turns
            .into_iter()
            .filter(|turn| !Self::is_trivial_conversation_turn(&turn.content))
            .collect();

        if meaningful_turns.is_empty() {
            return Ok(vec![]);
        }

        // Fast-path: skip if all turns are read-only tool results
        let all_readonly = meaningful_turns.iter().all(|t| {
            if t.role != "tool_result" {
                return false;
            }
            if let Some(tool) = t.source.strip_prefix("tool:") {
                Self::is_readonly_tool(tool)
            } else {
                false
            }
        });
        if all_readonly {
            return Ok(vec![]);
        }

        // Build extraction prompt from turns
        let mut input_block = String::with_capacity(4096);
        for turn in &meaningful_turns {
            input_block.push_str(&format!("[{}] ({}): {}\n\n", turn.role, turn.source, turn.content));
        }

        let request = ChatCompletionRequest {
            messages: vec![
                ChatMessage::system(EXTRACTION_SYSTEM_PROMPT),
                ChatMessage::user(input_block),
            ],
            model: "default".to_string(),
            temperature: Some(0.1),
            max_tokens: Some(1024),
            json_mode: true,
            ..Default::default()
        };

        let response = self
            .router
            .read()
            .await
            .complete(request)
            .await
            .map_err(|e| format!("Distillation LLM call failed: {}", e))?;

        let response_text = response.content.unwrap_or_default();
        let mut memories = parse_distilled_json(&response_text);

        // If parse failed, fail closed.
        if memories.is_empty() && !response_text.trim().is_empty() {
            let trimmed = response_text.trim();
            if trimmed == "[]" {
                return Ok(vec![]);
            }
            tracing::warn!("Memory distillation returned unparsable payload; skipping persistence");
            return Ok(vec![]);
        }

        // Apply importance floors
        for mem in &mut memories {
            match mem.category {
                MemoryCategory::Preference => mem.importance = mem.importance.max(0.7),
                MemoryCategory::Correction => mem.importance = mem.importance.max(0.8),
                _ => {}
            }
        }

        Ok(memories)
    }
}

fn parse_distilled_json(text: &str) -> Vec<DistilledMemory> {
    let trimmed = text.trim();

    // Try direct parse
    if let Ok(items) = serde_json::from_str::<Vec<RawDistilled>>(trimmed) {
        return items.into_iter().map(|r| r.into()).collect();
    }

    // Try to find JSON array in the response (LLM might wrap in markdown)
    if let Some(start) = trimmed.find('[') {
        if let Some(end) = trimmed.rfind(']') {
            let slice = &trimmed[start..=end];
            if let Ok(items) = serde_json::from_str::<Vec<RawDistilled>>(slice) {
                return items.into_iter().map(|r| r.into()).collect();
            }
        }
    }

    vec![]
}

#[derive(serde::Deserialize)]
struct RawDistilled {
    content: String,
    category: String,
    importance: f32,
}

impl From<RawDistilled> for DistilledMemory {
    fn from(raw: RawDistilled) -> Self {
        Self {
            content: raw.content,
            category: MemoryCategory::from_str_loose(&raw.category),
            importance: raw.importance.clamp(0.0, 1.0),
        }
    }
}
