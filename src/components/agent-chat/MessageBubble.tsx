import React from "react";
import {
  Loader2,
  Play,
  Ban,
  FileCode,
  FolderOpen,
  ArrowRight,
} from "lucide-react";
import { Button, Card } from "@heroui/react";
import type { AgentMessage, TaskPlan } from "../../types/agent";
import { MarkdownRenderer } from "./MarkdownRenderer";
import { PlanConfirmationCard } from "./PlanConfirmationCard";

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
  currentPlan?: TaskPlan | null;
  isExecuting?: boolean;
  onExecute?: (planId: string) => void;
  onExecuteToolCalls?: (
    messageId: string,
    toolCalls: any[],
    workspaceId: string,
  ) => void;
  workspaceId?: string;
}

export function MessageBubble({
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

  // Remove the raw tool calls from the display content if we are visualizing them
  // checking if toolCalls are present, we might want to keep the context though.

  return (
    <div
      className={`flex w-full gap-4 ${isUser ? "flex-row-reverse" : "flex-row"}`}
    >
      {/* Content */}
      <div
        className={`flex flex-col gap-1 max-w-[85%] ${isUser ? "items-end" : "items-start"}`}
      >
        <div
          className={`rounded-[20px] px-5 py-3.5 shadow-sm text-[15px] leading-relaxed transition-all ${
            isUser
              ? "bg-primary text-primary-foreground rounded-br-sm"
              : "bg-white/40 dark:bg-white/5 border border-white/10 text-foreground backdrop-blur-md rounded-bl-sm"
          }`}
        >
          {message.content ? (
            <MarkdownRenderer content={message.content} />
          ) : message.isLoading ? (
            <span className="flex items-center gap-2 text-muted-foreground italic">
              <Loader2 className="size-3 animate-spin" /> Thinking...
            </span>
          ) : null}
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
}

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
