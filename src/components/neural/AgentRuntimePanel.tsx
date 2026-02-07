import { useRef, useEffect } from "react";
import { Card, ScrollShadow, Chip, Spinner, Button } from "@heroui/react";
import {
  Terminal,
  Cpu,
  Play,
  AlertCircle,
  CheckCircle2,
  ChevronRight,
} from "lucide-react";
import { useAgentRuntime } from "../../hooks/useAgentRuntime";
import { AnimatePresence, motion } from "framer-motion";

interface AgentRuntimePanelProps {
  workspaceId: string;
  modelId?: string; // Optional, defaults to settings if not provided
}

export function AgentRuntimePanel({
  workspaceId,
  modelId = "gemini-2.0-flash-exp",
}: AgentRuntimePanelProps) {
  const { state, runAgent } = useAgentRuntime();
  const scrollRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  // Auto-scroll to bottom of logs
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [state.events]);

  const handleRun = async () => {
    if (inputRef.current?.value) {
      await runAgent(inputRef.current.value, modelId, workspaceId);
      inputRef.current.value = "";
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleRun();
    }
  };

  return (
    <div className="flex flex-col h-full bg-background/50 backdrop-blur-md rounded-xl border border-white/5 overflow-hidden">
      {/* Header */}
      <div className="flex items-center justify-between p-4 border-b border-white/5 bg-background/40">
        <div className="flex items-center gap-2">
          <Terminal className="w-5 h-5 text-primary" />
          <h2 className="font-semibold text-foreground">Agent Runtime</h2>
          <Chip
            size="sm"
            variant="flat"
            color={
              state.status === "running"
                ? "warning"
                : state.status === "completed"
                  ? "success"
                  : state.status === "error"
                    ? "danger"
                    : "default"
            }
            className="capitalize"
          >
            {state.status === "running" && (
              <Spinner size="sm" color="warning" className="mr-1" />
            )}
            {state.status}
          </Chip>
        </div>
        <div className="text-xs text-white/40 font-mono flex items-center gap-2">
          <Cpu className="w-4 h-4" />
          {modelId}
        </div>
      </div>

      {/* Execution View (Logs & Thoughts) */}
      <ScrollShadow
        ref={scrollRef}
        className="flex-1 p-4 space-y-4 overflow-y-auto"
      >
        <AnimatePresence mode="popLayout">
          {state.events.map((item) => (
            <motion.div
              key={item.id}
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0 }}
              className="flex flex-col gap-2"
            >
              {item.type === "thought" ? (
                <div className="flex gap-3 text-sm text-foreground/80 bg-white/5 p-3 rounded-lg border border-white/5">
                  <div className="mt-1 min-w-[20px]">
                    <div className="w-2 h-2 rounded-full bg-blue-500/50 mt-1.5" />
                  </div>
                  <div className="whitespace-pre-wrap font-mono">
                    {item.content}
                  </div>
                </div>
              ) : item.type === "status" ? (
                <div className="text-xs text-white/40 font-mono pl-8">
                  {`> ${item.content}`}
                </div>
              ) : item.type === "tool_call" ? (
                <Card className="bg-black/20 border-white/5 p-3 ml-4">
                  <div className="flex items-center justify-between mb-2">
                    <div className="flex items-center gap-2 text-xs font-mono text-warning">
                      <ChevronRight className="w-3 h-3" />
                      TOOL: {item.content.intent}
                    </div>
                    {item.content.status === "pending" ? (
                      <Spinner size="sm" color="warning" />
                    ) : item.content.status === "completed" ? (
                      <CheckCircle2 className="w-4 h-4 text-success" />
                    ) : (
                      <AlertCircle className="w-4 h-4 text-danger" />
                    )}
                  </div>
                  <pre className="text-xs text-white/60 overflow-x-auto bg-black/40 p-2 rounded">
                    {JSON.stringify(item.content.payload, null, 2)}
                  </pre>
                  {item.content.result && (
                    <div className="mt-2 text-xs border-t border-white/5 pt-2">
                      <div className="text-success mb-1">Result:</div>
                      <pre className="text-white/80 whitespace-pre-wrap font-mono max-h-32 overflow-y-auto">
                        {item.content.result}
                      </pre>
                    </div>
                  )}
                </Card>
              ) : (
                // Error
                <div className="p-2 text-xs text-danger font-mono bg-danger/5 border-l-2 border-danger pl-4">
                  Error: {item.content}
                </div>
              )}
            </motion.div>
          ))}

          {state.error && (
            <motion.div
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              className="p-4 rounded-lg bg-danger/10 border border-danger/20 text-danger text-sm flex items-center gap-2"
            >
              <AlertCircle className="w-4 h-4" />
              {state.error}
            </motion.div>
          )}

          {state.events.length === 0 && state.status === "idle" && (
            <div className="h-full flex flex-col items-center justify-center text-white/20 py-10">
              <Terminal className="w-12 h-12 mb-2 opacity-50" />
              <p>Ready to execute tasks...</p>
            </div>
          )}
        </AnimatePresence>
      </ScrollShadow>

      {/* Input Area */}
      <div className="p-4 bg-background/40 border-t border-white/5">
        <div className="relative">
          <input
            ref={inputRef}
            type="text"
            placeholder="Instruct the agent (e.g., 'List files in current directory')..."
            className="w-full bg-black/20 border-white/10 rounded-lg pl-4 pr-12 py-3 text-sm focus:outline-none focus:border-primary/50 transition-colors text-white placeholder-white/30"
            onKeyDown={handleKeyDown}
            disabled={state.status === "running"}
          />
          <Button
            isIconOnly
            size="sm"
            variant="flat"
            color="primary"
            className="absolute right-2 top-2"
            onClick={handleRun}
            isLoading={state.status === "running"}
          >
            {!state.status.startsWith("run") && <Play className="w-4 h-4" />}
          </Button>
        </div>
      </div>
    </div>
  );
}
