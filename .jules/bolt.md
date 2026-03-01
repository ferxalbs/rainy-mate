
## 2024-05-15 - [React.memo in chat UI]
**Learning:** `MessageBubble` and `MarkdownRenderer` in `src/components/agent-chat` and `src/components/shared` are missing `React.memo()`. This can cause significant performance degradation during message streaming as the parent component (`AgentChatPanel`) re-renders, causing all historical messages to re-render.
**Action:** Wrap these components with `React.memo()` to prevent unnecessary re-renders of historical chat messages during active streaming.
