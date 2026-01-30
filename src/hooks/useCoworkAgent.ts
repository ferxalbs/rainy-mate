// Rainy Cowork - useCoworkAgent Hook
// React hook for AI agent operations

import { useState, useCallback } from "react";
import * as tauri from "../services/tauri";
import type {
  TaskPlan,
  ExecutionResult,
  AgentEvent,
  WorkspaceAnalysis,
} from "../services/tauri";

export interface AgentMessage {
  id: string;
  type: "user" | "agent" | "system";
  content: string;
  timestamp: Date;
  plan?: TaskPlan;
  result?: ExecutionResult;
  isLoading?: boolean;
  thought?: string; // AI reasoning/thinking content (for models with thinking capabilities)
  thinkingLevel?: "minimal" | "low" | "medium" | "high"; // Level of thinking depth
  modelUsed?: {
    name: string;
    provider: string;
    thinkingEnabled?: boolean;
  };
}

export interface UseCoworkAgentReturn {
  messages: AgentMessage[];
  isPlanning: boolean;
  isExecuting: boolean;
  currentPlan: TaskPlan | null;
  analysis: WorkspaceAnalysis | null;

  // Actions
  sendInstruction: (
    instruction: string,
    workspacePath: string,
  ) => Promise<void>;
  streamChat: (message: string, modelId: string) => Promise<void>;
  executePlan: (planId: string) => Promise<void>;
  cancelPlan: (planId: string) => Promise<void>;
  analyzeWorkspace: (path: string) => Promise<void>;
  clearMessages: () => void;
}

export function useCoworkAgent(): UseCoworkAgentReturn {
  const [messages, setMessages] = useState<AgentMessage[]>([]);
  const [isPlanning, setIsPlanning] = useState(false);
  const [isExecuting, setIsExecuting] = useState(false);
  const [currentPlan, setCurrentPlan] = useState<TaskPlan | null>(null);
  const [analysis, setAnalysis] = useState<WorkspaceAnalysis | null>(null);

  const addMessage = useCallback(
    (message: Omit<AgentMessage, "id" | "timestamp">) => {
      const newMessage: AgentMessage = {
        ...message,
        id: crypto.randomUUID(),
        timestamp: new Date(),
      };
      setMessages((prev) => [...prev, newMessage]);
      return newMessage.id;
    },
    [],
  );

  const updateMessage = useCallback(
    (id: string, updates: Partial<AgentMessage>) => {
      setMessages((prev) =>
        prev.map((m) => (m.id === id ? { ...m, ...updates } : m)),
      );
    },
    [],
  );

  const sendInstruction = useCallback(
    async (instruction: string, workspacePath: string) => {
      // Add user message
      addMessage({
        type: "user",
        content: instruction,
      });

      // Add agent thinking message
      const thinkingId = addMessage({
        type: "agent",
        content: "Thinking...",
        isLoading: true,
      });

      setIsPlanning(true);

      try {
        const plan = await tauri.planTask(instruction, workspacePath);

        // Build model attribution footer
        const modelFooter = plan.modelUsed
          ? `\n\n_Powered by ${plan.modelUsed.model} via ${plan.modelUsed.provider}_`
          : "";

        // Handle QUESTIONS - show answer directly, no plan
        if (plan.intent === "question" && plan.answer) {
          updateMessage(thinkingId, {
            content: plan.answer + modelFooter,
            isLoading: false,
          });
          // Don't set currentPlan for questions
          setIsPlanning(false);
          return;
        }

        // Handle COMMANDS - show plan with steps
        setCurrentPlan(plan);

        if (plan.steps.length === 0) {
          updateMessage(thinkingId, {
            content: `I understand you want to "${instruction}", but I couldn't find any specific operations to perform. Could you be more specific about what files you'd like me to work with?${modelFooter}`,
            isLoading: false,
          });
        } else {
          const planSummary = plan.steps
            .map((step, i) => `${i + 1}. ${step.description}`)
            .join("\n");

          updateMessage(thinkingId, {
            content: `I've created a plan with ${plan.steps.length} step(s):\n\n${planSummary}${plan.warnings.length > 0 ? `\n\n⚠️ Warnings:\n${plan.warnings.join("\n")}` : ""}${modelFooter}`,
            isLoading: false,
            plan,
          });

          if (plan.requiresConfirmation) {
            addMessage({
              type: "system",
              content:
                "⚠️ This plan includes operations that may modify or delete files. Please review and confirm to proceed.",
            });
          }
        }
      } catch (error) {
        updateMessage(thinkingId, {
          content: `Sorry, I encountered an error: ${error}`,
          isLoading: false,
        });
      } finally {
        setIsPlanning(false);
      }
    },
    [addMessage, updateMessage],
  );

  const streamChat = useCallback(
    async (message: string, modelId: string) => {
      addMessage({
        type: "user",
        content: message,
      });

      const responseId = addMessage({
        type: "agent",
        content: "",
        isLoading: true,
      });

      let accumulatedContent = "";

      try {
        // Use default model if empty
        const targetModel = modelId || "rainy:gemini-2.0-flash";

        await tauri.streamUnifiedChat(message, targetModel, (event) => {
          switch (event.event) {
            case "token":
              // Backend sends full string for now
              accumulatedContent = event.data;
              updateMessage(responseId, {
                content: accumulatedContent,
                isLoading: true,
              });
              break;
            case "done":
              updateMessage(responseId, {
                isLoading: false,
              });
              break;
            case "error":
              updateMessage(responseId, {
                content: `Error: ${event.data}`,
                isLoading: false,
              });
              break;
          }
        });
      } catch (error) {
        updateMessage(responseId, {
          content: `Connection error: ${error}`,
          isLoading: false,
        });
      }
    },
    [addMessage, updateMessage],
  );

  const executePlan = useCallback(
    async (planId: string) => {
      const executingId = addMessage({
        type: "agent",
        content: "Executing plan...",
        isLoading: true,
      });

      setIsExecuting(true);

      try {
        const result = await tauri.executeAgentTask(
          planId,
          (event: AgentEvent) => {
            // Handle real-time events
            switch (event.event) {
              case "stepStarted":
                updateMessage(executingId, {
                  content: `Step ${event.data.stepIndex + 1}: ${event.data.description}...`,
                  isLoading: true,
                });
                break;
              case "progress":
                updateMessage(executingId, {
                  content: `${event.data.message} (${event.data.progress}%)`,
                  isLoading: true,
                });
                break;
              case "stepCompleted":
                // Could add individual step completion messages
                break;
              case "stepFailed":
                addMessage({
                  type: "system",
                  content: `⚠️ Step ${event.data.stepIndex + 1} failed: ${event.data.error}`,
                });
                break;
            }
          },
        );

        setCurrentPlan(null);

        if (result.success) {
          updateMessage(executingId, {
            content: `✅ Completed! Made ${result.totalChanges} change(s) in ${result.durationMs}ms.`,
            isLoading: false,
            result,
          });
        } else {
          updateMessage(executingId, {
            content: `⚠️ Completed with errors:\n${result.errors.join("\n")}`,
            isLoading: false,
            result,
          });
        }
      } catch (error) {
        updateMessage(executingId, {
          content: `Failed to execute: ${error}`,
          isLoading: false,
        });
      } finally {
        setIsExecuting(false);
      }
    },
    [addMessage, updateMessage],
  );

  const cancelPlan = useCallback(
    async (planId: string) => {
      try {
        await tauri.cancelAgentPlan(planId);
        setCurrentPlan(null);
        addMessage({
          type: "system",
          content: "Plan cancelled.",
        });
      } catch (error) {
        addMessage({
          type: "system",
          content: `Failed to cancel: ${error}`,
        });
      }
    },
    [addMessage],
  );

  const analyzeWorkspace = useCallback(
    async (path: string) => {
      const analysisId = addMessage({
        type: "agent",
        content: "Analyzing workspace...",
        isLoading: true,
      });

      try {
        const result = await tauri.agentAnalyzeWorkspace(path);
        setAnalysis(result);

        const summary = [
          `📁 **${result.totalFiles}** files in **${result.totalFolders}** folders`,
          `💾 Total size: **${formatBytes(result.totalSizeBytes)}**`,
          "",
          "File types:",
          ...Object.entries(result.fileTypes).map(
            ([type, stats]) =>
              `  • ${type}: ${stats.count} files (${formatBytes(stats.totalSize)})`,
          ),
        ];

        if (result.suggestions.length > 0) {
          summary.push("", "💡 Suggestions:");
          result.suggestions.forEach((s, i) => {
            summary.push(`  ${i + 1}. ${s.description}`);
          });
        }

        updateMessage(analysisId, {
          content: summary.join("\n"),
          isLoading: false,
        });
      } catch (error) {
        updateMessage(analysisId, {
          content: `Analysis failed: ${error}`,
          isLoading: false,
        });
      }
    },
    [addMessage, updateMessage],
  );

  const clearMessages = useCallback(() => {
    setMessages([]);
    setCurrentPlan(null);
    setAnalysis(null);
  }, []);

  return {
    messages,
    isPlanning,
    isExecuting,
    currentPlan,
    analysis,
    sendInstruction,
    streamChat,
    executePlan,
    cancelPlan,
    analyzeWorkspace,
    clearMessages,
  };
}

// Helper function to format bytes
function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
}
