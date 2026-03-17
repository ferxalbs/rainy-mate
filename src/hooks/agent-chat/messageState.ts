import type { AgentMessage } from "../../types/agent";

export function updateMessageById(
  messages: AgentMessage[],
  messageId: string,
  updater: (message: AgentMessage) => AgentMessage,
): AgentMessage[] {
  const index = messages.findIndex((message) => message.id === messageId);
  if (index === -1) return messages;

  const current = messages[index];
  const next = updater(current);
  if (next === current) return messages;

  const updated = [...messages];
  updated[index] = next;
  return updated;
}
