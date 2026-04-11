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

function buildLivePhaseEntry(message: AgentMessage): TimelineWorkEntry | null {
  if (message.runState !== "running") {
    return null;
  }

  const detail = message.statusText?.trim();
  const timestamp = message.timestamp;
  switch (message.runPhase) {
    case "starting":
      return {
        id: `phase-${message.id}-starting`,
        tone: "info",
        detail: detail || "Preparing the runtime for this turn.",
        status: "running",
        timestamp,
      };
    case "planning":
      return {
        id: `phase-${message.id}-planning`,
        tone: "thinking",
        detail: detail || "Planning the next step.",
        status: "running",
        timestamp,
      };
    case "awaiting_approval":
      return {
        id: `phase-${message.id}-approval`,
        tone: "info",
        detail: detail || "Waiting for approval before continuing.",
        status: "running",
        timestamp,
      };
    case "tool_waiting":
      return {
        id: `phase-${message.id}-tool-waiting`,
        tone: "tool",
        detail: detail || "Preparing tool work from the streamed plan.",
        command: message.activeToolName,
        status: "running",
        timestamp,
      };
    case "tool_running":
      return {
        id: `phase-${message.id}-tool`,
        tone: "tool",
        detail: detail || "Running tool work.",
        command: message.activeToolName,
        status: "running",
        timestamp,
      };
    case "responding":
      return {
        id: `phase-${message.id}-responding`,
        tone: "thinking",
        detail: detail || "Continuing the response after tool execution.",
        status: "running",
        timestamp,
      };
    case "streaming":
      return {
        id: `phase-${message.id}-streaming`,
        tone: "thinking",
        detail: detail || "Streaming assistant output.",
        status: "running",
        timestamp,
      };
    default:
      return null;
  }
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

  const livePhaseEntry = buildLivePhaseEntry(message);
  if (livePhaseEntry && !entries.some((entry) => entry.id === livePhaseEntry.id)) {
    entries.push(livePhaseEntry);
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
    const shouldSurfaceAssistantBody =
      Boolean(message.content) ||
      Boolean(message.artifacts?.length) ||
      message.runState === "completed" ||
      message.runState === "failed" ||
      message.runState === "cancelled";

    if (shouldSurfaceAssistantBody && message.runState === "running") {
      rows.push({
        id: `response-${message.id}`,
        kind: "assistant-message",
        messageId: message.id,
        message,
        createdAt: message.timestamp.toISOString(),
      });
    }

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

    if (
      shouldSurfaceAssistantBody &&
      message.runState !== "running"
    ) {
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
    return estimateTextHeight(content || "Awaiting response", charsPerLine, 24, 82 + artifactCount * 34);
  }

  return 54;
}
