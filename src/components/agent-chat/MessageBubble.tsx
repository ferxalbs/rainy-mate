import React, { useMemo } from "react";
import { Play, Ban, FileCode, FolderOpen, ArrowRight } from "lucide-react";
import { Button, Card } from "@heroui/react";
import { motion } from "framer-motion";
import type { AgentMessage, TaskPlan } from "../../types/agent";
import { MarkdownRenderer } from "./MarkdownRenderer";
import { PlanConfirmationCard } from "./PlanConfirmationCard";
import {
  type NeuralState,
  TOOL_STATE_MAP,
  getNeuralStateConfig,
} from "./neural-config";

import { ThoughtDisplay, ThoughtBadge } from "./ThoughtDisplay";

// Map step types to icons
const stepIcons: Record<string, React.ElementType> = {
  createFile: FileCode,
  modifyFile: FileCode,
  deleteFile: TrashIcon,
  moveFile: ArrowRight,
  organizeFolder: FolderOpen,
  default: FileCode,
};

function TrashIcon(props: any) {
  return <Ban {...props} />;
}

interface MessageBubbleProps {
  message: AgentMessage;
  isExecuting?: boolean;
  onExecute?: (planId: string) => void;
  onExecuteToolCalls?: (
    messageId: string,
    toolCalls: any[],
    workspaceId: string,
  ) => void;
  workspaceId?: string;
}

export const MessageBubble = React.memo(function MessageBubble({
  message,
  onExecute,
  onExecuteToolCalls,
  isExecuting,
  workspaceId,
}: MessageBubbleProps) {
  const isUser = message.type === "user";
  const isSystem = message.type === "system";

  const handleExecuteToolCalls = () => {
    if (message.toolCalls && onExecuteToolCalls && workspaceId) {
      onExecuteToolCalls(message.id, message.toolCalls, workspaceId);
    }
  };

  if (isSystem) {
    return (
      <div className="flex justify-center my-4">
        <span className="text-xs text-muted-foreground bg-muted/30 px-3 py-1 rounded-full border border-border/20">
          {message.content}
        </span>
      </div>
    );
  }

  // Determine Neural State
  const neuralState = useMemo((): NeuralState => {
    // 1. Top Priority: Real-time state from backend agent events
    if (message.neuralState && message.isLoading) {
      return message.neuralState as NeuralState;
    }

    // 2. Check for specific tool calls (Deep Mode - pre-existing analysis)
    if (
      message.toolCalls &&
      message.toolCalls.length > 0 &&
      !message.isExecuted
    ) {
      for (const tc of message.toolCalls) {
        if (TOOL_STATE_MAP[tc.method]) {
          return TOOL_STATE_MAP[tc.method];
        }
      }
      return "planning";
    }

    // 3. Generic execution / loading fallbacks
    if (isExecuting) return "executing";
    if (message.isLoading) return "thinking";

    return "idle";
  }, [
    isExecuting,
    message.toolCalls,
    message.isExecuted,
    message.isLoading,
    message.neuralState,
  ]);

  return (
    <div
      className={`flex w-full gap-4 ${isUser ? "flex-row-reverse" : "flex-row"}`}
    >
      {/* Content */}
      <div
        className={`flex flex-col gap-1 max-w-[85%] ${isUser ? "items-end" : "items-start"}`}
      >
        <div
          className={`rounded-[20px] px-5 py-3.5 shadow-sm text-[15px] leading-relaxed transition-all relative overflow-hidden ${
            isUser
              ? "bg-primary text-primary-foreground rounded-br-sm"
              : neuralState !== "idle"
                ? "bg-white/40 dark:bg-white/5 border border-primary/20 text-foreground backdrop-blur-md rounded-bl-sm shadow-[0_0_15px_-3px_rgba(var(--primary-rgb),0.1)]"
                : "bg-white/40 dark:bg-white/5 border border-white/10 text-foreground backdrop-blur-md rounded-bl-sm"
          }`}
        >
          {/* Animated Background for Active States */}
          {!isUser && neuralState !== "idle" && (
            <div className="absolute inset-0 pointer-events-none overflow-hidden rounded-[20px]">
              <div className="absolute inset-0 bg-primary/5" />
              <motion.div
                initial={{ opacity: 0 }}
                animate={{ opacity: [0.1, 0.3, 0.1] }}
                transition={{ duration: 2, repeat: Infinity }}
                className="absolute inset-0 bg-gradient-to-r from-transparent via-primary/10 to-transparent skew-x-12 translate-x-[-100%]"
                style={{ translateX: "-100%" }}
              />
            </div>
          )}

          <div className="relative z-10">
            {message.content ? (
              <MarkdownRenderer content={message.content} />
            ) : neuralState !== "idle" ? (
              <NeuralStatus
                state={neuralState}
                toolName={message.activeToolName}
              />
            ) : null}
          </div>
        </div>

        {/* Thought/Reasoning Display (Only for Agent with thinking) */}
        {!isUser && message.thought && (
          <ThoughtDisplay
            thought={message.thought}
            thinkingLevel={message.thinkingLevel || "medium"}
            modelName={message.modelUsed?.name}
            className="w-full max-w-md md:max-w-lg lg:max-w-xl"
            isStreaming={message.isLoading}
            durationMs={message.thoughtDuration}
          />
        )}

        {/* Compact Thought Badge (when thinking is enabled but content not shown) */}
        {!isUser && !message.thought && message.modelUsed?.thinkingEnabled && (
          <ThoughtBadge thinkingLevel={message.thinkingLevel || "medium"} />
        )}

        {/* New Plan Confirmation Card (Deep Mode) */}
        {!isUser && message.toolCalls && !message.isExecuted && (
          <PlanConfirmationCard
            toolCalls={message.toolCalls}
            onExecute={handleExecuteToolCalls}
            isExecuting={isExecuting}
          />
        )}

        {/* Legacy Plan Display (Only for Agent) */}
        {!isUser && message.plan && (
          <PlanCard
            plan={message.plan}
            onExecute={onExecute}
            isExecuting={isExecuting}
          />
        )}

        {/* Result Display */}
        {!isUser && message.result && (
          <div className="w-full bg-green-500/10 border border-green-500/20 rounded-xl p-4 text-sm">
            <p className="font-semibold text-green-600 dark:text-green-400 mb-2">
              Execution Result
            </p>
            <div className="space-y-1 text-muted-foreground">
              <p>Total steps: {message.result.totalSteps}</p>
              <p>Changes made: {message.result.totalChanges}</p>
              {message.result.errors.length > 0 && (
                <div className="mt-2 p-2 bg-red-500/10 text-red-500 rounded text-xs">
                  {message.result.errors.map((e, i) => (
                    <div key={i}>{e}</div>
                  ))}
                </div>
              )}
            </div>
          </div>
        )}

        {/* Timestamp */}
        <span className="text-[10px] text-muted-foreground/50 px-1">
          {message.timestamp.toLocaleTimeString([], {
            hour: "2-digit",
            minute: "2-digit",
          })}
        </span>
      </div>
    </div>
  );
});

function PlanCard({
  plan,
  onExecute,
  isExecuting,
}: {
  plan: TaskPlan;
  onExecute?: (id: string) => void;
  isExecuting?: boolean;
}) {
  return (
    <Card className="w-full max-w-md md:max-w-lg lg:max-w-xl p-4 space-y-4 border-l-4 border-l-purple-500 bg-purple-50/50 dark:bg-purple-900/10">
      <div className="flex items-center justify-between">
        <h3 className="font-medium text-sm flex items-center gap-2">
          <FileCode className="size-4 text-purple-500" />
          Proposed Plan
        </h3>
        <span className="text-xs text-muted-foreground bg-background/50 px-2 py-1 rounded">
          {plan.steps.length} steps
        </span>
      </div>

      <div className="space-y-2">
        {plan.steps.map((step, idx) => {
          const Icon = stepIcons[step.type] || stepIcons.default;
          return (
            <div
              key={idx}
              className="flex gap-3 items-start text-xs p-2 rounded bg-background/40 hover:bg-background/80 transition-colors border border-transparent hover:border-border/30"
            >
              <Icon className="size-3.5 mt-0.5 text-muted-foreground shrink-0" />
              <span className="text-foreground/80">{step.description}</span>
            </div>
          );
        })}
      </div>

      {plan.warnings.length > 0 && (
        <div className="bg-orange-500/10 text-orange-600 dark:text-orange-400 p-3 rounded-lg text-xs space-y-1">
          <p className="font-semibold flex items-center gap-1">⚠️ Warnings</p>
          <ul className="list-disc list-inside opacity-90">
            {plan.warnings.map((w, i: number) => (
              <li key={i}>{w}</li>
            ))}
          </ul>
        </div>
      )}

      <div className="flex gap-2 pt-2">
        <Button
          className="flex-1 bg-purple-600 hover:bg-purple-700 text-white shadow-lg shadow-purple-500/20"
          size="sm"
          onPress={() => onExecute?.(plan.id)}
          isDisabled={isExecuting}
          isPending={isExecuting}
        >
          <Play className="size-3.5" />
          Execute Plan
        </Button>
      </div>
    </Card>
  );
}

// Neural Status Component
const NeuralStatus = ({
  state,
  toolName,
}: {
  state: NeuralState;
  toolName?: string;
}) => {
  const config = useMemo(() => getNeuralStateConfig(state), [state]);

  const Icon = config.icon;

  return (
    <div className={`flex items-center gap-3 py-1 ${config.color}`}>
      <div className={`p-2 rounded-full ${config.bgColor}`}>
        <Icon className="size-4 animate-pulse" />
      </div>
      <div className="flex flex-col gap-0.5">
        <span className="text-sm font-medium">{toolName || config.text}</span>
        {/* Animated progress bar */}
        <div className="flex gap-0.5 h-1 mt-1 overflow-hidden rounded-full w-16">
          {[0, 1, 2, 3].map((i) => (
            <motion.div
              key={i}
              animate={{
                opacity: [0.2, 0.8, 0.2],
              }}
              transition={{
                duration: 1.2,
                repeat: Infinity,
                delay: i * 0.15,
                ease: "easeInOut",
              }}
              className={`flex-1 h-full rounded-full ${config.bgColor}`}
            />
          ))}
        </div>
      </div>
    </div>
  );
};
