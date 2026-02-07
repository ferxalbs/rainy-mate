import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { toast } from "sonner"; // Use sonner!

export type AgentEvent =
  | { type: "status"; data: string }
  | { type: "thought"; data: string }
  | { type: "tool_call"; data: any }
  | { type: "tool_result"; data: { id: string; result: string } }
  | { type: "error"; data: string };

export type AgentStatus = "idle" | "running" | "error" | "completed";

export interface AgentState {
  status: AgentStatus;
  thoughts: string[];
  toolCalls: any[];
  error: string | null;
}

export function useAgentRuntime() {
  const [state, setState] = useState<AgentState>({
    status: "idle",
    thoughts: [],
    toolCalls: [],
    error: null,
  });

  useEffect(() => {
    const unlistenPromise = listen<AgentEvent>("agent://event", (event) => {
      const payload = event.payload;
      console.log("Agent Event:", payload);

      setState((prev) => {
        const newState = { ...prev };

        switch (payload.type) {
          case "status":
            // Optional: update status text if needed, but "status" field is enum
            break;
          case "thought":
            newState.thoughts = [...prev.thoughts, payload.data];
            break;
          case "tool_call":
            newState.toolCalls = [
              ...prev.toolCalls,
              { ...payload.data, status: "pending" },
            ];
            break;
          case "tool_result":
            newState.toolCalls = prev.toolCalls.map((tc) =>
              tc.id === payload.data.id
                ? { ...tc, status: "completed", result: payload.data.result }
                : tc,
            );
            break;
          case "error":
            newState.error = payload.data;
            newState.status = "error";
            break;
        }
        return newState;
      });
    });

    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  const runAgent = useCallback(
    async (prompt: string, modelId: string, workspaceId: string) => {
      setState({
        status: "running",
        thoughts: [],
        toolCalls: [],
        error: null,
      });

      try {
        const response = await invoke<string>("run_agent_workflow", {
          prompt,
          modelId,
          workspaceId,
        });

        setState((prev) => ({ ...prev, status: "completed" }));
        return response;
      } catch (err: any) {
        console.error("Agent execution failed:", err);
        setState((prev) => ({
          ...prev,
          status: "error",
          error: err.toString(),
        }));
        toast.error(`Agent failed: ${err.toString()}`);
        throw err;
      }
    },
    [],
  );

  return {
    state,
    runAgent,
  };
}
