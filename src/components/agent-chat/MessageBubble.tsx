import React from "react";
import Markdown from "react-markdown";
import {
  User,
  Bot,
  Loader2,
  Play,
  Ban,
  FileCode,
  FolderOpen,
  ArrowRight,
} from "lucide-react";
import { Button, Card } from "@heroui/react";
import type { AgentMessage } from "../../hooks/useCoworkAgent";
import * as tauri from "../../services/tauri";
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
  currentPlan?: tauri.TaskPlan | null;
  isExecuting?: boolean;
  onExecute?: (planId: string) => void;
  onCancel?: (planId: string) => void;
}

export function MessageBubble({
  message,
  onExecute,
  onCancel,
  isExecuting,
}: MessageBubbleProps) {
  const isUser = message.type === "user";
  const isSystem = message.type === "system";

  if (isSystem) {
    return (
      <div className="flex justify-center my-4">
        <span className="text-xs text-muted-foreground bg-muted/30 px-3 py-1 rounded-full border border-border/20">
          {message.content}
        </span>
      </div>
    );
  }

  return (
    <div
      className={`flex w-full gap-4 ${isUser ? "flex-row-reverse" : "flex-row"}`}
    >
      {/* Avatar */}
      <div
        className={`size-8 shrink-0 rounded-full flex items-center justify-center border shadow-sm ${
          isUser
            ? "bg-primary/20 border-primary/20 text-primary"
            : "bg-purple-500/20 border-purple-500/20 text-purple-600"
        }`}
      >
        {isUser ? <User className="size-4" /> : <Bot className="size-4" />}
      </div>

      {/* Content */}
      <div
        className={`flex flex-col gap-2 max-w-[85%] ${isUser ? "items-end" : "items-start"}`}
      >
        <div
          className={`rounded-2xl px-5 py-3 shadow-sm border text-sm leading-relaxed ${
            isUser
              ? "bg-primary text-primary-foreground border-primary"
              : "bg-card border-border/40 text-foreground"
          }`}
        >
          {message.content ? (
            <div
              className={`prose prose-sm dark:prose-invert max-w-none ${isUser ? "text-primary-foreground" : ""}`}
            >
              <Markdown>{message.content}</Markdown>
            </div>
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
            className="w-full max-w-md"
          />
        )}

        {/* Compact Thought Badge (when thinking is enabled but content not shown) */}
        {!isUser && !message.thought && message.modelUsed?.thinkingEnabled && (
          <ThoughtBadge thinkingLevel={message.thinkingLevel || "medium"} />
        )}

        {/* Plan Display (Only for Agent) */}
        {!isUser && message.plan && (
          <PlanCard
            plan={message.plan}
            onExecute={onExecute}
            onCancel={onCancel}
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
  onCancel,
  isExecuting,
}: {
  plan: tauri.TaskPlan;
  onExecute?: (id: string) => void;
  onCancel?: (id: string) => void;
  isExecuting?: boolean;
}) {
  return (
    <Card className="w-full max-w-md p-4 space-y-4 border-l-4 border-l-purple-500 bg-purple-50/50 dark:bg-purple-900/10">
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
            {plan.warnings.map((w, i) => (
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
        <Button
          className="flex-1"
          variant="danger-soft"
          size="sm"
          onPress={() => onCancel?.(plan.id)}
          isDisabled={isExecuting}
        >
          Cancel
        </Button>
      </div>
    </Card>
  );
}
