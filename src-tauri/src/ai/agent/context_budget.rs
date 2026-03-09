use crate::ai::agent::runtime::AgentMessage;

pub struct ContextBudget;

impl ContextBudget {
    /// Applies a lightweight context guard over the session messages.
    /// Returns the active slice of messages that fit within the context window
    /// and a boolean indicating whether the session overflowed and requires compaction.
    pub fn apply_context_guard(
        messages: &[AgentMessage],
        window_tokens: usize,
    ) -> (Vec<AgentMessage>, bool) {
        // Approximate token counting (e.g. 4 chars per token)
        let max_chars = window_tokens * 4;
        let mut current_chars = 0;
        let mut keepers = Vec::new();
        let mut overflowed = false;

        // Iterate backwards to keep the most recent messages
        for msg in messages.iter().rev() {
            let msg_len = msg.content.as_text().len();
            if current_chars + msg_len <= max_chars {
                current_chars += msg_len;
                keepers.push(msg.clone());
            } else {
                overflowed = true;
                break;
            }
        }

        // Restore chronological order
        keepers.reverse();

        (keepers, overflowed)
    }

    /// Provides a recovery pipeline if context budget is severely breached.
    pub fn recover_from_overflow(messages: &[AgentMessage]) -> Vec<AgentMessage> {
        // Fallback pipeline: keep first message (e.g. system prompt context if any)
        // and a slice of the most recent tail.
        if messages.len() <= 10 {
            return messages.to_vec();
        }

        let mut recovered = Vec::new();
        recovered.push(messages[0].clone());

        let tail = &messages[messages.len() - 9..];
        recovered.extend_from_slice(tail);

        recovered
    }
}
