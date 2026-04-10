import type { AgentMessage } from "../../../types/agent";

export type TimelineRowKind = "user-message" | "assistant-message" | "work-group" | "working";

export interface TimelineWorkEntry {
  id: string;
  tone: "thinking" | "tool" | "info" | "error";
  detail?: string;
  command?: string;
  rawCommand?: string;
  status: "running" | "completed" | "failed";
  timestamp: Date;
}

export interface MessagesTimelineRow {
  id: string;
  kind: TimelineRowKind;
  messageId?: string;
  message?: AgentMessage;
  groupedEntries?: TimelineWorkEntry[];
  createdAt?: string;
  durationStart?: string;
  showCompletionDivider?: boolean;
}

export function deriveMessagesTimelineRows(messages: AgentMessage[]): MessagesTimelineRow[] {
  const rows: MessagesTimelineRow[] = [];

  for (const message of messages) {
    if (message.type === "user") {
      rows.push({
        id: `row-${message.id}`,
        kind: "user-message",
        messageId: message.id,
        message,
        createdAt: message.timestamp.toISOString(),
      });
      continue;
    }

    if (message.type === "agent") {
      const workEntries: TimelineWorkEntry[] = [];

      if (message.thought) {
          workEntries.push({
              id: `thought-${message.id}`,
              tone: "thinking",
              detail: message.thought,
              status: "completed",
              timestamp: message.timestamp,
          });
      }

      if (message.trace && message.trace.length > 0) {
        for (const trace of message.trace) {
          if (trace.phase === "tool" || trace.phase === "error") {
             workEntries.push({
                id: `trace-${trace.id}`,
                tone: trace.phase === "error" ? "error" : "tool",
                detail: trace.label || trace.toolName,
                command: trace.toolName,
                rawCommand: trace.preview,
                status: "completed",
                timestamp: trace.timestamp,
             });
          }
        }
      }

      if (workEntries.length > 0) {
        rows.push({
          id: `work-${message.id}`,
          kind: "work-group",
          messageId: message.id,
          groupedEntries: workEntries,
          createdAt: message.timestamp.toISOString(),
        });
      }

      // If the agent has output content, or is generating content / thinking and has text
      if (message.content || message.runState === "completed" || message.runState === "failed") {
          rows.push({
             id: `response-${message.id}`,
             kind: "assistant-message",
             messageId: message.id,
             message,
             createdAt: message.timestamp.toISOString(),
          });
      }

      if (message.runState === "running" && !message.content) {
         rows.push({
            id: `working-${message.id}`,
            kind: "working",
            messageId: message.id,
            createdAt: message.timestamp.toISOString(),
         });
      }
    }
  }

  return rows;
}

export function estimateTimelineRowHeight(row: MessagesTimelineRow, _widthPx: number | null): number {
  if (row.kind === "user-message") {
    // base + text approximation
    if (row.message?.content) {
         const lines = row.message.content.split("\n").length;
         return 60 + (lines * 24);
    }
    return 60;
  }
  if (row.kind === "work-group") {
    return 50 + (row.groupedEntries?.length || 0) * 28;
  }
  if (row.kind === "assistant-message") {
     if (row.message?.content) {
         return 40 + Math.ceil(row.message.content.length / 80) * 24;
     }
     return 40;
  }
  if (row.kind === "working") {
     return 40;
  }
  return 80;
}
