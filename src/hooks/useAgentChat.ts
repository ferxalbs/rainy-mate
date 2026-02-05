// src/hooks/useAgentChat.ts
import { useState, useCallback } from "react";
import { useStreaming } from "./useStreaming";
import { useTauriTask } from "./useTauriTask";
import type { AgentMessage, TaskPlan } from "../types/agent";
import * as tauri from "../services/tauri";

export function useAgentChat() {
  const [messages, setMessages] = useState<AgentMessage[]>([]);
  const [isPlanning, setIsPlanning] = useState(false);
  const [isExecuting, setIsExecuting] = useState(false);
  const [currentPlan, setCurrentPlan] = useState<TaskPlan | null>(null);

  const { streamWithRouting } = useStreaming();
  const { createTask } = useTauriTask();

  const clearMessages = useCallback(() => {
    setMessages([]);
    setCurrentPlan(null); // Reset plan too
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
        // Filter out temporary/loading states or failed messages if needed
        const history = messages
          .filter((m) => !m.isLoading && !m.content.startsWith("[Error"))
          .map((m) => ({
            role: m.type === "agent" ? "assistant" : "user",
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
    [streamWithRouting, messages], // Add messages to dependency array
  );

  const sendInstruction = useCallback(
    async (instruction: string, workspacePath: string, modelId: string) => {
      // This maps to "Deep Processing" / Task creation
      const userMsg: AgentMessage = {
        id: crypto.randomUUID(),
        type: "user",
        content: instruction,
        timestamp: new Date(),
      };
      setMessages((prev) => [...prev, userMsg]);

      setIsPlanning(true);

      // Parse modelId (e.g. "gemini-2.0-flash" -> "gemini-2.0-flash")
      // Parse modelId - remove explicit 'rainy:' prefix handling if backend handles it
      // But keep safety check if ID comes formatted weirdly from other places
      let targetModel = modelId;
      if (targetModel.startsWith("rainy:")) {
        targetModel = targetModel.replace("rainy:", "");
      } else if (targetModel.startsWith("cowork:")) {
        targetModel = targetModel.replace("cowork:", "");
      }

      const targetProvider = "rainyapi"; // Default to rainyapi

      // created a task
      try {
        // Use selected model instead of hardcoded default
        const task = await createTask(
          instruction,
          targetProvider as any,
          targetModel,
          workspacePath,
        );

        // Create a placeholder plan message
        const planMsgId = crypto.randomUUID();
        const planMsg: AgentMessage = {
          id: planMsgId,
          type: "agent",
          content: "Planning task...",
          isLoading: true,
          timestamp: new Date(),
        };
        setMessages((prev) => [...prev, planMsg]);

        // Execute the task
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
                    } // Mock result
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

  const executePlan = useCallback(async (_planId: string) => {
    // Legacy stub
  }, []);

  const cancelPlan = useCallback(async (_planId: string) => {
    // Legacy stub
  }, []);

  const executeDiscussedPlan = useCallback(
    async (workspaceId: string, modelId: string) => {
      if (messages.length === 0) return;

      setIsExecuting(true);

      // 1. Create a "Thinking..." message
      const thinkingMsgId = crypto.randomUUID();
      setMessages((prev) => [
        ...prev,
        {
          id: thinkingMsgId,
          type: "agent",
          content: "Generating execution plan from our discussion...",
          isLoading: true,
          timestamp: new Date(),
        },
      ]);

      try {
        // 2. Prepare history for context
        const history = messages
          .filter((m) => !m.isLoading && !m.content.startsWith("[Error"))
          .map((m) => ({
            role: m.type === "agent" ? "assistant" : "user",
            content: m.content,
          }));

        // 3. Ask AI to convert plan to JSON tool calls
        const response = await tauri.completeChat({
          messages: [
            ...history,
            {
              role: "system",
              content: `Convert the proposed plan from the conversation into executable JSON tool calls.

AVAILABLE TOOLS (use these exact names):
- write_file: Creates or overwrites a file. Params: { "path": "<filepath>", "content": "<file content>" }
- read_file: Reads a file. Params: { "path": "<filepath>" }
- list_files: Lists directory contents. Params: { "path": "<directory path>" }
- search_files: Searches for text. Params: { "query": "<search term>", "path": "<optional directory>" }

OUTPUT RULES:
1. Output ONLY a valid JSON array - no markdown, no explanation
2. If the plan mentions creating a file, use write_file with appropriate content (not empty)
3. If no file content was specified, generate reasonable default content
4. Use relative paths from the project root

EXAMPLE OUTPUT:
[{"skill":"filesystem","method":"write_file","params":{"path":"test.txt","content":"Hello World"}}]

Generate the JSON array now:`,
            },
          ],
          model: modelId,
          stream: false,
        });

        const content = response.content || "[]";
        console.log("[executeDiscussedPlan] AI response content:", content);
        let toolCalls: any[] = [];

        try {
          // clean markdown if present
          const cleanContent = content
            .replace(/```json/g, "")
            .replace(/```/g, "")
            .trim();
          console.log("[executeDiscussedPlan] Cleaned content:", cleanContent);
          toolCalls = JSON.parse(cleanContent);
          console.log("[executeDiscussedPlan] Parsed tool calls:", toolCalls);
        } catch (e) {
          console.error("[executeDiscussedPlan] JSON parse error:", e);
          throw new Error("Failed to parse execution plan: " + content);
        }

        // 4. Update UI to "Executing..."
        setMessages((prev) =>
          prev.map((m) =>
            m.id === thinkingMsgId
              ? {
                  ...m,
                  content: `Executing ${toolCalls.length} operations...`,
                  isLoading: true,
                }
              : m,
          ),
        );

        // 5. Execute each tool
        const results: { call: any; result: tauri.CommandResult }[] = [];
        console.log(
          "[executeDiscussedPlan] Starting execution of",
          toolCalls.length,
          "tool calls",
        );

        for (const call of toolCalls) {
          console.log("[executeDiscussedPlan] Processing call:", call);

          if (call.skill === "filesystem") {
            setMessages((prev) =>
              prev.map((m) =>
                m.id === thinkingMsgId
                  ? {
                      ...m,
                      content: `Executing: ${call.method} ${call.params?.path || ""}...`,
                    }
                  : m,
              ),
            );

            console.log("[executeDiscussedPlan] Calling executeSkill:", {
              workspaceId,
              skill: call.skill,
              method: call.method,
              params: call.params,
            });

            const result = await tauri.executeSkill(
              workspaceId,
              call.skill,
              call.method,
              call.params || {},
            );
            console.log("[executeDiscussedPlan] executeSkill result:", result);
            results.push({ call, result });

            if (!result.success) {
              throw new Error(
                `Failed to execute ${call.method}: ${result.error}`,
              );
            }
          } else {
            console.log(
              "[executeDiscussedPlan] Skipping non-filesystem call:",
              call,
            );
          }
        }

        // 6. Final success message
        setMessages((prev) =>
          prev.map((m) =>
            m.id === thinkingMsgId
              ? {
                  ...m,
                  content:
                    `✅ Successfully executed ${toolCalls.length} operations.\n\n` +
                    results
                      .map(
                        (r) =>
                          `- ${r.call.method} ${r.call.params.path}: ${r.result.success ? "Success" : "Failed"}`,
                      )
                      .join("\n"),
                  isLoading: false,
                }
              : m,
          ),
        );
      } catch (err: any) {
        setMessages((prev) =>
          prev.map((m) =>
            m.id === thinkingMsgId
              ? {
                  ...m,
                  content: `Execution failed: ${err.message}`,
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

  return {
    messages,
    isPlanning,
    isExecuting,
    currentPlan,
    sendInstruction,
    streamChat,
    executePlan,
    cancelPlan,
    executeDiscussedPlan,
    clearMessages,
  };
}
