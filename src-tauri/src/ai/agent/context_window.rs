// Context Window Manager — Sliding window to prevent unbounded history growth.
// Enforces max_tokens limit by evicting oldest non-system messages when exceeded.

use crate::ai::agent::runtime::AgentMessage;

/// Estimated average tokens per character for English text.
/// Conservative: 1 token ≈ 4 chars for most models.
const CHARS_PER_TOKEN: usize = 4;

/// Default maximum context window in tokens if not configured via spec.
const DEFAULT_MAX_TOKENS: usize = 120_000;

/// Manages the agent's context window to keep history within token limits.
pub struct ContextWindow {
    max_tokens: usize,
}

impl ContextWindow {
    /// Create a new ContextWindow with the given max_tokens limit.
    pub fn new(max_tokens: usize) -> Self {
        Self {
            max_tokens: if max_tokens == 0 {
                DEFAULT_MAX_TOKENS
            } else {
                max_tokens
            },
        }
    }

    /// Estimate the token count for a single message.
    fn estimate_tokens(msg: &AgentMessage) -> usize {
        let text_len = msg.content.as_text().len();
        // tool_calls add overhead: ~50 tokens per call for function name + args
        let tool_overhead = msg
            .tool_calls
            .as_ref()
            .map(|calls| calls.len() * 50)
            .unwrap_or(0);
        (text_len / CHARS_PER_TOKEN) + tool_overhead + 4 // +4 for role/metadata overhead
    }

    /// Estimate the total token count for a message list.
    pub fn estimate_total_tokens(messages: &[AgentMessage]) -> usize {
        messages.iter().map(Self::estimate_tokens).sum()
    }

    /// Trim the history to fit within the max_tokens limit.
    /// Preserves system messages (always kept) and the latest messages.
    /// Removes oldest non-system messages first.
    ///
    /// Returns the trimmed messages vector.
    pub fn trim_history(&self, messages: Vec<AgentMessage>) -> Vec<AgentMessage> {
        let total = Self::estimate_total_tokens(&messages);

        if total <= self.max_tokens {
            return messages;
        }

        // Separate system messages from the rest
        let mut system_msgs: Vec<AgentMessage> = Vec::new();
        let mut non_system: Vec<AgentMessage> = Vec::new();

        for msg in messages {
            if msg.role == "system" {
                system_msgs.push(msg);
            } else {
                non_system.push(msg);
            }
        }

        // System messages token budget
        let system_tokens: usize = system_msgs.iter().map(Self::estimate_tokens).sum();
        let available_tokens = self.max_tokens.saturating_sub(system_tokens);

        // Keep as many recent non-system messages as fit in the budget
        let mut kept: Vec<AgentMessage> = Vec::new();
        let mut used_tokens: usize = 0;

        // Iterate from newest to oldest, keeping messages that fit
        for msg in non_system.into_iter().rev() {
            let msg_tokens = Self::estimate_tokens(&msg);
            if used_tokens + msg_tokens <= available_tokens {
                kept.push(msg);
                used_tokens += msg_tokens;
            } else {
                break; // Stop keeping once we can't fit more
            }
        }

        // Reverse to restore chronological order
        kept.reverse();

        // Combine: system messages first, then trimmed history
        let mut result = system_msgs;
        result.extend(kept);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::agent::runtime::AgentContent;

    fn make_msg(role: &str, text: &str) -> AgentMessage {
        AgentMessage {
            role: role.to_string(),
            content: AgentContent::text(text),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    #[test]
    fn test_no_trim_under_limit() {
        let cw = ContextWindow::new(10_000);
        let msgs = vec![
            make_msg("system", "You are an agent."),
            make_msg("user", "Hello"),
            make_msg("assistant", "Hi there!"),
        ];
        let trimmed = cw.trim_history(msgs.clone());
        assert_eq!(trimmed.len(), 3);
    }

    #[test]
    fn test_trim_evicts_oldest() {
        // Very small window — only ~100 tokens
        let cw = ContextWindow::new(100);
        let msgs = vec![
            make_msg("system", "System prompt"),
            make_msg("user", &"A".repeat(200)),      // ~50 tokens
            make_msg("assistant", &"B".repeat(200)), // ~50 tokens
            make_msg("user", &"C".repeat(200)),      // ~50 tokens — this should be kept
            make_msg("assistant", &"D".repeat(200)), // ~50 tokens — this should be kept
        ];
        let trimmed = cw.trim_history(msgs);
        // System is always kept, then as many recent as fit
        assert!(trimmed.len() < 5);
        assert_eq!(trimmed[0].role, "system");
        // Last messages should be the most recent
        assert_eq!(trimmed.last().unwrap().content.as_text(), "D".repeat(200));
    }

    #[test]
    fn test_system_always_preserved() {
        let cw = ContextWindow::new(50); // Very tight
        let msgs = vec![
            make_msg("system", "You are a helpful agent."),
            make_msg("user", &"X".repeat(1000)),
        ];
        let trimmed = cw.trim_history(msgs);
        assert!(trimmed.iter().any(|m| m.role == "system"));
    }
}
