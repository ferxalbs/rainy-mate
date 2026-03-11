import { useState, useCallback } from "react";
import { useStreaming } from "./useStreaming";
import { useTauriTask } from "./useTauriTask";
import type {
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

export function useAgentChat() {
  const [messages, setMessages] = useState<AgentMessage[]>([]);
  const [isPlanning, setIsPlanning] = useState(false);
  const [isExecuting, setIsExecuting] = useState(false);
  const [currentPlan, setCurrentPlan] = useState<TaskPlan | null>(null);
  const [chatScopeId, setChatScopeId] = useState<string | null>(null);
  const [historyCursorRowid, setHistoryCursorRowid] = useState<number | null>(
    null,
  );
  const [hasMoreHistory, setHasMoreHistory] = useState(false);
  const [isHydratingHistory, setIsHydratingHistory] = useState(false);
  const [hasHydratedHistory, setHasHydratedHistory] = useState(false);

  const { streamWithRouting } = useStreaming();
  const { createTask } = useTauriTask();
  const defaultRagTelemetry = {
    historySource: "persisted_long_chat",
    retrievalMode: "unavailable",
    embeddingProfile: "gemini-embedding-2-preview",
  } as const;

  const clearMessages = useCallback(() => {
    setMessages([]);
    setCurrentPlan(null);
  }, []);

  const clearMessagesAndContext = useCallback(async (chatId: string) => {
    await tauri.clearChatHistory(chatScopeId || chatId);
    setMessages([]);
    setCurrentPlan(null);
    setHistoryCursorRowid(null);
    setHasMoreHistory(false);
    setHasHydratedHistory(true);
  }, [chatScopeId]);

  const mapPersistedRoleToUiType = useCallback(
    (role: "user" | "assistant" | "system"): AgentMessage["type"] => {
      if (role === "assistant") return "agent";
      if (role === "system") return "system";
      return "user";
    },
    [],
  );

  const ensureChatScope = useCallback(async () => {
    if (chatScopeId) return chatScopeId;
    const scope = await tauri.getDefaultChatScope();
    setChatScopeId(scope);
    return scope;
  }, [chatScopeId]);

  const hydrateLongChatHistory = useCallback(async () => {
    if (isHydratingHistory || hasHydratedHistory) return;
    setIsHydratingHistory(true);
    try {
      const scope = await ensureChatScope();
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
      setHasHydratedHistory(true);
    } catch (error) {
      console.error("Failed to hydrate long chat history:", error);
    } finally {
      setIsHydratingHistory(false);
    }
  }, [
    ensureChatScope,
    hasHydratedHistory,
    isHydratingHistory,
    mapPersistedRoleToUiType,
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
    ) => {
      const resolvedChatScopeId = await ensureChatScope().catch((error) => {
        console.error("Failed to resolve chat scope, using fallback:", error);
        return "global:long_chat:v1";
      });
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
      try {
        const { listen } = await import("@tauri-apps/api/event");
        unlisten = await listen<{
          type: string;
          data: any;
        }>("agent://event", (event) => {
          const payload = event.payload;

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

          setMessages((prev) =>
            prev.map((m) => {
              if (m.id !== agentMsgId) return m;

              switch (payload.type) {
                case "supervisor_plan_created": {
                  return {
                    ...m,
                    neuralState: "planning",
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
                  return {
                    ...m,
                    neuralState: activeTool
                      ? resolveNeuralState(activeTool)
                      : "planning",
                    activeToolName: activeTool
                      ? getToolDisplayName(activeTool)
                      : undefined,
                    specialists: upsertSpecialist(m.specialists, {
                      agentId: payload.data?.agentId,
                      role: payload.data?.role,
                      status: payload.data?.status,
                      detail: payload.data?.detail,
                      activeTool,
                    }),
                  };
                }
                case "specialist_completed": {
                  return {
                    ...m,
                    neuralState: "thinking",
                    activeToolName: undefined,
                    specialists: upsertSpecialist(m.specialists, {
                      agentId: payload.data?.agentId,
                      role: payload.data?.role,
                      status: "completed",
                      summary: payload.data?.summary,
                      responsePreview: payload.data?.responsePreview,
                    }),
                  };
                }
                case "specialist_failed": {
                  return {
                    ...m,
                    neuralState: "thinking",
                    activeToolName: undefined,
                    specialists: upsertSpecialist(m.specialists, {
                      agentId: payload.data?.agentId,
                      role: payload.data?.role,
                      status: "failed",
                      error: payload.data?.error,
                    }),
                  };
                }
                case "tool_call": {
                  // payload.data is a ToolCall: { id, type, function: { name, arguments } }
                  const functionName = payload.data?.function?.name || "";
                  return {
                    ...m,
                    neuralState: resolveNeuralState(functionName),
                    activeToolName: getToolDisplayName(functionName),
                  };
                }
                case "tool_result": {
                  // Tool finished, back to thinking for next iteration
                  return {
                    ...m,
                    neuralState: "thinking",
                    activeToolName: undefined,
                  };
                }
                case "thought":
                case "stream_chunk": {
                  return {
                    ...m,
                    neuralState: "thinking",
                    activeToolName: undefined,
                  };
                }
                case "supervisor_summary": {
                  return {
                    ...m,
                    neuralState: "thinking",
                    activeToolName: undefined,
                  };
                }
                case "status": {
                  const statusText = String(payload.data || "");
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
                          m.ragTelemetry?.historySource ||
                          defaultRagTelemetry.historySource,
                        retrievalMode:
                          parsed.retrieval_mode ||
                          m.ragTelemetry?.retrievalMode ||
                          defaultRagTelemetry.retrievalMode,
                        embeddingProfile:
                          parsed.embedding_profile ||
                          m.ragTelemetry?.embeddingProfile ||
                          defaultRagTelemetry.embeddingProfile,
                        compressionApplied: m.ragTelemetry?.compressionApplied,
                        compressionTriggerTokens:
                          m.ragTelemetry?.compressionTriggerTokens,
                      };
                      if (
                        m.ragTelemetry?.historySource ===
                          nextTelemetry.historySource &&
                        m.ragTelemetry?.retrievalMode ===
                          nextTelemetry.retrievalMode &&
                        m.ragTelemetry?.embeddingProfile ===
                          nextTelemetry.embeddingProfile
                      ) {
                        return m;
                      }
                      return {
                        ...m,
                        ragTelemetry: nextTelemetry,
                      };
                    } catch {
                      return m;
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
                        ...m,
                        ragTelemetry: {
                          ...m.ragTelemetry,
                          compressionApplied:
                            parsed.applied ?? m.ragTelemetry?.compressionApplied,
                          compressionTriggerTokens:
                            parsed.trigger_tokens ||
                            m.ragTelemetry?.compressionTriggerTokens,
                        },
                      };
                    } catch {
                      return m;
                    }
                  }
                  const waitingOnMcpApproval =
                    statusText.toLowerCase().includes("approval") &&
                    statusText.toLowerCase().includes("mcp");
                  return {
                    ...m,
                    neuralState: waitingOnMcpApproval ? "communicating" : "planning",
                    activeToolName: waitingOnMcpApproval
                      ? "Awaiting MCP approval"
                      : undefined,
                  };
                }
                default:
                  return m;
              }
            }),
          );
        });

        const result = await runAgentWorkflow(
          instruction,
          modelId,
          workspaceId,
          agentSpecId,
          resolvedChatScopeId,
        );

        setMessages((prev) =>
          prev.map((m) =>
            m.id === agentMsgId
              ? {
                  ...m,
                  content:
                    typeof result === "string" && result.trim().length > 0
                      ? result
                      : "No final text response was generated. Please try again with a more specific instruction.",
                  isLoading: false,
                  neuralState: undefined,
                  activeToolName: undefined,
                  specialists: m.specialists,
                  supervisorPlan: m.supervisorPlan,
                }
              : m,
          ),
        );
      } catch (err: any) {
        console.error("Native agent error:", err);
        setMessages((prev) =>
          prev.map((m) =>
            m.id === agentMsgId
              ? {
                  ...m,
                  content: `❌ Agent Runtime Error: ${err.message || err}`,
                  isLoading: false,
                  neuralState: undefined,
                  activeToolName: undefined,
                  specialists: m.specialists,
                  supervisorPlan: m.supervisorPlan,
                }
              : m,
          ),
        );
      } finally {
        setIsExecuting(false);
        if (unlisten) unlisten();
      }
    },
    [ensureChatScope],
  );

  return {
    messages,
    setMessages,
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
    hydrateLongChatHistory,
    loadOlderHistory,
    hasMoreHistory,
    isHydratingHistory,
  };
}
