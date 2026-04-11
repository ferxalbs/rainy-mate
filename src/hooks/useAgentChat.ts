import { useState, useCallback, useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { useStreaming } from "./useStreaming";
import { useTauriTask } from "./useTauriTask";
import type {
  AgentTraceEntry,
  AgentMessage,
  ChatAttachment,
  ExternalAgentSession,
  SpecialistRunState,
  TaskPlan,
} from "../types/agent";
import * as tauri from "../services/tauri";
import { runAgentWorkflow } from "../services/tauri";
import {
  resolveNeuralState,
  getToolDisplayName,
} from "../components/agent-chat/neural-config";
import { updateMessageById } from "./agent-chat/messageState";
import {
  appendUniqueArtifact,
  artifactsFromToolResult,
} from "../lib/chat-artifacts";
import type { ActiveChatRunBinding } from "../services/tauri";

type RuntimeAgentEventData = {
  [key: string]: unknown;
  summary?: string;
  steps?: string[];
  verificationRequired?: boolean;
  mode?: string;
  delegationPolicy?: string;
  maxDepth?: number;
  maxThreads?: number;
  maxParallelSubagents?: number;
  internalCoordinationLanguage?: string;
  finalResponseLanguageMode?: string;
  agentId?: string;
  role?: string;
  status?: SpecialistRunState["status"];
  detail?: string;
  activeTool?: string;
  dependsOn?: string[];
  startedAtMs?: number;
  finishedAtMs?: number;
  toolCount?: number;
  writeLikeUsed?: boolean;
  parentAgentId?: string;
  depth?: number;
  branchId?: string;
  spawnReason?: string;
  error?: string;
  responsePreview?: string;
  function?: {
    name?: string;
    arguments?: string;
  };
  state?: string;
  name?: string;
  arguments?: string;
  index?: number;
  model?: string;
  prompt_tokens?: number;
  completion_tokens?: number;
  total_tokens?: number;
  id?: string;
  result?: string;
  text?: string;
  airlock_level?: number;
  history_source?: string;
  retrieval_mode?: string;
  embedding_profile?: string;
  applied?: boolean;
  trigger_tokens?: number;
};

type RuntimeAgentEvent =
  | {
      runId?: string;
      timestampMs?: number;
      type:
        | "supervisor_plan_created"
        | "specialist_spawned"
        | "specialist_status_changed"
        | "specialist_completed"
        | "specialist_failed"
        | "supervisor_summary"
        | "status"
        | "thought"
        | "stream_chunk"
        | "stream_tool_call"
        | "usage"
        | "tool_call"
        | "tool_result";
      data?: RuntimeAgentEventData;
    }
  | {
      runId?: string;
      timestampMs?: number;
      type: string;
      data?: RuntimeAgentEventData;
    };

type RuntimeToolCallIndex = Map<
  string,
  { name: string; arguments?: string }
>;

type ActiveChatSessionBinding = {
  workspaceId: string;
  workspacePath?: string;
  workspaceName?: string;
  chatId: string;
  runId: string;
  source: "local" | "remote" | "native_modal" | string;
  connectorId?: string | null;
  elapsedSecs?: number;
};

const EXTERNAL_SESSION_POLL_INTERVAL_MS = 2000;
type AgentRunPhase = NonNullable<AgentMessage["runPhase"]>;

function appendStreamText(current: string, next: string | undefined): string {
  if (typeof next !== "string" || next.trim().length === 0) {
    return current;
  }
  return `${current}${current ? "\n" : ""}${next}`;
}

function deriveStatusRunPhase(statusText: string): AgentRunPhase | null {
  const lower = statusText.toLowerCase();
  if (
    lower.includes("awaiting airlock approval") ||
    (lower.includes("approval") && lower.includes("mcp"))
  ) {
    return "awaiting_approval";
  }
  if (lower.includes("executing tool:")) {
    return "tool_running";
  }
  if (lower.includes("streaming plan and tool intent")) {
    return "tool_waiting";
  }
  if (lower.includes("analyzing and planning tools")) {
    return "planning";
  }
  if (lower.includes("thinking")) {
    return "planning";
  }
  if (lower.includes("execution cancelled") || lower.includes("cancelled")) {
    return null;
  }
  return null;
}

function clearTransientRunDecorations(): Partial<AgentMessage> {
  return {
    activeToolName: undefined,
    statusText: undefined,
  };
}

function mergeDefinedFields<T>(current: T, next: Partial<T>): T {
  const merged = { ...(current as Record<string, unknown>) };
  for (const [key, value] of Object.entries(next as Record<string, unknown>)) {
    if (value !== undefined) {
      merged[key] = value;
    }
  }
  return merged as T;
}

function upsertSpecialistState(
  specialists: SpecialistRunState[] | undefined,
  next: Partial<SpecialistRunState> & {
    agentId: string;
    role: SpecialistRunState["role"];
    status: SpecialistRunState["status"];
  },
): SpecialistRunState[] {
  const current = specialists ? [...specialists] : [];
  const idx = current.findIndex((item) => item.agentId === next.agentId);
  const merged = mergeDefinedFields(
    idx >= 0 ? current[idx] : ({} as SpecialistRunState),
    next,
  ) as SpecialistRunState;
  if (idx >= 0) {
    current[idx] = merged;
  } else {
    current.push(merged);
  }
  return current;
}

function isCancellationError(error: unknown): boolean {
  const message =
    error instanceof Error ? error.message : String(error ?? "");
  const normalized = message.toLowerCase();
  return normalized.includes("execution cancelled");
}

function summarizeExternalSessionResult(raw: string): string | null {
  try {
    const parsed = JSON.parse(raw) as {
      runtimeKind?: string;
      status?: string;
      touchedPaths?: string[];
      artifacts?: Array<{ path?: string }>;
      error?: string | null;
    };
    if (
      parsed.runtimeKind !== "codex" &&
      parsed.runtimeKind !== "claude"
    ) {
      return null;
    }
    const runtimeLabel = parsed.runtimeKind === "codex" ? "Codex" : "Claude Code";
    const touchedCount = Array.isArray(parsed.touchedPaths)
      ? parsed.touchedPaths.length
      : 0;
    const artifactCount = Array.isArray(parsed.artifacts) ? parsed.artifacts.length : 0;
    const statusLabel = parsed.status || "pending";
    if (parsed.error) {
      return `${runtimeLabel} session ${statusLabel}: ${parsed.error}`;
    }
    return `${runtimeLabel} session ${statusLabel} · ${touchedCount} path${
      touchedCount === 1 ? "" : "s"
    } · ${artifactCount} artifact${artifactCount === 1 ? "" : "s"}`;
  } catch {
    return null;
  }
}

function extractExternalSessions(raw: string): ExternalAgentSession[] {
  try {
    const parsed = JSON.parse(raw) as unknown;
    const values = Array.isArray(parsed) ? parsed : [parsed];
    return values.filter((value): value is ExternalAgentSession => {
      if (!value || typeof value !== "object") return false;
      const candidate = value as Record<string, unknown>;
      return (
        typeof candidate.sessionId === "string" &&
        (candidate.runtimeKind === "codex" || candidate.runtimeKind === "claude")
      );
    });
  } catch {
    return [];
  }
}

function mergeExternalSessions(
  current: ExternalAgentSession[] | undefined,
  next: ExternalAgentSession[],
): ExternalAgentSession[] | undefined {
  if (!next.length) return current;
  const map = new Map((current ?? []).map((session) => [session.sessionId, session]));
  for (const session of next) {
    map.set(session.sessionId, session);
  }
  return [...map.values()].sort((a, b) => b.createdAt - a.createdAt);
}

function isExternalSessionActive(session: ExternalAgentSession): boolean {
  return session.status === "pending" || session.status === "running";
}

function mergeExternalSessionArtifacts(
  current: AgentMessage["artifacts"],
  sessions: ExternalAgentSession[],
): AgentMessage["artifacts"] {
  let nextArtifacts = current;
  for (const session of sessions) {
    for (const artifact of session.artifacts ?? []) {
      nextArtifacts = appendUniqueArtifact(nextArtifacts, artifact);
    }
  }
  return nextArtifacts;
}

function applyExternalSessionUpdates(
  message: AgentMessage,
  sessions: ExternalAgentSession[],
): AgentMessage {
  const mergedSessions = mergeExternalSessions(message.externalSessions, sessions);
  if (!mergedSessions) {
    return message;
  }

  return {
    ...message,
    externalSessions: mergedSessions,
    artifacts: mergeExternalSessionArtifacts(message.artifacts, sessions),
  };
}

export function useAgentChat(
  initialChatScopeId?: string | null,
  onSessionChanged?: () => void | Promise<void>,
  activeRunBinding?: ActiveChatRunBinding | null,
) {
  const [messages, setMessages] = useState<AgentMessage[]>([]);
  const [isPlanning, setIsPlanning] = useState(false);
  const [isExecuting, setIsExecuting] = useState(false);
  const [currentPlan, setCurrentPlan] = useState<TaskPlan | null>(null);
  const [chatScopeId, setChatScopeId] = useState<string | null>(initialChatScopeId ?? null);
  const [chatSession, setChatSession] = useState<tauri.ChatSession | null>(null);
  const [chatTitleStatus, setChatTitleStatus] = useState<
    "idle" | "generating" | "ready" | "fallback"
  >("idle");
  const [historyCursorRowid, setHistoryCursorRowid] = useState<number | null>(
    null,
  );
  const [hasMoreHistory, setHasMoreHistory] = useState(false);
  const [isHydratingHistory, setIsHydratingHistory] = useState(false);
  const [activeSessionBinding, setActiveSessionBinding] =
    useState<ActiveChatSessionBinding | null>(null);
  const forgeRecordingIdRef = useRef<string | null>(null);
  const isHydratingRef = useRef(false);
  const hasHydratedRef = useRef(false);
  const messagesRef = useRef(messages);
  const sessionRunMessageIdRef = useRef<string | null>(null);
  const sessionRunIdRef = useRef<string | null>(null);
  const sessionToolCallIndexRef = useRef<RuntimeToolCallIndex>(new Map());
  messagesRef.current = messages;

  const { streamWithRouting } = useStreaming();
  const { createTask } = useTauriTask();
  const defaultRagTelemetry = {
    historySource: "persisted_long_chat",
    retrievalMode: "unavailable",
    embeddingProfile: "gemini-embedding-2-preview",
    executionMode: "local",
    workspaceMemoryEnabled: false,
    promptTokens: 0,
    completionTokens: 0,
    totalTokens: 0,
  } as const;

  const createTraceEntry = useCallback(
    (
      phase: AgentTraceEntry["phase"],
      label: string,
      extras?: Partial<Pick<AgentTraceEntry, "attempt" | "toolName" | "preview">>,
      timestampMs?: number,
    ): AgentTraceEntry => ({
      id: crypto.randomUUID(),
      phase,
      label,
      timestamp: new Date(timestampMs || Date.now()),
      attempt: extras?.attempt,
      toolName: extras?.toolName,
      preview: extras?.preview,
    }),
    [],
  );

  const captureForgeStep = useCallback(
    async (kind: string, label: string, payload?: Record<string, unknown>) => {
      try {
        if (!forgeRecordingIdRef.current) {
          const active = await tauri.getActiveWorkflowRecording();
          forgeRecordingIdRef.current = active?.id ?? null;
        }
        if (!forgeRecordingIdRef.current) return;
        await tauri.recordWorkflowStep({
          kind,
          label,
          payload: payload ?? {},
        });
      } catch (error) {
        console.error("Forge auto-capture failed:", error);
      }
    },
    [],
  );

  const applyRuntimeEventToMessage = useCallback(
    (
      message: AgentMessage,
      payload: RuntimeAgentEvent,
      toolCallIndex: RuntimeToolCallIndex,
    ): AgentMessage => {
      const upsert = (next: Partial<SpecialistRunState> & {
        agentId: string;
        role: SpecialistRunState["role"];
        status: SpecialistRunState["status"];
      }) => upsertSpecialistState(message.specialists, next);

      switch (payload.type) {
        case "supervisor_plan_created": {
          void captureForgeStep(
            "decision",
            payload.data?.summary || "supervisor_plan_created",
            {
              steps: Array.isArray(payload.data?.steps)
                ? payload.data.steps.length
                : 0,
            },
          );
          return {
            ...message,
            neuralState: "planning",
            runPhase: "planning",
            statusText: payload.data?.summary || "Supervisor plan ready",
            trace: [
              ...(message.trace || []),
              createTraceEntry(
                "act",
                payload.data?.summary || "Supervisor plan created",
                undefined,
                payload.timestampMs,
              ),
            ],
            supervisorPlan: {
              summary: payload.data?.summary || "Supervisor plan ready",
              steps: Array.isArray(payload.data?.steps)
                ? payload.data.steps
                : [],
              verificationRequired: payload.data?.verificationRequired || false,
              mode: payload.data?.mode,
              delegationPolicy: payload.data?.delegationPolicy,
              maxDepth: payload.data?.maxDepth,
              maxThreads: payload.data?.maxThreads,
              maxParallelSubagents: payload.data?.maxParallelSubagents,
              internalCoordinationLanguage:
                payload.data?.internalCoordinationLanguage,
              finalResponseLanguageMode:
                payload.data?.finalResponseLanguageMode,
            },
          };
        }
        case "specialist_spawned":
        case "specialist_status_changed": {
          const activeTool = payload.data?.activeTool || undefined;
          const waitingOnAirlock =
            payload.data?.status === "waiting_on_airlock";
          return {
            ...message,
            neuralState: waitingOnAirlock
              ? "planning"
              : activeTool
                ? resolveNeuralState(activeTool)
                : "planning",
            runPhase: waitingOnAirlock ? "awaiting_approval" : activeTool ? "tool_running" : "planning",
            statusText: payload.data?.detail || `${payload.data?.role || "specialist"}: ${payload.data?.status || "running"}`,
            trace: [
              ...(message.trace || []),
              createTraceEntry(
                waitingOnAirlock ? "approval" : "act",
                `${payload.data?.role || "specialist"}: ${payload.data?.status || "running"}`,
                { toolName: activeTool || undefined },
                payload.timestampMs,
              ),
            ],
            activeToolName: waitingOnAirlock
              ? "Awaiting Airlock approval"
              : activeTool
                ? getToolDisplayName(activeTool)
                : undefined,
            specialists: upsert({
              agentId: payload.data?.agentId ?? "unknown",
              role: payload.data?.role ?? "specialist",
              status: payload.data?.status ?? "planning",
              detail: payload.data?.detail,
              activeTool,
              dependsOn: payload.data?.dependsOn || [],
              startedAtMs: payload.data?.startedAtMs,
              finishedAtMs: payload.data?.finishedAtMs,
              toolCount: payload.data?.toolCount,
              writeLikeUsed: payload.data?.writeLikeUsed,
              parentAgentId: payload.data?.parentAgentId,
              depth: payload.data?.depth,
              branchId: payload.data?.branchId,
              spawnReason: payload.data?.spawnReason,
            }),
          };
        }
        case "specialist_completed": {
          return {
            ...message,
            neuralState: "thinking",
            runPhase: "responding",
            trace: [
              ...(message.trace || []),
              createTraceEntry(
                "act",
                `${payload.data?.role || "specialist"} completed`,
                {
                  preview: payload.data?.summary || payload.data?.responsePreview,
                },
                payload.timestampMs,
              ),
            ],
            ...clearTransientRunDecorations(),
            specialists: upsert({
              agentId: payload.data?.agentId ?? "unknown",
              role: payload.data?.role ?? "specialist",
              status: "completed",
              summary: payload.data?.summary,
              responsePreview: payload.data?.responsePreview,
              dependsOn: payload.data?.dependsOn || [],
              startedAtMs: payload.data?.startedAtMs,
              finishedAtMs: payload.data?.finishedAtMs,
              toolCount: payload.data?.toolCount,
              writeLikeUsed: payload.data?.writeLikeUsed,
              parentAgentId: payload.data?.parentAgentId,
              depth: payload.data?.depth,
              branchId: payload.data?.branchId,
              spawnReason: payload.data?.spawnReason,
            }),
          };
        }
        case "specialist_failed": {
          void captureForgeStep(
            "error",
            payload.data?.error || "specialist_failed",
            {
              agentId: payload.data?.agentId ?? null,
              role: payload.data?.role ?? null,
            },
          );
          return {
            ...message,
            neuralState: "thinking",
            runPhase: "responding",
            trace: [
              ...(message.trace || []),
              createTraceEntry(
                "error",
                `${payload.data?.role || "specialist"} failed`,
                { preview: payload.data?.error },
                payload.timestampMs,
              ),
            ],
            ...clearTransientRunDecorations(),
            specialists: upsert({
              agentId: payload.data?.agentId ?? "unknown",
              role: payload.data?.role ?? "specialist",
              status: "failed",
              error: payload.data?.error,
              dependsOn: payload.data?.dependsOn || [],
              startedAtMs: payload.data?.startedAtMs,
              finishedAtMs: payload.data?.finishedAtMs,
              toolCount: payload.data?.toolCount,
              writeLikeUsed: payload.data?.writeLikeUsed,
              parentAgentId: payload.data?.parentAgentId,
              depth: payload.data?.depth,
              branchId: payload.data?.branchId,
              spawnReason: payload.data?.spawnReason,
            }),
          };
        }
        case "stream_tool_call": {
          const functionName = payload.data?.name || "";
          const toolCallId =
            payload.data?.id ||
            `stream-tool-${payload.data?.index ?? functionName ?? "pending"}`;
          const state = String(payload.data?.state || "announced");
          if (functionName) {
            toolCallIndex.set(toolCallId, {
              name: functionName,
              arguments: payload.data?.arguments,
            });
          }
          return {
            ...message,
            neuralState: functionName ? resolveNeuralState(functionName) : "planning",
            runPhase: "tool_waiting",
            statusText:
              state === "ready"
                ? `Queued ${getToolDisplayName(functionName || "tool")}`
                : `Preparing ${getToolDisplayName(functionName || "tool")}`,
            activeToolName: getToolDisplayName(functionName || "tool"),
            trace: [
              ...(message.trace || []),
              createTraceEntry(
                "think",
                state === "ready"
                  ? `Tool ready: ${getToolDisplayName(functionName || "tool")}`
                  : `Tool draft: ${getToolDisplayName(functionName || "tool")}`,
                {
                  toolName: functionName || undefined,
                  preview: payload.data?.arguments?.slice(0, 180),
                },
                payload.timestampMs,
              ),
            ],
          };
        }
        case "usage": {
          return {
            ...message,
            ragTelemetry: {
              ...message.ragTelemetry,
              lastModel:
                payload.data?.model || message.ragTelemetry?.lastModel,
              promptTokens:
                (payload.data?.prompt_tokens as number | undefined) ??
                message.ragTelemetry?.promptTokens ??
                0,
              completionTokens:
                (payload.data?.completion_tokens as number | undefined) ??
                message.ragTelemetry?.completionTokens ??
                0,
              totalTokens:
                (payload.data?.total_tokens as number | undefined) ??
                message.ragTelemetry?.totalTokens ??
                0,
            },
          };
        }
        case "tool_call": {
          const functionName = payload.data?.function?.name || "";
          if (payload.data?.id) {
            toolCallIndex.set(payload.data.id, {
              name: functionName,
              arguments: payload.data?.function?.arguments,
            });
          }
          void captureForgeStep("tool_call", functionName || "tool_call", {
            toolCallId: payload.data?.id ?? null,
            arguments: payload.data?.function?.arguments ?? null,
          });
          const incomingLevel: number =
            (payload.data?.airlock_level as number | undefined) ?? 0;
          return {
            ...message,
            neuralState: resolveNeuralState(functionName),
            runPhase: "tool_running",
            statusText: `Executing ${getToolDisplayName(functionName)}`,
            airlockLevel: Math.max(message.airlockLevel ?? 0, incomingLevel),
            trace: [
              ...(message.trace || []),
              createTraceEntry(
                "tool",
                `Tool call: ${getToolDisplayName(functionName)}`,
                { toolName: functionName },
                payload.timestampMs,
              ),
            ],
            activeToolName: getToolDisplayName(functionName),
          };
        }
        case "tool_result": {
          const toolCall = payload.data?.id
            ? toolCallIndex.get(payload.data.id)
            : undefined;
          const artifacts =
            toolCall && typeof payload.data?.result === "string"
              ? artifactsFromToolResult(
                  toolCall.name,
                  toolCall.arguments,
                  payload.data.result,
                )
              : [];
          const externalSessions =
            toolCall && typeof payload.data?.result === "string"
              ? extractExternalSessions(payload.data.result)
              : [];
          const externalSessionSummary =
            toolCall &&
            typeof payload.data?.result === "string" &&
            (toolCall.name === "spawn_external_agent_session" ||
              toolCall.name === "send_external_agent_message" ||
              toolCall.name === "wait_external_agent_session" ||
              toolCall.name === "cancel_external_agent_session" ||
              toolCall.name === "list_external_agent_sessions")
              ? summarizeExternalSessionResult(payload.data.result)
              : null;
          void captureForgeStep(
            "tool_result",
            payload.data?.id || "tool_result",
            {
              toolCallId: payload.data?.id ?? null,
              resultPreview: payload.data?.result ?? null,
            },
          );
          return {
            ...applyExternalSessionUpdates(message, externalSessions),
            artifacts: artifacts.length
              ? artifacts.reduce(
                  (current, nextArtifact) =>
                    appendUniqueArtifact(current, nextArtifact),
                  mergeExternalSessionArtifacts(message.artifacts, externalSessions),
                )
              : mergeExternalSessionArtifacts(message.artifacts, externalSessions),
            neuralState: "thinking",
            runPhase: "responding",
            statusText: externalSessionSummary || "Tool execution completed",
            trace: [
              ...(message.trace || []),
              createTraceEntry(
                "tool",
                externalSessionSummary
                  ? externalSessionSummary
                  : `Tool result: ${payload.data?.id || "completed"}`,
                {
                  preview: externalSessionSummary
                    ? externalSessionSummary
                    : typeof payload.data?.result === "string"
                      ? payload.data.result.slice(0, 180)
                      : undefined,
                },
                payload.timestampMs,
              ),
            ],
            ...clearTransientRunDecorations(),
          };
        }
        case "thought":
        case "stream_chunk":
        case "supervisor_summary":
          return {
            ...message,
            content: appendStreamText(message.content, payload.data?.text),
            neuralState: "thinking",
            runPhase:
              message.trace?.some((entry) => entry.phase === "tool") ||
              message.activeToolName
                ? "responding"
                : "streaming",
            statusText:
              payload.type === "supervisor_summary"
                ? payload.data?.summary || message.statusText
                : "Streaming response",
            activeToolName:
              payload.type === "stream_chunk" ? message.activeToolName : undefined,
          };
        case "status": {
          const statusText = String(payload.data || "");
          const lower = statusText.toLowerCase();
          if (lower.includes("retry")) {
            void captureForgeStep("retry", statusText, {});
          } else if (
            lower.includes("error") ||
            lower.includes("failed") ||
            lower.includes("exception")
          ) {
            void captureForgeStep("error", statusText, {});
          }
          if (statusText.startsWith("RAG_TELEMETRY:")) {
            try {
              const raw = statusText.slice("RAG_TELEMETRY:".length);
              const parsed = JSON.parse(raw) as {
                history_source?: string;
                retrieval_mode?: string;
                embedding_profile?: string;
              };
              const nextTelemetry = {
                historySource:
                  parsed.history_source ||
                  message.ragTelemetry?.historySource ||
                  defaultRagTelemetry.historySource,
                retrievalMode:
                  parsed.retrieval_mode ||
                  message.ragTelemetry?.retrievalMode ||
                  defaultRagTelemetry.retrievalMode,
                embeddingProfile:
                  parsed.embedding_profile ||
                  message.ragTelemetry?.embeddingProfile ||
                  defaultRagTelemetry.embeddingProfile,
                executionMode:
                  message.ragTelemetry?.executionMode ||
                  defaultRagTelemetry.executionMode,
                workspaceMemoryEnabled:
                  message.ragTelemetry?.workspaceMemoryEnabled ??
                  defaultRagTelemetry.workspaceMemoryEnabled,
                workspaceMemoryRoot: message.ragTelemetry?.workspaceMemoryRoot,
                lastModel: message.ragTelemetry?.lastModel,
                promptTokens:
                  message.ragTelemetry?.promptTokens ??
                  defaultRagTelemetry.promptTokens,
                completionTokens:
                  message.ragTelemetry?.completionTokens ??
                  defaultRagTelemetry.completionTokens,
                totalTokens:
                  message.ragTelemetry?.totalTokens ??
                  defaultRagTelemetry.totalTokens,
                compressionApplied: message.ragTelemetry?.compressionApplied,
                compressionTriggerTokens:
                  message.ragTelemetry?.compressionTriggerTokens,
              };
              if (
                message.ragTelemetry?.historySource ===
                  nextTelemetry.historySource &&
                message.ragTelemetry?.retrievalMode ===
                  nextTelemetry.retrievalMode &&
                message.ragTelemetry?.embeddingProfile ===
                  nextTelemetry.embeddingProfile
              ) {
                return message;
              }
              return {
                ...message,
                ragTelemetry: nextTelemetry,
              };
            } catch {
              return message;
            }
          }
          if (statusText.startsWith("RUN_USAGE:")) {
            try {
              const raw = statusText.slice("RUN_USAGE:".length);
              const parsed = JSON.parse(raw) as {
                model?: string;
                prompt_tokens?: number;
                completion_tokens?: number;
                total_tokens?: number;
              };
              return {
                ...message,
                ragTelemetry: {
                  ...message.ragTelemetry,
                  lastModel:
                    parsed.model || message.ragTelemetry?.lastModel,
                  promptTokens:
                    parsed.prompt_tokens ?? message.ragTelemetry?.promptTokens ?? 0,
                  completionTokens:
                    parsed.completion_tokens ??
                    message.ragTelemetry?.completionTokens ??
                    0,
                  totalTokens:
                    parsed.total_tokens ?? message.ragTelemetry?.totalTokens ?? 0,
                },
              };
            } catch {
              return message;
            }
          }
          if (statusText.startsWith("CONTEXT_COMPACTION:")) {
            try {
              const raw = statusText.slice("CONTEXT_COMPACTION:".length);
              const parsed = JSON.parse(raw) as {
                applied?: boolean;
                trigger_tokens?: number;
              };
              return {
                ...message,
                ragTelemetry: {
                  ...message.ragTelemetry,
                  compressionApplied:
                    parsed.applied ?? message.ragTelemetry?.compressionApplied,
                  compressionTriggerTokens:
                    parsed.trigger_tokens ||
                    message.ragTelemetry?.compressionTriggerTokens,
                },
              };
            } catch {
              return message;
            }
          }
          const waitingOnMcpApproval =
            statusText.toLowerCase().includes("approval") &&
            statusText.toLowerCase().includes("mcp");
          const waitingOnAirlockApproval =
            statusText.toLowerCase().includes("awaiting airlock approval");
          const statusLower = statusText.toLowerCase();
          const isRetry = statusLower.includes("retry");
          const isError =
            statusLower.includes("error") ||
            statusLower.includes("failed") ||
            statusLower.includes("exception");
          const isCancelled =
            statusLower.includes("execution cancelled") ||
            statusLower.includes("terminated by fleet kill switch") ||
            statusLower.includes("terminated by user") ||
            statusLower.includes("cancelled by user");
          const tracePhase: AgentTraceEntry["phase"] =
            waitingOnMcpApproval || waitingOnAirlockApproval
              ? "approval"
              : isRetry
                ? "retry"
                : isError
                  ? "error"
                  : isCancelled
                    ? "cancelled"
                    : "think";
          const nextRunPhase = deriveStatusRunPhase(statusText);
          return {
            ...message,
            neuralState: "planning",
            ...(nextRunPhase ? { runPhase: nextRunPhase } : {}),
            statusText,
            runState: isCancelled ? "cancelled" : message.runState,
            trace: [
              ...(message.trace || []),
              createTraceEntry(tracePhase, statusText, undefined, payload.timestampMs),
            ],
            activeToolName: waitingOnMcpApproval
              ? "Awaiting MCP approval"
              : waitingOnAirlockApproval
                ? "Awaiting Airlock approval"
                : undefined,
          };
        }
        default:
          return message;
      }
    },
    [captureForgeStep, createTraceEntry],
  );

  const clearMessages = useCallback(() => {
    setMessages([]);
    setCurrentPlan(null);
  }, []);

  const mapPersistedRoleToUiType = useCallback(
    (role: "user" | "assistant" | "system"): AgentMessage["type"] => {
      if (role === "assistant") return "agent";
      if (role === "system") return "system";
      return "user";
    },
    [],
  );

  const ensureChatScope = useCallback(async () => {
    // When the parent switches threads, prefer the externally-selected scope
    // immediately instead of reusing a stale previous chatScopeId mid-transition.
    if (initialChatScopeId && initialChatScopeId !== chatScopeId) {
      setChatScopeId(initialChatScopeId);
      return initialChatScopeId;
    }
    if (chatScopeId) return chatScopeId;
    if (initialChatScopeId) {
      setChatScopeId(initialChatScopeId);
      return initialChatScopeId;
    }
    const scope = await tauri.getDefaultChatScope();
    setChatScopeId(scope);
    return scope;
  }, [chatScopeId, initialChatScopeId]);

  useEffect(() => {
    if (!initialChatScopeId || initialChatScopeId === chatScopeId) return;
    setChatScopeId(initialChatScopeId);
    setMessages([]);
    setCurrentPlan(null);
    setHistoryCursorRowid(null);
    setHasMoreHistory(false);
    setChatTitleStatus("idle");
    setChatSession(null);
    hasHydratedRef.current = false;
    isHydratingRef.current = false;
  }, [chatScopeId, initialChatScopeId]);

  useEffect(() => {
    const scopedRemoteBinding =
      activeRunBinding?.chatId && activeRunBinding.chatId === chatScopeId
        ? {
            workspaceId:
              activeRunBinding.workspacePath ||
              activeRunBinding.workspaceId,
            workspacePath:
              activeRunBinding.workspacePath ||
              activeRunBinding.workspaceId,
            workspaceName: activeRunBinding.workspaceName,
            chatId: activeRunBinding.chatId,
            runId: activeRunBinding.runId || "",
            source: activeRunBinding.source || "remote",
            connectorId: activeRunBinding.connectorId,
            elapsedSecs: activeRunBinding.elapsedSecs,
          }
        : null;

    if (scopedRemoteBinding?.runId) {
      setActiveSessionBinding(scopedRemoteBinding);
      return;
    }

    if (!chatScopeId) {
      setActiveSessionBinding(null);
      return;
    }

    let cancelled = false;
    void tauri
      .listActiveSessions()
      .then((sessions) => {
        if (cancelled) return;
        const current = sessions.find((session) => session.chatId === chatScopeId);
        setActiveSessionBinding(
          current
            ? {
                workspaceId: current.workspaceId,
                workspacePath: current.workspaceId,
                chatId: current.chatId,
                runId: current.runId,
                source: current.source,
                connectorId: current.connectorId,
                elapsedSecs: current.elapsedSecs,
              }
            : null,
        );
      })
      .catch((error) => {
        console.error("Failed to load active sessions for chat scope:", error);
      });

    return () => {
      cancelled = true;
    };
  }, [activeRunBinding, chatScopeId]);

  useEffect(() => {
    let cancelled = false;
    const unlisteners: Array<() => void> = [];

    void listen<tauri.RemoteSessionStartedEvent>("session://started", (event) => {
      if (cancelled) return;
      const payload = event.payload;
      if (!chatScopeId || payload.chatId !== chatScopeId) return;
      setActiveSessionBinding({
        workspaceId: payload.workspacePath || payload.workspaceId,
        workspacePath: payload.workspacePath || payload.workspaceId,
        workspaceName: payload.workspaceName,
        chatId: payload.chatId,
        runId: payload.runId,
        source: payload.source || "remote",
        connectorId: payload.connectorId,
      });
    }).then((fn) => {
      if (cancelled) fn();
      else unlisteners.push(fn);
    });

    void listen<tauri.RemoteSessionFinishedEvent>("session://finished", (event) => {
      if (cancelled) return;
      const payload = event.payload;
      if (!chatScopeId || payload.chatId !== chatScopeId) return;
      setActiveSessionBinding((current) => {
        if (!current) return null;
        if (payload.runId && current.runId && payload.runId !== current.runId) {
          return current;
        }
        return null;
      });
    }).then((fn) => {
      if (cancelled) fn();
      else unlisteners.push(fn);
    });

    return () => {
      cancelled = true;
      unlisteners.forEach((fn) => fn());
    };
  }, [chatScopeId]);

  const refreshChatSession = useCallback(
    async (scopeOverride?: string) => {
      try {
        const scope = scopeOverride || (await ensureChatScope());
        const session = await tauri.getChatSession(scope);
        setChatSession(session);
        setChatTitleStatus(session.title?.trim() ? "ready" : "idle");
        await onSessionChanged?.();
        return session;
      } catch (error) {
        console.error("Failed to load chat session:", error);
        return null;
      }
    },
    [ensureChatScope, onSessionChanged],
  );

  useEffect(() => {
    const runId = activeSessionBinding?.runId ?? null;
    const previousRunId = sessionRunIdRef.current;
    sessionRunIdRef.current = runId;

    if (previousRunId !== runId) {
      sessionRunMessageIdRef.current = null;
      sessionToolCallIndexRef.current.clear();
    }

    if (!runId) {
      sessionRunMessageIdRef.current = null;
      sessionToolCallIndexRef.current.clear();
      return;
    }

    if (isHydratingRef.current || isHydratingHistory) {
      return;
    }

    const existing = messagesRef.current.find(
      (message) => message.requestContext?.runId === runId,
    );
    if (existing) {
      sessionRunMessageIdRef.current = existing.id;
      return;
    }

    if (!sessionRunMessageIdRef.current) {
      sessionRunMessageIdRef.current = crypto.randomUUID();
    }

    const placeholderId = sessionRunMessageIdRef.current;
    sessionToolCallIndexRef.current = new Map();
    setMessages((prev) => {
      if (
        prev.some(
          (message) =>
            message.id === placeholderId ||
            message.requestContext?.runId === runId,
        )
      ) {
        return prev;
      }

      const sessionLabel =
        activeSessionBinding?.source === "native_modal"
          ? "Quick Delegate"
          : activeSessionBinding?.source === "remote"
            ? "Remote session"
            : "Background run";
      const placeholder: AgentMessage = {
        id: placeholderId,
        type: "agent",
        content: activeSessionBinding?.workspaceName?.trim().length
          ? `${sessionLabel} running in ${activeSessionBinding.workspaceName}.`
          : `${sessionLabel} running.`,
        isLoading: true,
        timestamp: new Date(),
        neuralState: "thinking",
        runPhase: "starting",
        statusText: "Binding live session",
        runState: "running",
        requestContext: {
          runId,
          workspaceId:
            activeSessionBinding?.workspacePath || activeSessionBinding?.workspaceId,
          chatScopeId: activeSessionBinding?.chatId || chatScopeId || undefined,
          startedAtMs: Date.now(),
        },
        trace: [
          createTraceEntry(
            "think",
            `${sessionLabel} bound. Streaming live updates.`,
          ),
        ],
      };

      return [...prev, placeholder];
    });

    void onSessionChanged?.();
  }, [
    activeSessionBinding,
    chatScopeId,
    createTraceEntry,
    isHydratingHistory,
    onSessionChanged,
  ]);

  const clearMessagesAndContext = useCallback(async (chatId: string) => {
    await tauri.clearChatHistory(chatScopeId || chatId);
    setMessages([]);
    setCurrentPlan(null);
    setHistoryCursorRowid(null);
    setHasMoreHistory(false);
    setChatTitleStatus("idle");
    hasHydratedRef.current = true;
    await refreshChatSession(chatScopeId || chatId);
  }, [chatScopeId, refreshChatSession]);

  useEffect(() => {
    const runId = activeSessionBinding?.runId ?? null;
    if (!runId) {
      sessionRunMessageIdRef.current = null;
      sessionToolCallIndexRef.current.clear();
      return;
    }

    sessionToolCallIndexRef.current = new Map();

    let cancelled = false;
    let unlisten: (() => void) | null = null;
    let queuedRuntimeEvents: RuntimeAgentEvent[] = [];
    let runtimeEventFrame: number | null = null;

    const flushRuntimeEvents = () => {
      runtimeEventFrame = null;
      if (!sessionRunMessageIdRef.current || !queuedRuntimeEvents.length) return;
      const batch = queuedRuntimeEvents;
      queuedRuntimeEvents = [];
      setMessages((prev) =>
        updateMessageById(prev, sessionRunMessageIdRef.current as string, (message) =>
          batch.reduce(
            (current, payload) =>
              applyRuntimeEventToMessage(
                current,
                payload,
                sessionToolCallIndexRef.current,
              ),
            message,
          ),
        ),
      );
    };

    const scheduleRuntimeEvent = (payload: RuntimeAgentEvent) => {
      queuedRuntimeEvents.push(payload);
      if (runtimeEventFrame != null) return;
      runtimeEventFrame = window.requestAnimationFrame(flushRuntimeEvents);
    };

    void listen<RuntimeAgentEvent>("agent://event", (event) => {
      if (cancelled) return;
      if (event.payload?.runId && event.payload.runId !== runId) return;
      if (!sessionRunMessageIdRef.current) return;
      scheduleRuntimeEvent(event.payload);
    }).then((fn) => {
      if (cancelled) {
        fn();
      } else {
        unlisten = fn;
      }
    });

    return () => {
      cancelled = true;
      unlisten?.();
      if (runtimeEventFrame != null) {
        window.cancelAnimationFrame(runtimeEventFrame);
      }
    };
  }, [activeSessionBinding?.runId, applyRuntimeEventToMessage]);

  useEffect(() => {
    const runId = activeSessionBinding?.runId ?? null;
    if (!runId) return;

    let cancelled = false;
    let unlisten: (() => void) | null = null;

    void listen<{
      chatId: string;
      workspaceId?: string;
      workspacePath?: string;
      runId?: string;
      source?: string;
    }>("session://finished", (event) => {
      if (cancelled) return;
      const payload = event.payload;

      const matchesRun = payload.runId ? payload.runId === runId : false;
      const matchesChat =
        activeSessionBinding?.chatId && payload.chatId === activeSessionBinding.chatId;
      if (!matchesRun && !matchesChat) return;

      const messageId = sessionRunMessageIdRef.current;
      if (!messageId) return;

      setMessages((prev) =>
        updateMessageById(prev, messageId, (message) => ({
          ...message,
          isLoading: false,
          runState: message.runState === "cancelled" ? "cancelled" : "completed",
          neuralState: undefined,
          activeToolName: undefined,
          trace: [
            ...(message.trace || []),
            createTraceEntry("done", "Background session finished."),
          ],
        })),
      );
    }).then((fn) => {
      if (cancelled) {
        fn();
      } else {
        unlisten = fn;
      }
    });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [activeSessionBinding?.chatId, activeSessionBinding?.runId, createTraceEntry]);

  useEffect(() => {
    const activeSessions = messages.filter((message) =>
      (message.externalSessions ?? []).some(isExternalSessionActive),
    );
    if (!activeSessions.length) {
      return;
    }

    let cancelled = false;
    let timeoutId: number | null = null;
    let inFlight = false;

    const scheduleNextPoll = () => {
      if (cancelled) return;
      timeoutId = window.setTimeout(pollExternalSessions, EXTERNAL_SESSION_POLL_INTERVAL_MS);
    };

    const pollExternalSessions = async () => {
      if (cancelled || inFlight) {
        return;
      }

      const pendingLookups = messagesRef.current.flatMap((message) =>
        (message.externalSessions ?? [])
          .filter(isExternalSessionActive)
          .map((session) => ({
            messageId: message.id,
            sessionId: session.sessionId,
          })),
      );

      if (!pendingLookups.length) {
        return;
      }

      inFlight = true;
      try {
        const results = await Promise.all(
          pendingLookups.map(async ({ messageId, sessionId }) => {
            try {
              const session = await tauri.getExternalAgentSession(sessionId);
              return { messageId, session };
            } catch (error) {
              console.error(`Failed to refresh external session ${sessionId}:`, error);
              return null;
            }
          }),
        );

        if (cancelled) {
          return;
        }

        const updatesByMessage = new Map<string, ExternalAgentSession[]>();
        for (const result of results) {
          if (!result) continue;
          const existing = updatesByMessage.get(result.messageId) ?? [];
          existing.push(result.session);
          updatesByMessage.set(result.messageId, existing);
        }

        if (updatesByMessage.size > 0) {
          setMessages((prev) =>
            prev.map((message) => {
              const nextSessions = updatesByMessage.get(message.id);
              if (!nextSessions?.length) {
                return message;
              }
              return applyExternalSessionUpdates(message, nextSessions);
            }),
          );
        }
      } finally {
        inFlight = false;
        const shouldContinue = messagesRef.current.some((message) =>
          (message.externalSessions ?? []).some(isExternalSessionActive),
        );
        if (shouldContinue) {
          scheduleNextPoll();
        }
      }
    };

    void pollExternalSessions();

    return () => {
      cancelled = true;
      if (timeoutId != null) {
        window.clearTimeout(timeoutId);
      }
    };
  }, [messages]);

  const hydrateLongChatHistory = useCallback(async () => {
    if (isHydratingRef.current || hasHydratedRef.current) return;
    isHydratingRef.current = true;
    setIsHydratingHistory(true);
    try {
      const scope = await ensureChatScope();
      await refreshChatSession(scope);
      const window = await tauri.getChatHistoryWindow(scope, undefined, 100);
      const hydratedMessages: AgentMessage[] = window.messages.map((msg) => ({
        id: msg.id,
        type: mapPersistedRoleToUiType(msg.role),
        content: msg.content,
        artifacts: msg.artifacts ?? undefined,
        timestamp: new Date(msg.created_at),
        ragTelemetry:
          msg.role === "assistant"
            ? { ...defaultRagTelemetry }
            : undefined,
      }));
      const runtimeTelemetry = await tauri.getChatRuntimeTelemetry(scope);
      if (runtimeTelemetry) {
        for (let i = hydratedMessages.length - 1; i >= 0; i -= 1) {
          if (hydratedMessages[i].type === "agent") {
            hydratedMessages[i] = {
              ...hydratedMessages[i],
              ragTelemetry: {
                ...hydratedMessages[i].ragTelemetry,
                historySource: runtimeTelemetry.history_source || defaultRagTelemetry.historySource,
                retrievalMode:
                  runtimeTelemetry.retrieval_mode || defaultRagTelemetry.retrievalMode,
                embeddingProfile:
                  runtimeTelemetry.embedding_profile ||
                  defaultRagTelemetry.embeddingProfile,
                executionMode:
                  runtimeTelemetry.execution_mode || defaultRagTelemetry.executionMode,
                workspaceMemoryEnabled:
                  runtimeTelemetry.workspace_memory_enabled ??
                  defaultRagTelemetry.workspaceMemoryEnabled,
                workspaceMemoryRoot:
                  runtimeTelemetry.workspace_memory_root ?? undefined,
                lastModel: runtimeTelemetry.last_model ?? undefined,
                promptTokens:
                  runtimeTelemetry.prompt_tokens ?? defaultRagTelemetry.promptTokens,
                completionTokens:
                  runtimeTelemetry.completion_tokens ??
                  defaultRagTelemetry.completionTokens,
                totalTokens:
                  runtimeTelemetry.total_tokens ?? defaultRagTelemetry.totalTokens,
              },
            };
            break;
          }
        }
      }
      setMessages((prev) => {
        if (prev.length === 0) return hydratedMessages;

        const merged = new Map<string, AgentMessage>();
        for (const message of hydratedMessages) {
          merged.set(message.id, message);
        }

        for (const message of prev) {
          // Preserve optimistic/live messages while hydration catches up.
          if (
            message.isLoading ||
            message.runState === "running" ||
            message.runState === "failed" ||
            !merged.has(message.id)
          ) {
            merged.set(message.id, message);
          }
        }

        return Array.from(merged.values()).sort(
          (a, b) => a.timestamp.getTime() - b.timestamp.getTime(),
        );
      });
      setHistoryCursorRowid(window.next_cursor_rowid ?? null);
      setHasMoreHistory(window.has_more);
      hasHydratedRef.current = true;
    } catch (error) {
      console.error("Failed to hydrate long chat history:", error);
    } finally {
      isHydratingRef.current = false;
      setIsHydratingHistory(false);
    }
  }, [
    chatScopeId,
    ensureChatScope,
    mapPersistedRoleToUiType,
    refreshChatSession,
  ]);

  const switchChat = useCallback(async (newChatId: string) => {
    setChatScopeId(newChatId);
    setMessages([]);
    setCurrentPlan(null);
    setHistoryCursorRowid(null);
    setHasMoreHistory(false);
    setChatTitleStatus("idle");
    hasHydratedRef.current = false;
    isHydratingRef.current = false;
  }, []);

  const refreshActiveChat = useCallback(async () => {
    const scope = await ensureChatScope();
    setMessages([]);
    setCurrentPlan(null);
    setHistoryCursorRowid(null);
    setHasMoreHistory(false);
    setChatTitleStatus("idle");
    setChatSession(null);
    hasHydratedRef.current = false;
    isHydratingRef.current = false;
    await hydrateLongChatHistory();
    return scope;
  }, [ensureChatScope, hydrateLongChatHistory]);

  useEffect(() => {
    if (!chatScopeId) return;
    void hydrateLongChatHistory();
  }, [chatScopeId, hydrateLongChatHistory]);

  const loadOlderHistory = useCallback(async () => {
    if (!hasMoreHistory || historyCursorRowid == null || isHydratingHistory) return;
    setIsHydratingHistory(true);
    try {
      const scope = await ensureChatScope();
      const window = await tauri.getChatHistoryWindow(
        scope,
        historyCursorRowid,
        100,
      );
      const olderMessages: AgentMessage[] = window.messages.map((msg) => ({
        id: msg.id,
        type: mapPersistedRoleToUiType(msg.role),
        content: msg.content,
        artifacts: msg.artifacts ?? undefined,
        timestamp: new Date(msg.created_at),
        ragTelemetry:
          msg.role === "assistant"
            ? { ...defaultRagTelemetry }
            : undefined,
      }));
      setMessages((prev) => [...olderMessages, ...prev]);
      setHistoryCursorRowid(window.next_cursor_rowid ?? null);
      setHasMoreHistory(window.has_more);
    } catch (error) {
      console.error("Failed to load older chat history:", error);
    } finally {
      setIsHydratingHistory(false);
    }
  }, [
    ensureChatScope,
    hasMoreHistory,
    historyCursorRowid,
    isHydratingHistory,
    mapPersistedRoleToUiType,
  ]);


  const streamChat = useCallback(
    async (instruction: string, modelId: string, hiddenContext?: string) => {
      const userMsg: AgentMessage = {
        id: crypto.randomUUID(),
        type: "user",
        content: instruction,
        timestamp: new Date(),
      };

      // Optimistically update UI
      setMessages((prev) => [...prev, userMsg]);

      const agentMsgId = crypto.randomUUID();
      const initialAgentMsg: AgentMessage = {
        id: agentMsgId,
        type: "agent",
        content: "",
        isLoading: true,
        timestamp: new Date(),
        modelUsed: { name: modelId, thinkingEnabled: false },
      };
      setMessages((prev) => [...prev, initialAgentMsg]);

      let accumulatedContent = "";

      try {
        // Construct history from current messages
        const history = messages
          .filter((m) => !m.isLoading && !m.content.startsWith("[Error"))
          .map((m) => ({
            role: m.type === "agent" ? "assistant" : ("user" as const),
            content: m.content,
          }));

        // Add the new user message (with hidden context if provided)
        const effectiveContent = hiddenContext
          ? `${hiddenContext}\n\nUser Query: "${instruction}"`
          : instruction;

        const fullMessages = [
          ...history,
          { role: "user", content: effectiveContent },
        ];

        await streamWithRouting(
          {
            messages: fullMessages,
            model: modelId,
          },
          (event) => {
            if (event.event === "chunk") {
              accumulatedContent += event.data.content;

              setMessages((prev) =>
                prev.map((m) =>
                  m.id === agentMsgId
                    ? { ...m, content: accumulatedContent, isLoading: false }
                    : m,
                ),
              );
            } else if (event.event === "thought") {
              setMessages((prev) =>
                prev.map((m) =>
                  m.id === agentMsgId
                    ? {
                        ...m,
                        thought: event.data.content,
                        modelUsed: { name: modelId, thinkingEnabled: true },
                      }
                    : m,
                ),
              );
            } else if (event.event === "finished") {
              // Parse tool calls from the completed response via Rust.
              void tauri.parseToolCalls(accumulatedContent).then((detected) => {
                setMessages((prev) =>
                  prev.map((m) =>
                    m.id === agentMsgId
                      ? {
                          ...m,
                          isLoading: false,
                          toolCalls: detected.length > 0 ? detected : undefined,
                        }
                      : m,
                  ),
                );
              });
              setMessages((prev) =>
                prev.map((m) =>
                  m.id === agentMsgId ? { ...m, isLoading: false } : m,
                ),
              );
            } else if (event.event === "error") {
              setMessages((prev) =>
                prev.map((m) =>
                  m.id === agentMsgId
                    ? {
                        ...m,
                        content:
                          accumulatedContent +
                          `\n[Error: ${event.data.message}]`,
                        isLoading: false,
                      }
                    : m,
                ),
              );
            }
          },
        );
      } catch (err) {
        setMessages((prev) =>
          prev.map((m) =>
            m.id === agentMsgId
              ? {
                  ...m,
                  content: accumulatedContent + `\n[Error: ${err}]`,
                  isLoading: false,
                }
              : m,
          ),
        );
      }
    },
    [streamWithRouting, messages],
  );

  const sendInstruction = useCallback(
    async (instruction: string, workspacePath: string, modelId: string) => {
      const userMsg: AgentMessage = {
        id: crypto.randomUUID(),
        type: "user",
        content: instruction,
        timestamp: new Date(),
      };
      setMessages((prev) => [...prev, userMsg]);

      setIsPlanning(true);

      // Model ID prefix stripping (rainy:/cowork:) is handled in Rust's create_task.
      const targetProvider = "rainyapi";

      try {
        const task = await createTask(
          instruction,
          targetProvider as any,
          modelId,
          workspacePath,
        );

        const planMsgId = crypto.randomUUID();
        const planMsg: AgentMessage = {
          id: planMsgId,
          type: "agent",
          content: "Planning task...",
          isLoading: true,
          timestamp: new Date(),
        };
        setMessages((prev) => [...prev, planMsg]);

        await tauri.executeTask(task.id, (event) => {
          if (event.event === "progress") {
            setMessages((prev) =>
              prev.map((m) =>
                m.id === planMsgId
                  ? {
                      ...m,
                      content: `Executing... ${event.data.progress}%: ${event.data.message || ""}`,
                    }
                  : m,
              ),
            );
          } else if (event.event === "completed") {
            setIsExecuting(false);
            setMessages((prev) =>
              prev.map((m) =>
                m.id === planMsgId
                  ? {
                      ...m,
                      content: "Task completed successfully.",
                      isLoading: false,
                      result: { totalSteps: 0, totalChanges: 0, errors: [] },
                    }
                  : m,
              ),
            );
          } else if (event.event === "failed") {
            setIsExecuting(false);
            setMessages((prev) =>
              prev.map((m) =>
                m.id === planMsgId
                  ? {
                      ...m,
                      content: `Task failed: ${event.data.error}`,
                      isLoading: false,
                    }
                  : m,
              ),
            );
          }
        });

        setIsPlanning(false);
        setIsExecuting(true);
      } catch (err) {
        setIsPlanning(false);
        setIsExecuting(false);
        console.error(err);
      }
    },
    [createTask],
  );

  const executeDiscussedPlan = useCallback(
    async (workspaceId: string, _modelId: string) => {
      if (messages.length === 0) return;

      const lastAgentMessage = [...messages]
        .reverse()
        .find((m) => m.type === "agent" && !m.isLoading);

      if (!lastAgentMessage) return;

      setIsExecuting(true);
      const statusMsgId = crypto.randomUUID();
      setMessages((prev) => [
        ...prev,
        {
          id: statusMsgId,
          type: "agent",
          content: "Parsing and executing plan...",
          isLoading: true,
          timestamp: new Date(),
        },
      ]);

      try {
        const result = await tauri.executePlanFromContent(
          workspaceId,
          lastAgentMessage.content,
        );

        if (!result.success && !result.summary) {
          // No operations found
          setMessages((prev) =>
            prev.map((m) =>
              m.id === statusMsgId
                ? {
                    ...m,
                    content:
                      "❌ Could not find any executable operations in the plan. Please ask the AI to use write_file, read_file, or list_files commands.",
                    isLoading: false,
                  }
                : m,
            ),
          );
          return;
        }

        setMessages((prev) =>
          prev.map((m) =>
            m.id === statusMsgId
              ? {
                  ...m,
                  content: result.success
                    ? result.summary
                    : `❌ ${result.error ?? "Execution failed"}`,
                  isLoading: false,
                }
              : m,
          ),
        );
      } catch (err: any) {
        setMessages((prev) =>
          prev.map((m) =>
            m.id === statusMsgId
              ? {
                  ...m,
                  content: `❌ ${err.message ?? String(err)}`,
                  isLoading: false,
                }
              : m,
          ),
        );
      } finally {
        setIsExecuting(false);
      }
    },
    [messages],
  );

  const executeToolCalls = useCallback(
    async (
      messageId: string,
      toolCalls: Array<{ skill: string; method: string; params: any }>,
      workspaceId: string,
    ) => {
      setIsExecuting(true);
      const statusMsgId = crypto.randomUUID();

      setMessages((prev) => [
        ...prev,
        {
          id: statusMsgId,
          type: "agent",
          content: `Executing ${toolCalls.length} operation(s)...`,
          isLoading: true,
          timestamp: new Date(),
        },
      ]);

      try {
        const results = [];
        for (const call of toolCalls) {
          setMessages((prev) =>
            prev.map((m) =>
              m.id === statusMsgId
                ? {
                    ...m,
                    content: `⏳ ${call.method}("${call.params.path || call.params.query}")...`,
                  }
                : m,
            ),
          );

          const result = await tauri.executeSkill(
            workspaceId,
            call.skill,
            call.method,
            call.params,
            workspaceId,
          );
          results.push({ call, result });

          if (!result.success)
            throw new Error(`${call.method} failed: ${result.error}`);
        }

        setMessages((prev) =>
          prev.map((m) =>
            m.id === messageId ? { ...m, isExecuted: true } : m,
          ),
        );

        const successDetails = results
          .map(
            (r) =>
              `✅ ${r.call.method}("${r.call.params.path || r.call.params.query}")`,
          )
          .join("\n");

        setMessages((prev) =>
          prev.map((m) =>
            m.id === statusMsgId
              ? {
                  ...m,
                  content: `**Execution Complete**\n\n${successDetails}`,
                  isLoading: false,
                }
              : m,
          ),
        );
      } catch (err: any) {
        setMessages((prev) =>
          prev.map((m) =>
            m.id === statusMsgId
              ? { ...m, content: `❌ ${err.message}`, isLoading: false }
              : m,
          ),
        );
      } finally {
        setIsExecuting(false);
      }
    },
    [],
  );

  const executePlan = useCallback(async (_planId: string) => {
    // Legacy stub
  }, []);

  const cancelPlan = useCallback(async (_planId: string) => {
    // Legacy stub
  }, []);

  const runNativeAgent = useCallback(
    async (
      instruction: string,
      modelId: string,
      workspaceId: string,
      agentSpecId?: string,
      reasoningEffort?: string,
      attachments?: ChatAttachment[],
      displayInstruction?: string,
    ): Promise<{
      ok: boolean;
      chatScopeId: string | null;
      runId?: string;
      error?: string;
      actualToolIds: string[];
      actualTouchedPaths: string[];
      producedArtifactPaths: string[];
    }> => {
      const resolvedChatScopeId = await ensureChatScope().catch((error) => {
        console.error("Failed to resolve chat scope:", error);
        throw error;
      });
      const clientRunId = crypto.randomUUID();
      const userMsg: AgentMessage = {
        id: crypto.randomUUID(),
        type: "user",
        content: displayInstruction ?? instruction,
        attachments: attachments?.length ? attachments : undefined,
        timestamp: new Date(),
      };
      setMessages((prev) => [...prev, userMsg]);

      const agentMsgId = crypto.randomUUID();
      const initialAgentMsg: AgentMessage = {
        id: agentMsgId,
        type: "agent",
        content: "",
        isLoading: true,
        timestamp: new Date(),
        modelUsed: { name: modelId, thinkingEnabled: true },
        neuralState: "thinking",
        runPhase: "starting",
        statusText: "Starting agent workflow",
        runState: "running",
        requestContext: {
          runId: clientRunId,
          prompt: instruction,
          modelId,
          reasoningEffort,
          workspaceId,
          agentSpecId,
          chatScopeId: resolvedChatScopeId,
          startedAtMs: Date.now(),
        },
        trace: [
          createTraceEntry("think", "Task received. Starting agent workflow."),
        ],
        ragTelemetry: { ...defaultRagTelemetry },
      };
      setMessages((prev) => [...prev, initialAgentMsg]);

      setIsExecuting(true);

      // Listen to real-time agent events from Rust backend
      let unlisten: (() => void) | null = null;
      let queuedRuntimeEvents: RuntimeAgentEvent[] = [];
      let runtimeEventFrame: number | null = null;
      const toolCallIndex = new Map<string, { name: string; arguments?: string }>();
      let applyRuntimeEventToMessage:
        | ((message: AgentMessage, payload: RuntimeAgentEvent) => AgentMessage)
        | null = null;
      try {
        void captureForgeStep("agent_run_requested", "run_agent_workflow", {
          workspaceId,
          modelId,
          reasoningEffort: reasoningEffort || null,
          agentSpecId: agentSpecId || null,
        });
        const upsertSpecialist = (
          specialists: SpecialistRunState[] | undefined,
          next: Partial<SpecialistRunState> & {
            agentId: string;
            role: SpecialistRunState["role"];
            status: SpecialistRunState["status"];
          },
        ): SpecialistRunState[] => {
          const current = specialists ? [...specialists] : [];
          const idx = current.findIndex((item) => item.agentId === next.agentId);
          const merged = mergeDefinedFields(
            idx >= 0 ? current[idx] : ({} as SpecialistRunState),
            next,
          ) as SpecialistRunState;
          if (idx >= 0) {
            current[idx] = merged;
          } else {
            current.push(merged);
          }
          return current;
        };

        const mergeDefinedFields = <T,>(
          current: T,
          next: Partial<T>,
        ): T => {
          const merged = { ...(current as Record<string, unknown>) };
          for (const [key, value] of Object.entries(
            next as Record<string, unknown>,
          )) {
            if (value !== undefined) {
              merged[key] = value;
            }
          }
          return merged as T;
        };

        applyRuntimeEventToMessage = (
          message: AgentMessage,
          payload: RuntimeAgentEvent,
        ): AgentMessage => {
          switch (payload.type) {
            case "supervisor_plan_created": {
              void captureForgeStep(
                "decision",
                payload.data?.summary || "supervisor_plan_created",
                {
                  steps: Array.isArray(payload.data?.steps)
                    ? payload.data.steps.length
                    : 0,
                },
              );
              return {
                ...message,
                neuralState: "planning",
                runPhase: "planning",
                statusText: payload.data?.summary || "Supervisor plan ready",
                trace: [
                  ...(message.trace || []),
                  createTraceEntry(
                    "act",
                    payload.data?.summary || "Supervisor plan created",
                    undefined,
                    payload.timestampMs,
                  ),
                ],
                supervisorPlan: {
                  summary: payload.data?.summary || "Supervisor plan ready",
                  steps: Array.isArray(payload.data?.steps)
                    ? payload.data.steps
                    : [],
                  verificationRequired:
                    payload.data?.verificationRequired || false,
                  mode: payload.data?.mode,
                  delegationPolicy: payload.data?.delegationPolicy,
                  maxDepth: payload.data?.maxDepth,
                  maxThreads: payload.data?.maxThreads,
                  maxParallelSubagents: payload.data?.maxParallelSubagents,
                  internalCoordinationLanguage:
                    payload.data?.internalCoordinationLanguage,
                  finalResponseLanguageMode:
                    payload.data?.finalResponseLanguageMode,
                },
              };
            }
            case "specialist_spawned":
            case "specialist_status_changed": {
              const activeTool = payload.data?.activeTool || undefined;
              const waitingOnAirlock =
                payload.data?.status === "waiting_on_airlock";
              return {
                ...message,
                neuralState: waitingOnAirlock
                  ? "planning"
                  : activeTool
                    ? resolveNeuralState(activeTool)
                    : "planning",
                runPhase: waitingOnAirlock ? "awaiting_approval" : activeTool ? "tool_running" : "planning",
                statusText: payload.data?.detail || `${payload.data?.role || "specialist"}: ${payload.data?.status || "running"}`,
                trace: [
                  ...(message.trace || []),
                  createTraceEntry(
                    waitingOnAirlock ? "approval" : "act",
                    `${payload.data?.role || "specialist"}: ${payload.data?.status || "running"}`,
                    { toolName: activeTool || undefined },
                    payload.timestampMs,
                  ),
                ],
                activeToolName: waitingOnAirlock
                  ? "Awaiting Airlock approval"
                  : activeTool
                    ? getToolDisplayName(activeTool)
                    : undefined,
                specialists: upsertSpecialist(message.specialists, {
                  agentId: payload.data?.agentId ?? "unknown",
                  role: payload.data?.role ?? "specialist",
                  status: payload.data?.status ?? "planning",
                  detail: payload.data?.detail,
                  activeTool,
                  dependsOn: payload.data?.dependsOn || [],
                  startedAtMs: payload.data?.startedAtMs,
                  finishedAtMs: payload.data?.finishedAtMs,
                  toolCount: payload.data?.toolCount,
                  writeLikeUsed: payload.data?.writeLikeUsed,
                  parentAgentId: payload.data?.parentAgentId,
                  depth: payload.data?.depth,
                  branchId: payload.data?.branchId,
                  spawnReason: payload.data?.spawnReason,
                }),
              };
            }
            case "specialist_completed": {
              return {
                ...message,
                neuralState: "thinking",
                runPhase: "responding",
                trace: [
                  ...(message.trace || []),
                  createTraceEntry(
                    "act",
                    `${payload.data?.role || "specialist"} completed`,
                    {
                      preview: payload.data?.summary || payload.data?.responsePreview,
                    },
                    payload.timestampMs,
                  ),
                ],
                ...clearTransientRunDecorations(),
                specialists: upsertSpecialist(message.specialists, {
                  agentId: payload.data?.agentId ?? "unknown",
                  role: payload.data?.role ?? "specialist",
                  status: "completed",
                  summary: payload.data?.summary,
                  responsePreview: payload.data?.responsePreview,
                  dependsOn: payload.data?.dependsOn || [],
                  startedAtMs: payload.data?.startedAtMs,
                  finishedAtMs: payload.data?.finishedAtMs,
                  toolCount: payload.data?.toolCount,
                  writeLikeUsed: payload.data?.writeLikeUsed,
                  parentAgentId: payload.data?.parentAgentId,
                  depth: payload.data?.depth,
                  branchId: payload.data?.branchId,
                  spawnReason: payload.data?.spawnReason,
                }),
              };
            }
            case "specialist_failed": {
              void captureForgeStep(
                "error",
                payload.data?.error || "specialist_failed",
                {
                  agentId: payload.data?.agentId ?? null,
                  role: payload.data?.role ?? null,
                },
              );
              return {
                ...message,
                neuralState: "thinking",
                runPhase: "responding",
                trace: [
                  ...(message.trace || []),
                  createTraceEntry(
                    "error",
                    `${payload.data?.role || "specialist"} failed`,
                    { preview: payload.data?.error },
                    payload.timestampMs,
                  ),
                ],
                ...clearTransientRunDecorations(),
                specialists: upsertSpecialist(message.specialists, {
                  agentId: payload.data?.agentId ?? "unknown",
                  role: payload.data?.role ?? "specialist",
                  status: "failed",
                  error: payload.data?.error,
                  dependsOn: payload.data?.dependsOn || [],
                  startedAtMs: payload.data?.startedAtMs,
                  finishedAtMs: payload.data?.finishedAtMs,
                  toolCount: payload.data?.toolCount,
                  writeLikeUsed: payload.data?.writeLikeUsed,
                  parentAgentId: payload.data?.parentAgentId,
                  depth: payload.data?.depth,
                  branchId: payload.data?.branchId,
                  spawnReason: payload.data?.spawnReason,
                }),
              };
            }
            case "stream_tool_call": {
              const functionName = payload.data?.name || "";
              const toolCallId =
                payload.data?.id ||
                `stream-tool-${payload.data?.index ?? functionName ?? "pending"}`;
              const state = String(payload.data?.state || "announced");
              if (functionName) {
                toolCallIndex.set(toolCallId, {
                  name: functionName,
                  arguments: payload.data?.arguments,
                });
              }
              return {
                ...message,
                neuralState: functionName ? resolveNeuralState(functionName) : "planning",
                runPhase: "tool_waiting",
                statusText:
                  state === "ready"
                    ? `Queued ${getToolDisplayName(functionName || "tool")}`
                    : `Preparing ${getToolDisplayName(functionName || "tool")}`,
                activeToolName: getToolDisplayName(functionName || "tool"),
                trace: [
                  ...(message.trace || []),
                  createTraceEntry(
                    "think",
                    state === "ready"
                      ? `Tool ready: ${getToolDisplayName(functionName || "tool")}`
                      : `Tool draft: ${getToolDisplayName(functionName || "tool")}`,
                    {
                      toolName: functionName || undefined,
                      preview: payload.data?.arguments?.slice(0, 180),
                    },
                    payload.timestampMs,
                  ),
                ],
              };
            }
            case "usage": {
              return {
                ...message,
                ragTelemetry: {
                  ...message.ragTelemetry,
                  lastModel:
                    payload.data?.model || message.ragTelemetry?.lastModel,
                  promptTokens:
                    (payload.data?.prompt_tokens as number | undefined) ??
                    message.ragTelemetry?.promptTokens ??
                    0,
                  completionTokens:
                    (payload.data?.completion_tokens as number | undefined) ??
                    message.ragTelemetry?.completionTokens ??
                    0,
                  totalTokens:
                    (payload.data?.total_tokens as number | undefined) ??
                    message.ragTelemetry?.totalTokens ??
                    0,
                },
              };
            }
            case "tool_call": {
              const functionName = payload.data?.function?.name || "";
              if (payload.data?.id) {
                toolCallIndex.set(payload.data.id, {
                  name: functionName,
                  arguments: payload.data?.function?.arguments,
                });
              }
              void captureForgeStep("tool_call", functionName || "tool_call", {
                toolCallId: payload.data?.id ?? null,
                arguments: payload.data?.function?.arguments ?? null,
              });
              const incomingLevel: number =
                (payload.data?.airlock_level as number | undefined) ?? 0;
              return {
                ...message,
                neuralState: resolveNeuralState(functionName),
                runPhase: "tool_running",
                statusText: `Executing ${getToolDisplayName(functionName)}`,
                airlockLevel: Math.max(message.airlockLevel ?? 0, incomingLevel),
                trace: [
                  ...(message.trace || []),
                  createTraceEntry(
                    "tool",
                    `Tool call: ${getToolDisplayName(functionName)}`,
                    { toolName: functionName },
                    payload.timestampMs,
                  ),
                ],
                activeToolName: getToolDisplayName(functionName),
              };
            }
            case "tool_result": {
              const toolCall = payload.data?.id
                ? toolCallIndex.get(payload.data.id)
                : undefined;
              const artifacts =
                toolCall && typeof payload.data?.result === "string"
                  ? artifactsFromToolResult(
                      toolCall.name,
                      toolCall.arguments,
                      payload.data.result,
                    )
                  : [];
              const externalSessions =
                toolCall && typeof payload.data?.result === "string"
                  ? extractExternalSessions(payload.data.result)
                  : [];
              const externalSessionSummary =
                toolCall &&
                typeof payload.data?.result === "string" &&
                (toolCall.name === "spawn_external_agent_session" ||
                  toolCall.name === "send_external_agent_message" ||
                  toolCall.name === "wait_external_agent_session" ||
                  toolCall.name === "cancel_external_agent_session" ||
                  toolCall.name === "list_external_agent_sessions")
                  ? summarizeExternalSessionResult(payload.data.result)
                  : null;
              void captureForgeStep(
                "tool_result",
                payload.data?.id || "tool_result",
                {
                  toolCallId: payload.data?.id ?? null,
                  resultPreview: payload.data?.result ?? null,
                },
              );
              return {
                ...applyExternalSessionUpdates(message, externalSessions),
                artifacts: artifacts.length
                  ? artifacts.reduce(
                      (current, nextArtifact) =>
                        appendUniqueArtifact(current, nextArtifact),
                      mergeExternalSessionArtifacts(message.artifacts, externalSessions),
                    )
                  : mergeExternalSessionArtifacts(message.artifacts, externalSessions),
                neuralState: "thinking",
                runPhase: "responding",
                statusText: externalSessionSummary || "Tool execution completed",
                trace: [
                  ...(message.trace || []),
                  createTraceEntry(
                    "tool",
                    externalSessionSummary
                      ? externalSessionSummary
                      : `Tool result: ${payload.data?.id || "completed"}`,
                    {
                      preview: externalSessionSummary
                        ? externalSessionSummary
                        : typeof payload.data?.result === "string"
                          ? payload.data.result.slice(0, 180)
                          : undefined,
                    },
                    payload.timestampMs,
                  ),
                ],
                ...clearTransientRunDecorations(),
              };
            }
            case "thought":
            case "stream_chunk":
            case "supervisor_summary":
              return {
                ...message,
                content: appendStreamText(message.content, payload.data?.text),
                neuralState: "thinking",
                runPhase:
                  message.trace?.some((entry) => entry.phase === "tool") ||
                  message.activeToolName
                    ? "responding"
                    : "streaming",
                statusText:
                  payload.type === "supervisor_summary"
                    ? payload.data?.summary || message.statusText
                    : "Streaming response",
                activeToolName:
                  payload.type === "stream_chunk" ? message.activeToolName : undefined,
              };
            case "status": {
              const statusText = String(payload.data || "");
              const lower = statusText.toLowerCase();
              if (lower.includes("retry")) {
                void captureForgeStep("retry", statusText, {});
              } else if (
                lower.includes("error") ||
                lower.includes("failed") ||
                lower.includes("exception")
              ) {
                void captureForgeStep("error", statusText, {});
              }
              if (statusText.startsWith("RAG_TELEMETRY:")) {
                try {
                  const raw = statusText.slice("RAG_TELEMETRY:".length);
                  const parsed = JSON.parse(raw) as {
                    history_source?: string;
                    retrieval_mode?: string;
                    embedding_profile?: string;
                  };
                  const nextTelemetry = {
                    historySource:
                      parsed.history_source ||
                      message.ragTelemetry?.historySource ||
                      defaultRagTelemetry.historySource,
                    retrievalMode:
                      parsed.retrieval_mode ||
                      message.ragTelemetry?.retrievalMode ||
                      defaultRagTelemetry.retrievalMode,
                    embeddingProfile:
                      parsed.embedding_profile ||
                      message.ragTelemetry?.embeddingProfile ||
                      defaultRagTelemetry.embeddingProfile,
                    executionMode:
                      message.ragTelemetry?.executionMode ||
                      defaultRagTelemetry.executionMode,
                    workspaceMemoryEnabled:
                      message.ragTelemetry?.workspaceMemoryEnabled ??
                      defaultRagTelemetry.workspaceMemoryEnabled,
                    workspaceMemoryRoot: message.ragTelemetry?.workspaceMemoryRoot,
                    lastModel: message.ragTelemetry?.lastModel,
                    promptTokens:
                      message.ragTelemetry?.promptTokens ??
                      defaultRagTelemetry.promptTokens,
                    completionTokens:
                      message.ragTelemetry?.completionTokens ??
                      defaultRagTelemetry.completionTokens,
                    totalTokens:
                      message.ragTelemetry?.totalTokens ??
                      defaultRagTelemetry.totalTokens,
                    compressionApplied: message.ragTelemetry?.compressionApplied,
                    compressionTriggerTokens:
                      message.ragTelemetry?.compressionTriggerTokens,
                  };
                  if (
                    message.ragTelemetry?.historySource ===
                      nextTelemetry.historySource &&
                    message.ragTelemetry?.retrievalMode ===
                      nextTelemetry.retrievalMode &&
                    message.ragTelemetry?.embeddingProfile ===
                      nextTelemetry.embeddingProfile
                  ) {
                    return message;
                  }
                  return {
                    ...message,
                    ragTelemetry: nextTelemetry,
                  };
                } catch {
                  return message;
                }
              }
              if (statusText.startsWith("RUN_USAGE:")) {
                try {
                  const raw = statusText.slice("RUN_USAGE:".length);
                  const parsed = JSON.parse(raw) as {
                    model?: string;
                    prompt_tokens?: number;
                    completion_tokens?: number;
                    total_tokens?: number;
                  };
                  return {
                    ...message,
                    ragTelemetry: {
                      ...message.ragTelemetry,
                      lastModel:
                        parsed.model || message.ragTelemetry?.lastModel,
                      promptTokens:
                        parsed.prompt_tokens ??
                        message.ragTelemetry?.promptTokens ??
                        0,
                      completionTokens:
                        parsed.completion_tokens ??
                        message.ragTelemetry?.completionTokens ??
                        0,
                      totalTokens:
                        parsed.total_tokens ?? message.ragTelemetry?.totalTokens ?? 0,
                    },
                  };
                } catch {
                  return message;
                }
              }
              if (statusText.startsWith("CONTEXT_COMPACTION:")) {
                try {
                  const raw = statusText.slice("CONTEXT_COMPACTION:".length);
                  const parsed = JSON.parse(raw) as {
                    applied?: boolean;
                    trigger_tokens?: number;
                  };
                  return {
                    ...message,
                    ragTelemetry: {
                      ...message.ragTelemetry,
                      compressionApplied:
                        parsed.applied ?? message.ragTelemetry?.compressionApplied,
                      compressionTriggerTokens:
                        parsed.trigger_tokens ||
                        message.ragTelemetry?.compressionTriggerTokens,
                    },
                  };
                } catch {
                  return message;
                }
              }
              const waitingOnMcpApproval =
                statusText.toLowerCase().includes("approval") &&
                statusText.toLowerCase().includes("mcp");
              const waitingOnAirlockApproval =
                statusText.toLowerCase().includes("awaiting airlock approval");
              const statusLower = statusText.toLowerCase();
              const isRetry = statusLower.includes("retry");
              const isError =
                statusLower.includes("error") ||
                statusLower.includes("failed") ||
                statusLower.includes("exception");
              const isCancelled =
                statusLower.includes("execution cancelled") ||
                statusLower.includes("terminated by fleet kill switch") ||
                statusLower.includes("terminated by user") ||
                statusLower.includes("cancelled by user");
              const tracePhase: AgentTraceEntry["phase"] =
                waitingOnMcpApproval || waitingOnAirlockApproval
                  ? "approval"
                  : isRetry
                    ? "retry"
                    : isError
                      ? "error"
                      : isCancelled
                        ? "cancelled"
                        : "think";
              const nextRunPhase = deriveStatusRunPhase(statusText);
              return {
                ...message,
                neuralState: "planning",
                ...(nextRunPhase ? { runPhase: nextRunPhase } : {}),
                statusText,
                runState: isCancelled ? "cancelled" : message.runState,
                trace: [
                  ...(message.trace || []),
                  createTraceEntry(tracePhase, statusText, undefined, payload.timestampMs),
                ],
                activeToolName: waitingOnMcpApproval
                  ? "Awaiting MCP approval"
                  : waitingOnAirlockApproval
                    ? "Awaiting Airlock approval"
                    : undefined,
              };
            }
            default:
              return message;
          }
        };

        const flushRuntimeEvents = () => {
          runtimeEventFrame = null;
          if (!queuedRuntimeEvents.length) return;
          if (!applyRuntimeEventToMessage) return;
          const applyQueuedRuntimeEvent = applyRuntimeEventToMessage;
          const batch = queuedRuntimeEvents;
          queuedRuntimeEvents = [];
          setMessages((prev) =>
            updateMessageById(prev, agentMsgId, (message) =>
              batch.reduce(
                (current, payload) => applyQueuedRuntimeEvent(current, payload),
                message,
              ),
            ),
          );
        };

        const scheduleRuntimeEvent = (payload: RuntimeAgentEvent) => {
          queuedRuntimeEvents.push(payload);
          if (runtimeEventFrame != null) return;
          runtimeEventFrame = window.requestAnimationFrame(flushRuntimeEvents);
        };

        unlisten = await listen<RuntimeAgentEvent>("agent://event", (event) => {
          const payload = event.payload;
          if (payload?.runId && payload.runId !== clientRunId) return;
          scheduleRuntimeEvent(payload);
        });

        const result = await runAgentWorkflow(
          instruction,
          modelId,
          workspaceId,
          agentSpecId,
          resolvedChatScopeId,
          clientRunId,
          reasoningEffort,
          attachments?.length
            ? attachments.map((a) => ({ path: a.path, name: a.filename }))
            : undefined,
        );

        setMessages((prev) =>
          updateMessageById(prev, agentMsgId, (message) => ({
            ...message,
            content:
              typeof result.response === "string" && result.response.trim().length > 0
                ? result.response
                : "No final text response was generated. Please try again with a more specific instruction.",
            isLoading: false,
            runState: message.runState === "cancelled" ? "cancelled" : "completed",
            requestContext: {
              ...message.requestContext,
              runId: result.runId || clientRunId,
              completedAtMs: Date.now(),
            },
            trace: [
              ...(message.trace || []),
              createTraceEntry(
                message.runState === "cancelled" ? "cancelled" : "done",
                message.runState === "cancelled"
                  ? "Run cancelled."
                  : "Run completed.",
              ),
            ],
            neuralState: undefined,
            runPhase: undefined,
            ...clearTransientRunDecorations(),
            specialists: message.specialists,
            supervisorPlan: message.supervisorPlan,
          })),
        );

        if (typeof result.response === "string" && result.response.trim().length > 0) {
          setChatTitleStatus("generating");
          try {
            const titleResult = await tauri.ensureChatTitle({
              chatScopeId: resolvedChatScopeId,
              prompt: instruction,
              response: result.response,
            });
            setChatSession(titleResult.chat);
            setChatTitleStatus(
              titleResult.status === "fallback" ? "fallback" : "ready",
            );
            await onSessionChanged?.();
          } catch (titleError) {
            console.error("Failed to ensure chat title:", titleError);
            const refreshed = await refreshChatSession(resolvedChatScopeId);
            setChatTitleStatus(refreshed?.title?.trim() ? "ready" : "fallback");
          }
        }
        return {
          ok: true,
          chatScopeId: resolvedChatScopeId,
          runId: result.runId || clientRunId,
          actualToolIds: result.actualToolIds ?? [],
          actualTouchedPaths: result.actualTouchedPaths ?? [],
          producedArtifactPaths: result.producedArtifactPaths ?? [],
        };
      } catch (err: any) {
        console.error("Native agent error:", err);
        const cancelled = isCancellationError(err);
        setMessages((prev) =>
          updateMessageById(prev, agentMsgId, (message) => ({
            ...message,
            content: cancelled
              ? "Execution cancelled."
              : `❌ Agent Runtime Error: ${err.message || err}`,
            isLoading: false,
            runState: cancelled ? "cancelled" : "failed",
            requestContext: {
              ...message.requestContext,
              completedAtMs: Date.now(),
            },
            trace: [
              ...(message.trace || []),
              createTraceEntry(
                cancelled ? "cancelled" : "error",
                cancelled ? "Execution cancelled." : "Agent runtime failed.",
                {
                  preview: cancelled ? undefined : String(err?.message || err),
                },
              ),
            ],
            neuralState: undefined,
            runPhase: undefined,
            ...clearTransientRunDecorations(),
            specialists: message.specialists,
            supervisorPlan: message.supervisorPlan,
          })),
        );
        return {
          ok: false,
          chatScopeId: resolvedChatScopeId,
          error: cancelled ? "Execution cancelled." : String(err?.message || err),
          actualToolIds: [],
          actualTouchedPaths: [],
          producedArtifactPaths: [],
        };
      } finally {
        forgeRecordingIdRef.current = null;
        setIsExecuting(false);
        if (runtimeEventFrame != null) {
          window.cancelAnimationFrame(runtimeEventFrame);
          runtimeEventFrame = null;
        }
        if (queuedRuntimeEvents.length > 0) {
          const batch = queuedRuntimeEvents;
          queuedRuntimeEvents = [];
          setMessages((prev) =>
            updateMessageById(prev, agentMsgId, (message) =>
              batch.reduce(
                (current, payload) =>
                  applyRuntimeEventToMessage
                    ? applyRuntimeEventToMessage(current, payload)
                    : current,
                message,
              ),
            ),
          );
        }
        if (unlisten) unlisten();
      }
    },
    [captureForgeStep, createTraceEntry, ensureChatScope, onSessionChanged, refreshChatSession],
  );

  const stopAgentRun = useCallback(async (messageId: string) => {
    const target = messagesRef.current.find((m) => m.id === messageId);
    const runId = target?.requestContext?.runId;
    if (!runId) return;
    try {
      const res = await tauri.cancelAgentRun(runId);
      setMessages((prev) =>
        updateMessageById(prev, messageId, (message) => ({
          ...message,
          isLoading: res.status === "cancelled" ? false : message.isLoading,
          neuralState: res.status === "cancelled" ? undefined : message.neuralState,
          runState: res.status === "cancelled" ? "cancelled" : message.runState,
          runPhase: res.status === "cancelled" ? undefined : message.runPhase,
          statusText:
            res.status === "cancelled"
              ? "Cancellation requested by user."
              : message.statusText,
          trace: [
            ...(message.trace || []),
            createTraceEntry(
              res.status === "cancelled" ? "cancelled" : "error",
              res.status === "cancelled"
                ? "Cancellation requested by user."
                : "Run already finished.",
            ),
          ],
        })),
      );
    } catch (error) {
      console.error("Failed to cancel run", error);
    }
  }, [createTraceEntry]);

  const stopAgentRunByRunId = useCallback(async (runId: string) => {
    if (!runId) return;

    const target = messagesRef.current.find((message) => (
      message.type === "agent" && message.requestContext?.runId === runId
    ));

    try {
      const res = await tauri.cancelAgentRun(runId);
      if (!target?.id) {
        return;
      }

      setMessages((prev) =>
        updateMessageById(prev, target.id, (message) => ({
          ...message,
          isLoading: res.status === "cancelled" ? false : message.isLoading,
          neuralState: res.status === "cancelled" ? undefined : message.neuralState,
          runState: res.status === "cancelled" ? "cancelled" : message.runState,
          runPhase: res.status === "cancelled" ? undefined : message.runPhase,
          statusText:
            res.status === "cancelled"
              ? "Cancellation requested by user."
              : message.statusText,
          trace: [
            ...(message.trace || []),
            createTraceEntry(
              res.status === "cancelled" ? "cancelled" : "error",
              res.status === "cancelled"
                ? "Cancellation requested by user."
                : "Run already finished.",
            ),
          ],
        })),
      );
    } catch (error) {
      console.error("Failed to cancel run", error);
    }
  }, [createTraceEntry]);

  const retryAgentRun = useCallback(async (messageId: string) => {
    const target = messagesRef.current.find((m) => m.id === messageId);
    if (!target?.requestContext?.prompt) return;
    await runNativeAgent(
      target.requestContext.prompt,
      target.requestContext.modelId || "rainy:gpt-5",
      target.requestContext.workspaceId || ".",
      target.requestContext.agentSpecId,
      target.requestContext.reasoningEffort,
    );
  }, [runNativeAgent]);

  return {
    messages,
    setMessages,
    chatScopeId,
    chatSession,
    chatTitleStatus,
    activeSessionBinding,
    isPlanning,
    isExecuting,
    currentPlan,
    sendInstruction,
    streamChat,
    executePlan,
    cancelPlan,
    executeDiscussedPlan,
    executeToolCalls,
    clearMessages,
    clearMessagesAndContext,
    refreshActiveChat,
    runNativeAgent,
    stopAgentRun,
    stopAgentRunByRunId,
    retryAgentRun,
    refreshChatSession,
    hydrateLongChatHistory,
    loadOlderHistory,
    hasMoreHistory,
    isHydratingHistory,
    switchChat,
    setChatScopeId,
  };
}
