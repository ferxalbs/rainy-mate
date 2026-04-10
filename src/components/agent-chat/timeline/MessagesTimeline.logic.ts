import type { AgentMessage, AgentTraceEntry } from "../../../types/agent";

export type TimelineRowKind = "user-message" | "assistant-message" | "work-group" | "working";

export interface TimelineWorkEntry {
  id: string;
  tone: "thinking" | "tool" | "info" | "error";
  detail: string;
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
}

function buildWorkEntryFromTrace(trace: AgentTraceEntry): TimelineWorkEntry | null {
  if (trace.phase === "tool") {
    return {
      id: `trace-${trace.id}`,
      tone: "tool",
      detail: trace.label || trace.toolName || "Tool execution",
      command: trace.toolName,
      rawCommand: trace.preview,
      status: "completed",
      timestamp: trace.timestamp,
    };
  }

  if (trace.phase === "error") {
    return {
      id: `trace-${trace.id}`,
      tone: "error",
      detail: trace.label || "Runtime error",
      rawCommand: trace.preview,
      status: "failed",
      timestamp: trace.timestamp,
    };
  }

  if (trace.phase === "approval" || trace.phase === "retry" || trace.phase === "act") {
    return {
      id: `trace-${trace.id}`,
      tone: "info",
      detail: trace.label,
      rawCommand: trace.preview,
      status: "completed",
      timestamp: trace.timestamp,
    };
  }

  return null;
}

function deriveWorkEntries(message: AgentMessage): TimelineWorkEntry[] {
  const entries: TimelineWorkEntry[] = [];

  if (message.thought?.trim()) {
    entries.push({
      id: `thought-${message.id}`,
      tone: "thinking",
      detail: message.thought.trim(),
      status: message.runState === "failed" ? "failed" : "completed",
      timestamp: message.timestamp,
    });
  }

  for (const trace of message.trace ?? []) {
    const entry = buildWorkEntryFromTrace(trace);
    if (entry) {
      entries.push(entry);
    }
  }

  if (message.runState === "running" && message.activeToolName) {
    const runningEntryId = `running-${message.id}-${message.activeToolName}`;
    const alreadyPresent = entries.some((entry) => entry.id === runningEntryId);
    if (!alreadyPresent) {
      entries.push({
        id: runningEntryId,
        tone: "tool",
        detail: `Executing ${message.activeToolName}`,
        command: message.activeToolName,
        status: "running",
        timestamp: message.timestamp,
      });
    }
  }

  return entries;
}

export function deriveMessagesTimelineRows(messages: AgentMessage[]): MessagesTimelineRow[] {
  const rows: MessagesTimelineRow[] = [];

  for (const message of messages) {
    if (message.type === "system") {
      continue;
    }

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

    const workEntries = deriveWorkEntries(message);
    if (workEntries.length > 0) {
      rows.push({
        id: `work-${message.id}`,
        kind: "work-group",
        messageId: message.id,
        message,
        groupedEntries: workEntries,
        createdAt: message.timestamp.toISOString(),
      });
    }

    if (message.content || message.artifacts?.length || message.runState === "completed" || message.runState === "failed" || message.runState === "cancelled") {
      rows.push({
        id: `response-${message.id}`,
        kind: "assistant-message",
        messageId: message.id,
        message,
        createdAt: message.timestamp.toISOString(),
      });
      continue;
    }

    if (message.runState === "running") {
      rows.push({
        id: `working-${message.id}`,
        kind: "working",
        messageId: message.id,
        message,
        createdAt: message.timestamp.toISOString(),
      });
    }
  }

  return rows;
}

function estimateTextHeight(text: string, charsPerLine: number, lineHeight: number, base: number): number {
  const lines = Math.max(1, Math.ceil(text.length / charsPerLine) + text.split("\n").length - 1);
  return base + lines * lineHeight;
}

export function estimateTimelineRowHeight(row: MessagesTimelineRow, widthPx: number | null): number {
  const narrow = widthPx !== null && widthPx < 720;
  const charsPerLine = narrow ? 36 : 54;

  if (row.kind === "user-message") {
    const attachmentCount = row.message?.attachments?.length ?? 0;
    const content = row.message?.content ?? "";
    return estimateTextHeight(content, charsPerLine, 24, 76 + attachmentCount * 86);
  }

  if (row.kind === "work-group") {
    const entries = row.groupedEntries ?? [];
    return 64 + entries.reduce((total, entry) => total + estimateTextHeight(entry.detail, charsPerLine, 18, 22), 0);
  }

  if (row.kind === "assistant-message") {
    const content = row.message?.content ?? "";
    const artifactCount = row.message?.artifacts?.length ?? 0;
    return estimateTextHeight(content || "Awaiting response", charsPerLine, 24, 68 + artifactCount * 34);
  }

  return 54;
}
