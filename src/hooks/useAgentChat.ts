import { useState, useCallback, useRef } from "react";
import { useStreaming } from "./useStreaming";
import { useTauriTask } from "./useTauriTask";
import type {
  AgentTraceEntry,
  AgentMessage,
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

type RuntimeAgentEvent = {
  runId?: string;
  timestampMs?: number;
  type: string;
  data: any;
};

export function useAgentChat() {
  const [messages, setMessages] = useState<AgentMessage[]>([]);
  const [isPlanning, setIsPlanning] = useState(false);
  const [isExecuting, setIsExecuting] = useState(false);
  const [currentPlan, setCurrentPlan] = useState<TaskPlan | null>(null);
  const [chatScopeId, setChatScopeId] = useState<string | null>(null);
  const [chatSession, setChatSession] = useState<tauri.ChatSession | null>(null);
  const [chatTitleStatus, setChatTitleStatus] = useState<
    "idle" | "generating" | "ready" | "fallback"
  >("idle");
  const [historyCursorRowid, setHistoryCursorRowid] = useState<number | null>(
    null,
  );
  const [hasMoreHistory, setHasMoreHistory] = useState(false);
  const [isHydratingHistory, setIsHydratingHistory] = useState(false);
  const forgeRecordingIdRef = useRef<string | null>(null);
  const isHydratingRef = useRef(false);
  const hasHydratedRef = useRef(false);

  const { streamWithRouting } = useStreaming();
  const { createTask } = useTauriTask();
  const defaultRagTelemetry = {
    historySource: "persisted_long_chat",
    retrievalMode: "unavailable",
    embeddingProfile: "gemini-embedding-2-preview",
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
    // Check state but don't depend on it in the outer useCallback if possible
    // Actually, we can just use the value from a ref or just call the service.
    const scope = await tauri.getDefaultChatScope();
    setChatScopeId(scope);
    return scope;
  }, []); // No dependency on chatScopeId to avoid recreation loop

  const refreshChatSession = useCallback(
    async (scopeOverride?: string) => {
      try {
        const scope = scopeOverride || (await ensureChatScope());
        const session = await tauri.getChatSession(scope);
        setChatSession(session);
        setChatTitleStatus(session.title?.trim() ? "ready" : "idle");
        return session;
      } catch (error) {
        console.error("Failed to load chat session:", error);
        return null;
      }
    },
    [ensureChatScope],
  );

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
              },
            };
            break;
          }
        }
      }
      setMessages(hydratedMessages);
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
    ensureChatScope,
    mapPersistedRoleToUiType,
    refreshChatSession,
  ]);

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

  // Helper to parse tool calls from content
  const parseToolCalls = useCallback((content: string) => {
    const toolCalls: Array<{
      skill: string;
      method: string;
      params: Record<string, any>;
    }> = [];

    // Pattern: write_file("path", "content")
    const writeFileRegex =
      /write_file\s*\(\s*["']([^"']+)["']\s*,\s*["']?([^)]*?)["']?\s*\)/gi;
    let match;
    while ((match = writeFileRegex.exec(content)) !== null) {
      toolCalls.push({
        skill: "filesystem",
        method: "write_file",
        params: { path: match[1], content: match[2] || "" },
      });
    }

    // Pattern: append_file("path", "content")
    const appendFileRegex =
      /append_file\s*\(\s*["']([^"']+)["']\s*,\s*["']?([^)]*?)["']?\s*\)/gi;
    while ((match = appendFileRegex.exec(content)) !== null) {
      toolCalls.push({
        skill: "filesystem",
        method: "append_file",
        params: { path: match[1], content: match[2] || "" },
      });
    }

    // Pattern: read_file("path")
    const readFileRegex = /read_file\s*\(\s*["']([^"']+)["']\s*\)/gi;
    while ((match = readFileRegex.exec(content)) !== null) {
      toolCalls.push({
        skill: "filesystem",
        method: "read_file",
        params: { path: match[1] },
      });
    }

    // Pattern: list_files("path")
    const listFilesRegex = /list_files\s*\(\s*["']([^"']+)["']\s*\)/gi;
    while ((match = listFilesRegex.exec(content)) !== null) {
      toolCalls.push({
        skill: "filesystem",
        method: "list_files",
        params: { path: match[1] },
      });
    }

    // Pattern: search_files("query", "path")
    const searchFilesRegex =
      /search_files\s*\(\s*["']([^"']+)["']\s*(?:,\s*["']([^"']+)["'])?\s*\)/gi;
    while ((match = searchFilesRegex.exec(content)) !== null) {
      toolCalls.push({
        skill: "filesystem",
        method: "search_files",
        params: { query: match[1], path: match[2] },
      });
    }

    return toolCalls;
  }, []);

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
              const detectedTools = parseToolCalls(accumulatedContent);

              setMessages((prev) =>
                prev.map((m) =>
                  m.id === agentMsgId
                    ? {
                        ...m,
                        content: accumulatedContent,
                        isLoading: false,
                        toolCalls:
                          detectedTools.length > 0 ? detectedTools : undefined,
                      }
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
    [streamWithRouting, messages, parseToolCalls],
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

      let targetModel = modelId;
      if (targetModel.startsWith("rainy:")) {
        targetModel = targetModel.replace("rainy:", "");
      } else if (targetModel.startsWith("cowork:")) {
        targetModel = targetModel.replace("cowork:", "");
      }

      const targetProvider = "rainyapi";

      try {
        const task = await createTask(
          instruction,
          targetProvider as any,
          targetModel,
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

      setIsExecuting(true);

      const lastAgentMessage = [...messages]
        .reverse()
        .find((m) => m.type === "agent" && !m.isLoading);

      if (!lastAgentMessage) {
        setIsExecuting(false);
        return;
      }

      const toolCalls = parseToolCalls(lastAgentMessage.content);

      if (toolCalls.length === 0) {
        setMessages((prev) => [
          ...prev,
          {
            id: crypto.randomUUID(),
            type: "agent",
            content:
              "❌ Could not find any executable operations in the plan. Please ask the AI to use write_file, read_file, or list_files commands.",
            isLoading: false,
            timestamp: new Date(),
          },
        ]);
        setIsExecuting(false);
        return;
      }

      // Re-use executeToolCalls logic via direct execution since we are in a hook
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
                    content: `⏳ ${call.method}("${call.params.path}")...`,
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

          if (!result.success) {
            throw new Error(
              `Failed: ${call.method}("${call.params.path}"): ${result.error}`,
            );
          }
        }

        const successDetails = results
          .map((r) => `✅ ${r.call.method}("${r.call.params.path}")`)
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
              ? {
                  ...m,
                  content: `❌ ${err.message}`,
                  isLoading: false,
                }
              : m,
          ),
        );
      } finally {
        setIsExecuting(false);
      }
    },
    [messages, parseToolCalls],
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
    ) => {
      const resolvedChatScopeId = await ensureChatScope().catch((error) => {
        console.error("Failed to resolve chat scope, using fallback:", error);
        return "global:long_chat:v1";
      });
      const clientRunId = crypto.randomUUID();
      const userMsg: AgentMessage = {
        id: crypto.randomUUID(),
        type: "user",
        content: instruction,
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
        ragTelemetry: {
          historySource: "persisted_long_chat",
          retrievalMode: "unavailable",
          embeddingProfile: "gemini-embedding-2-preview",
        },
      };
      setMessages((prev) => [...prev, initialAgentMsg]);

      setIsExecuting(true);

      // Listen to real-time agent events from Rust backend
      let unlisten: (() => void) | null = null;
      let queuedRuntimeEvents: RuntimeAgentEvent[] = [];
      let runtimeEventFrame: number | null = null;
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
          const merged = {
            ...(idx >= 0 ? current[idx] : {}),
            ...next,
          } as SpecialistRunState;
          if (idx >= 0) {
            current[idx] = merged;
          } else {
            current.push(merged);
          }
          return current;
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
                  agentId: payload.data?.agentId,
                  role: payload.data?.role,
                  status: payload.data?.status,
                  detail: payload.data?.detail,
                  activeTool,
                  dependsOn: payload.data?.dependsOn || [],
                  startedAtMs: payload.data?.startedAtMs,
                  finishedAtMs: payload.data?.finishedAtMs,
                  toolCount: payload.data?.toolCount,
                  writeLikeUsed: payload.data?.writeLikeUsed,
                }),
              };
            }
            case "specialist_completed": {
              return {
                ...message,
                neuralState: "thinking",
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
                activeToolName: undefined,
                specialists: upsertSpecialist(message.specialists, {
                  agentId: payload.data?.agentId,
                  role: payload.data?.role,
                  status: "completed",
                  summary: payload.data?.summary,
                  responsePreview: payload.data?.responsePreview,
                  dependsOn: payload.data?.dependsOn || [],
                  startedAtMs: payload.data?.startedAtMs,
                  finishedAtMs: payload.data?.finishedAtMs,
                  toolCount: payload.data?.toolCount,
                  writeLikeUsed: payload.data?.writeLikeUsed,
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
                trace: [
                  ...(message.trace || []),
                  createTraceEntry(
                    "error",
                    `${payload.data?.role || "specialist"} failed`,
                    { preview: payload.data?.error },
                    payload.timestampMs,
                  ),
                ],
                activeToolName: undefined,
                specialists: upsertSpecialist(message.specialists, {
                  agentId: payload.data?.agentId,
                  role: payload.data?.role,
                  status: "failed",
                  error: payload.data?.error,
                  dependsOn: payload.data?.dependsOn || [],
                  startedAtMs: payload.data?.startedAtMs,
                  finishedAtMs: payload.data?.finishedAtMs,
                  toolCount: payload.data?.toolCount,
                  writeLikeUsed: payload.data?.writeLikeUsed,
                }),
              };
            }
            case "tool_call": {
              const functionName = payload.data?.function?.name || "";
              void captureForgeStep("tool_call", functionName || "tool_call", {
                toolCallId: payload.data?.id ?? null,
                arguments: payload.data?.function?.arguments ?? null,
              });
              return {
                ...message,
                neuralState: resolveNeuralState(functionName),
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
              void captureForgeStep(
                "tool_result",
                payload.data?.id || "tool_result",
                {
                  toolCallId: payload.data?.id ?? null,
                  resultPreview: payload.data?.result ?? null,
                },
              );
              return {
                ...message,
                neuralState: "thinking",
                trace: [
                  ...(message.trace || []),
                  createTraceEntry(
                    "tool",
                    `Tool result: ${payload.data?.id || "completed"}`,
                    {
                      preview:
                        typeof payload.data?.result === "string"
                          ? payload.data.result.slice(0, 180)
                          : undefined,
                    },
                    payload.timestampMs,
                  ),
                ],
                activeToolName: undefined,
              };
            }
            case "thought":
            case "stream_chunk":
            case "supervisor_summary":
              return {
                ...message,
                neuralState: "thinking",
                activeToolName: undefined,
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
              return {
                ...message,
                neuralState: "planning",
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

        const { listen } = await import("@tauri-apps/api/event");
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
            activeToolName: undefined,
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
          } catch (titleError) {
            console.error("Failed to ensure chat title:", titleError);
            const refreshed = await refreshChatSession(resolvedChatScopeId);
            setChatTitleStatus(refreshed?.title?.trim() ? "ready" : "fallback");
          }
        }
      } catch (err: any) {
        console.error("Native agent error:", err);
        setMessages((prev) =>
          updateMessageById(prev, agentMsgId, (message) => ({
            ...message,
            content: `❌ Agent Runtime Error: ${err.message || err}`,
            isLoading: false,
            runState: "failed",
            requestContext: {
              ...message.requestContext,
              completedAtMs: Date.now(),
            },
            trace: [
              ...(message.trace || []),
              createTraceEntry(
                "error",
                "Agent runtime failed.",
                { preview: String(err?.message || err) },
              ),
            ],
            neuralState: undefined,
            activeToolName: undefined,
            specialists: message.specialists,
            supervisorPlan: message.supervisorPlan,
          })),
        );
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
    [captureForgeStep, createTraceEntry, ensureChatScope, refreshChatSession],
  );

  const stopAgentRun = useCallback(async (messageId: string) => {
    const target = messages.find((m) => m.id === messageId);
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
  }, [createTraceEntry, messages]);

  const retryAgentRun = useCallback(async (messageId: string) => {
    const target = messages.find((m) => m.id === messageId);
    if (!target?.requestContext?.prompt) return;
    await runNativeAgent(
      target.requestContext.prompt,
      target.requestContext.modelId || "rainy:gpt-5",
      target.requestContext.workspaceId || ".",
      target.requestContext.agentSpecId,
      target.requestContext.reasoningEffort,
    );
  }, [messages, runNativeAgent]);

  return {
    messages,
    setMessages,
    chatScopeId,
    chatSession,
    chatTitleStatus,
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
    runNativeAgent,
    stopAgentRun,
    retryAgentRun,
    refreshChatSession,
    hydrateLongChatHistory,
    loadOlderHistory,
    hasMoreHistory,
    isHydratingHistory,
  };
}
