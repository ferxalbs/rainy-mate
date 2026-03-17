import React, { useMemo } from "react";
import {
  Play,
  Ban,
  FileCode,
  FolderOpen,
  ArrowRight,
  Copy,
  RotateCcw,
  Square,
  ChevronDown,
} from "lucide-react";
import { Button } from "../ui/button";
import { Card } from "../ui/card";
// framer-motion removed — CSS animations only for perf
import type { AgentMessage, TaskPlan } from "../../types/agent";
import { MarkdownRenderer } from "./MarkdownRenderer";
import { PlanConfirmationCard } from "./PlanConfirmationCard";
import {
  type NeuralState,
  TOOL_STATE_MAP,
  getNeuralStateConfig,
} from "./neural-config";

import { ThoughtDisplay } from "./ThoughtDisplay";
import type { SpecialistRunState } from "../../types/agent";

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
  onStopRun?: (messageId: string) => void;
  onRetryRun?: (messageId: string) => void;
  workspaceId?: string;
}

function MessageBubbleComponent({
  message,
  onExecute,
  onExecuteToolCalls,
  onStopRun,
  onRetryRun,
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

  const handleCopy = async () => {
    if (!message.content) return;
    try {
      await navigator.clipboard.writeText(message.content);
    } catch (error) {
      console.error("Failed to copy message", error);
    }
  };

  const traceStats = useMemo(() => {
    const trace = message.trace || [];
    return {
      total: trace.length,
      toolCalls: trace.filter((item) => item.phase === "tool").length,
      retries: trace.filter((item) => item.phase === "retry").length,
      errors: trace.filter((item) => item.phase === "error").length,
      approvals: trace.filter((item) => item.phase === "approval").length,
    };
  }, [message.trace]);

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
      className={`flex w-full min-w-0 gap-4 ${isUser ? "flex-row-reverse" : "flex-row"}`}
    >
      {/* Content */}
      <div
        className={`flex min-w-0 flex-col gap-1 max-w-[85%] ${isUser ? "items-end" : "items-start"}`}
      >
        <div
          className={`relative w-full min-w-0 overflow-hidden rounded-[20px] px-5 py-3.5 text-[15px] leading-relaxed shadow-sm transition-all ${
            isUser
              ? "bg-primary text-primary-foreground rounded-br-sm"
              : neuralState !== "idle"
                ? "bg-white/40 dark:bg-white/5 border border-primary/20 text-foreground backdrop-blur-md rounded-bl-sm shadow-[0_0_15px_-3px_rgba(var(--primary-rgb),0.1)]"
                : "bg-white/40 dark:bg-white/5 border border-white/10 text-foreground backdrop-blur-md rounded-bl-sm"
          }`}
        >
          {/* Animated Background for Active States — CSS only */}
          {!isUser && neuralState !== "idle" && (
            <div className="absolute inset-0 pointer-events-none overflow-hidden rounded-[20px]">
              <div className="absolute inset-0 bg-primary/5" />
              <div
                className="absolute inset-0 bg-gradient-to-r from-transparent via-primary/10 to-transparent skew-x-12 animate-[shimmer_2s_ease-in-out_infinite]"
              />
            </div>
          )}

          <div className="relative z-10 min-w-0 select-text">
            {message.content ? (
              <MarkdownRenderer
                content={message.content}
                isStreaming={message.isLoading}
              />
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

        {!isUser && (
          <div className="flex items-center gap-2">
            <Button
              size="sm"
              variant="ghost"
              className="h-7 px-2 text-xs text-muted-foreground hover:text-foreground"
              onClick={handleCopy}
            >
              <Copy className="size-3.5" />
              Copy
            </Button>
            <Button
              size="sm"
              variant="ghost"
              className="h-7 px-2 text-xs text-muted-foreground hover:text-foreground"
              onClick={() => onRetryRun?.(message.id)}
              disabled={!message.requestContext?.prompt || message.isLoading}
            >
              <RotateCcw className="size-3.5" />
              Retry
            </Button>
            {message.isLoading && (
              <Button
                size="sm"
                variant="ghost"
                className="h-7 px-2 text-xs text-red-500 hover:text-red-400"
                onClick={() => onStopRun?.(message.id)}
              >
                <Square className="size-3.5" />
                Stop
              </Button>
            )}
          </div>
        )}

        {!isUser && (message.trace?.length || message.isLoading) ? (
          <TraceAccordion
            trace={message.trace || []}
            runState={message.runState}
            stats={traceStats}
          />
        ) : null}

        {!isUser &&
          (message.supervisorPlan ||
            (message.specialists && message.specialists.length > 0)) && (
            <SupervisorRail
              summary={message.supervisorPlan?.summary}
              steps={message.supervisorPlan?.steps || []}
              specialists={message.specialists || []}
            />
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
                  {message.result.errors.map((e) => (
                    <div key={e}>{e}</div>
                  ))}
                </div>
              )}
            </div>
          </div>
        )}

        {/* Timestamp */}
        <div className="flex items-center gap-2 px-1">
          {!isUser && (message.supervisorPlan || (message.specialists && message.specialists.length > 0)) && (
            <span className="rounded-full border border-primary/20 bg-primary/10 px-2 py-0.5 text-[10px] font-medium uppercase tracking-wide text-primary">
              Supervisor
            </span>
          )}
          {!isUser && message.modelUsed?.name && (
            <span className="text-[10px] text-muted-foreground/70 font-medium">
              {message.modelUsed.name}
            </span>
          )}
          <span className="text-[10px] text-muted-foreground/50">
            {message.timestamp.toLocaleTimeString([], {
              hour: "2-digit",
              minute: "2-digit",
            })}
          </span>
        </div>
      </div>
    </div>
  );
}

export const MessageBubble = React.memo(
  MessageBubbleComponent,
  (prev, next) =>
    prev.message === next.message &&
    prev.isExecuting === next.isExecuting &&
    prev.workspaceId === next.workspaceId,
);

// Re-export with a name hint for the parent to avoid confusion
export { MessageBubble as MemoizedMessageBubble };

function SupervisorRail({
  summary,
  steps,
  specialists,
}: {
  summary?: string;
  steps: string[];
  specialists: SpecialistRunState[];
}) {
  const roleLabel: Record<SpecialistRunState["role"], string> = {
    research: "Research",
    executor: "Executor",
    verifier: "Verifier",
  };

  const statusTone: Record<
    SpecialistRunState["status"],
    string
  > = {
    pending: "text-muted-foreground",
    planning: "text-amber-400",
    running: "text-cyan-500",
    waiting_on_airlock: "text-orange-500",
    verifying: "text-emerald-500",
    completed: "text-green-500",
    failed: "text-red-500",
    cancelled: "text-muted-foreground",
  };

  const formatDuration = (specialist: SpecialistRunState) => {
    if (!specialist.startedAtMs) return null;
    // Use finishedAtMs when available; for running specialists show nothing
    // to avoid impure Date.now() calls during render
    if (!specialist.finishedAtMs) return null;
    const elapsedMs = Math.max(0, specialist.finishedAtMs - specialist.startedAtMs);
    if (elapsedMs < 1000) return `${elapsedMs}ms`;
    return `${(elapsedMs / 1000).toFixed(1)}s`;
  };

  return (
    <div className="w-full rounded-2xl border border-primary/15 bg-background/50 p-4 backdrop-blur-md">
      {summary && (
        <div className="mb-3">
          <p className="text-xs uppercase tracking-[0.18em] text-primary/70">
            Supervisor
          </p>
          <p className="mt-1 text-sm text-foreground">{summary}</p>
        </div>
      )}

      {steps.length > 0 && (
        <div className="mb-3 flex flex-wrap gap-2">
          {steps.map((step) => (
            <span
              key={step}
              className="rounded-full border border-border/40 bg-background/70 px-3 py-1 text-[11px] text-muted-foreground"
            >
              {step}
            </span>
          ))}
        </div>
      )}

      {specialists.length > 0 && (
        <div className="grid gap-2 xl:grid-cols-3">
          {specialists.map((specialist) => (
            <div
              key={specialist.agentId}
              className="min-w-0 rounded-xl border border-border/30 bg-background/70 p-3"
            >
              <div className="flex items-center justify-between gap-2">
                <span className="text-sm font-medium text-foreground">
                  {roleLabel[specialist.role]}
                </span>
                <span
                  className={`text-[11px] font-medium uppercase tracking-wide ${statusTone[specialist.status]}`}
                >
                  {specialist.status.replace(/_/g, " ")}
                </span>
              </div>

              {specialist.detail && (
                <p className="mt-2 break-words text-xs text-muted-foreground">
                  {specialist.detail}
                </p>
              )}
              {specialist.dependsOn && specialist.dependsOn.length > 0 && (
                <p className="mt-2 break-words text-[11px] text-muted-foreground">
                  Depends on: {specialist.dependsOn.join(", ")}
                </p>
              )}
              {specialist.activeTool && (
                <p className="mt-2 text-xs text-primary/80">
                  Tool: {specialist.activeTool}
                </p>
              )}
              {(specialist.toolCount || formatDuration(specialist) || specialist.writeLikeUsed) && (
                <p className="mt-2 text-[11px] text-muted-foreground">
                  {specialist.toolCount ? `${specialist.toolCount} tool${specialist.toolCount === 1 ? "" : "s"}` : "No tools yet"}
                  {formatDuration(specialist) ? ` · ${formatDuration(specialist)}` : ""}
                  {specialist.writeLikeUsed ? " · write-like actions" : ""}
                </p>
              )}
              {specialist.responsePreview && (
                <p className="mt-2 line-clamp-3 text-xs text-foreground/80">
                  {specialist.responsePreview}
                </p>
              )}
              {specialist.error && (
                <p className="mt-2 text-xs text-red-500">{specialist.error}</p>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function TraceAccordion({
  trace,
  runState,
  stats,
}: {
  trace: NonNullable<AgentMessage["trace"]>;
  runState?: AgentMessage["runState"];
  stats: {
    total: number;
    toolCalls: number;
    retries: number;
    errors: number;
    approvals: number;
  };
}) {
  const stateTone =
    runState === "failed"
      ? "text-red-500"
      : runState === "cancelled"
        ? "text-amber-500"
        : runState === "completed"
          ? "text-emerald-500"
          : "text-cyan-500";

  return (
    <details className="w-full rounded-xl border border-border/30 bg-background/40 px-3 py-2">
      <summary className="flex cursor-pointer list-none items-center justify-between gap-2 text-xs text-muted-foreground">
        <span className="flex items-center gap-2">
          <ChevronDown className="size-3.5" />
          <span className={`font-medium ${stateTone}`}>
            Runtime Trace {runState ? `(${runState})` : ""}
          </span>
          <span>calls {stats.toolCalls}</span>
          <span>retries {stats.retries}</span>
          <span>errors {stats.errors}</span>
          <span>approvals {stats.approvals}</span>
        </span>
        <span>{stats.total} events</span>
      </summary>

      <div className="mt-2 max-h-52 overflow-y-auto rounded-lg border border-border/20 bg-background/30 p-2 font-mono text-[11px]">
        {trace.length === 0 ? (
          <div className="text-muted-foreground">Waiting for runtime events...</div>
        ) : (
          trace.map((item) => (
            <div
              key={item.id}
              className="mb-1.5 rounded-md border border-border/10 bg-background/40 p-2"
            >
              <div className="flex items-center justify-between gap-2 text-muted-foreground">
                <span className="uppercase tracking-wide">{item.phase}</span>
                <span>
                  {item.timestamp.toLocaleTimeString([], {
                    hour: "2-digit",
                    minute: "2-digit",
                    second: "2-digit",
                  })}
                </span>
              </div>
              <div className="mt-1 text-foreground">{item.label}</div>
              {item.preview && (
                <div className="mt-1 line-clamp-2 text-muted-foreground">{item.preview}</div>
              )}
            </div>
          ))
        )}
      </div>
    </details>
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
        {plan.steps.map((step) => {
          const Icon = stepIcons[step.type] || stepIcons.default;
          return (
            <div
              key={`${step.type}-${step.description}`}
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
            {plan.warnings.map((w) => (
              <li key={w}>{w}</li>
            ))}
          </ul>
        </div>
      )}

      <div className="flex gap-2 pt-2">
        <Button
          className="flex-1 bg-purple-600 hover:bg-purple-700 text-white shadow-lg shadow-purple-500/20"
          size="sm"
          onClick={() => onExecute?.(plan.id)}
          disabled={isExecuting}
        >
          <Play className="size-3.5" />
          Execute Plan
        </Button>
      </div>
    </Card>
  );
}

// Neural Status Component — CSS animations only
const NeuralStatus = React.memo(({
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
        <div className="flex gap-0.5 h-1 mt-1 overflow-hidden rounded-full w-16">
          {[0, 1, 2, 3].map((barIndex) => (
            <div
              key={`neural-bar-${barIndex}`}
              className={`flex-1 h-full rounded-full ${config.bgColor} animate-[neural-bar_1.2s_ease-in-out_infinite]`}
              style={{ animationDelay: `${barIndex * 0.15}s` }}
            />
          ))}
        </div>
      </div>
    </div>
  );
});
