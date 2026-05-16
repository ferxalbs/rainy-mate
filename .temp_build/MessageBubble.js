// src/components/agent-chat/MessageBubble.tsx
import React, { useMemo, useCallback } from "react";
import {
  Play,
  Ban,
  FileCode,
  FileText,
  FolderOpen,
  ArrowRight,
  Copy,
  Image,
  RotateCcw,
  Square,
  ChevronDown,
  Bot,
  TerminalSquare,
  FileOutput
} from "lucide-react";
import { Button } from "../ui/button";
import { Card } from "../ui/card";
import { MarkdownRenderer } from "./MarkdownRenderer";
import { PlanConfirmationCard } from "./PlanConfirmationCard";
import { ArtifactBadgeRow } from "./ArtifactBadgeRow";
import {
  TOOL_STATE_MAP,
  getNeuralStateConfig
} from "./neural-config";
import { ThoughtDisplay } from "./ThoughtDisplay";
import { jsxDEV, Fragment } from "react/jsx-dev-runtime";
var EMPTY_ARRAY = [];
var stepIcons = {
  createFile: FileCode,
  modifyFile: FileCode,
  deleteFile: TrashIcon,
  moveFile: ArrowRight,
  organizeFolder: FolderOpen,
  default: FileCode
};
function TrashIcon(props) {
  return /* @__PURE__ */ jsxDEV(Ban, {
    ...props
  }, undefined, false, undefined, this);
}
function MessageBubbleComponent({
  message,
  onExecute,
  onExecuteToolCalls,
  onStopRun,
  onRetryRun,
  isExecuting,
  workspaceId
}) {
  const isUser = message.type === "user";
  const isSystem = message.type === "system";
  const handleExecuteToolCalls = useCallback(() => {
    if (message.toolCalls && onExecuteToolCalls && workspaceId) {
      onExecuteToolCalls(message.id, message.toolCalls, workspaceId);
    }
  }, [message.id, message.toolCalls, onExecuteToolCalls, workspaceId]);
  const handleCopy = useCallback(async () => {
    if (!message.content)
      return;
    try {
      await navigator.clipboard.writeText(message.content);
    } catch (error) {
      console.error("Failed to copy message", error);
    }
  }, [message.content]);
  const handleStopRun = useCallback(() => {
    if (onStopRun)
      onStopRun(message.id);
  }, [message.id, onStopRun]);
  const handleRetryRun = useCallback(() => {
    if (onRetryRun)
      onRetryRun(message.id);
  }, [message.id, onRetryRun]);
  const handleExecutePlan = useCallback((planId) => {
    if (onExecute)
      onExecute(planId);
  }, [onExecute]);
  const traceStats = useMemo(() => {
    const trace = message.trace || EMPTY_ARRAY;
    let toolCalls = 0;
    let retries = 0;
    let errors = 0;
    let approvals = 0;
    for (const item of trace) {
      if (item.phase === "tool")
        toolCalls += 1;
      if (item.phase === "retry")
        retries += 1;
      if (item.phase === "error")
        errors += 1;
      if (item.phase === "approval")
        approvals += 1;
    }
    return {
      total: trace.length,
      toolCalls,
      retries,
      errors,
      approvals
    };
  }, [message.trace]);
  const neuralState = useMemo(() => {
    if (message.neuralState && message.isLoading) {
      return message.neuralState;
    }
    if (message.toolCalls && message.toolCalls.length > 0 && !message.isExecuted) {
      for (const tc of message.toolCalls) {
        if (TOOL_STATE_MAP[tc.method]) {
          return TOOL_STATE_MAP[tc.method];
        }
      }
      return "planning";
    }
    if (isExecuting)
      return "executing";
    if (message.isLoading)
      return "thinking";
    return "idle";
  }, [
    isExecuting,
    message.toolCalls,
    message.isExecuted,
    message.isLoading,
    message.neuralState
  ]);
  if (isSystem) {
    return /* @__PURE__ */ jsxDEV("div", {
      className: "flex justify-center my-4",
      children: /* @__PURE__ */ jsxDEV("span", {
        className: "text-xs text-muted-foreground bg-muted/30 px-3 py-1 rounded-full border border-border/20",
        children: message.content
      }, undefined, false, undefined, this)
    }, undefined, false, undefined, this);
  }
  return /* @__PURE__ */ jsxDEV("div", {
    className: `flex w-full min-w-0 gap-4 ${isUser ? "flex-row-reverse" : "flex-row"}`,
    children: /* @__PURE__ */ jsxDEV("div", {
      className: `flex min-w-0 flex-col gap-1 max-w-[85%] ${isUser ? "items-end" : "items-start"}`,
      children: [
        /* @__PURE__ */ jsxDEV("div", {
          className: `relative w-full min-w-0 overflow-hidden rounded-[20px] px-5 py-3.5 text-[15px] leading-relaxed shadow-sm transition-colors ${isUser ? "bg-primary/70 dark:bg-primary/40 text-primary-foreground rounded-br-sm" : neuralState !== "idle" ? "bg-white/40 dark:bg-white/5 border border-primary/20 text-foreground backdrop-blur-md rounded-bl-sm shadow-[0_0_15px_-3px_rgba(var(--primary-rgb),0.1)]" : "bg-white/40 dark:bg-white/5 border border-white/10 text-foreground backdrop-blur-md rounded-bl-sm"}`,
          children: [
            !isUser && neuralState !== "idle" && /* @__PURE__ */ jsxDEV("div", {
              className: "absolute inset-0 pointer-events-none overflow-hidden rounded-[20px]",
              children: [
                /* @__PURE__ */ jsxDEV("div", {
                  className: "absolute inset-0 bg-primary/5"
                }, undefined, false, undefined, this),
                /* @__PURE__ */ jsxDEV("div", {
                  className: "absolute inset-0 bg-gradient-to-r from-transparent via-primary/10 to-transparent skew-x-12 animate-[shimmer_2s_ease-in-out_infinite]"
                }, undefined, false, undefined, this)
              ]
            }, undefined, true, undefined, this),
            /* @__PURE__ */ jsxDEV("div", {
              className: "relative z-10 min-w-0 select-text",
              children: [
                isUser && message.attachments && message.attachments.length > 0 && /* @__PURE__ */ jsxDEV("div", {
                  className: "mb-2 flex flex-wrap gap-1.5",
                  children: message.attachments.map((att) => /* @__PURE__ */ jsxDEV("div", {
                    className: "flex items-center gap-1.5 rounded-lg border border-white/20 bg-white/10 px-2 py-1 text-xs text-primary-foreground/80",
                    children: [
                      att.type === "image" && att.thumbnailDataUri ? /* @__PURE__ */ jsxDEV("img", {
                        src: att.thumbnailDataUri,
                        alt: att.filename,
                        className: "size-7 rounded object-cover"
                      }, undefined, false, undefined, this) : att.type === "image" ? /* @__PURE__ */ jsxDEV(Image, {
                        className: "size-4 shrink-0"
                      }, undefined, false, undefined, this) : /* @__PURE__ */ jsxDEV(FileText, {
                        className: "size-4 shrink-0"
                      }, undefined, false, undefined, this),
                      /* @__PURE__ */ jsxDEV("span", {
                        className: "max-w-[140px] truncate",
                        children: att.filename
                      }, undefined, false, undefined, this)
                    ]
                  }, att.id, true, undefined, this))
                }, undefined, false, undefined, this),
                message.content ? /* @__PURE__ */ jsxDEV(MarkdownRenderer, {
                  content: message.content,
                  isStreaming: message.isLoading,
                  useContentVisibility: false,
                  tone: isUser ? "user" : "assistant"
                }, undefined, false, undefined, this) : neuralState !== "idle" ? /* @__PURE__ */ jsxDEV(NeuralStatus, {
                  state: neuralState,
                  toolName: message.activeToolName,
                  airlockLevel: message.airlockLevel
                }, undefined, false, undefined, this) : null
              ]
            }, undefined, true, undefined, this)
          ]
        }, undefined, true, undefined, this),
        !isUser && !message.isLoading && message.artifacts && message.artifacts.length > 0 && /* @__PURE__ */ jsxDEV(ArtifactBadgeRow, {
          artifacts: message.artifacts
        }, undefined, false, undefined, this),
        !isUser && message.thought && /* @__PURE__ */ jsxDEV(ThoughtDisplay, {
          thought: message.thought,
          thinkingLevel: message.thinkingLevel || "medium",
          modelName: message.modelUsed?.name,
          className: "w-full max-w-md md:max-w-lg lg:max-w-xl",
          isStreaming: message.isLoading,
          durationMs: message.thoughtDuration
        }, undefined, false, undefined, this),
        !isUser && /* @__PURE__ */ jsxDEV("div", {
          className: "flex items-center gap-2",
          children: [
            /* @__PURE__ */ jsxDEV(Button, {
              size: "sm",
              variant: "ghost",
              className: "h-7 px-2 text-xs text-muted-foreground hover:text-foreground",
              onClick: handleCopy,
              children: [
                /* @__PURE__ */ jsxDEV(Copy, {
                  className: "size-3.5"
                }, undefined, false, undefined, this),
                "Copy"
              ]
            }, undefined, true, undefined, this),
            /* @__PURE__ */ jsxDEV(Button, {
              size: "sm",
              variant: "ghost",
              className: "h-7 px-2 text-xs text-muted-foreground hover:text-foreground",
              onClick: handleRetryRun,
              disabled: !message.requestContext?.prompt || message.isLoading,
              children: [
                /* @__PURE__ */ jsxDEV(RotateCcw, {
                  className: "size-3.5"
                }, undefined, false, undefined, this),
                "Retry"
              ]
            }, undefined, true, undefined, this),
            message.isLoading && /* @__PURE__ */ jsxDEV(Button, {
              size: "sm",
              variant: "ghost",
              className: "h-7 px-2 text-xs text-red-500 hover:text-red-400",
              onClick: handleStopRun,
              children: [
                /* @__PURE__ */ jsxDEV(Square, {
                  className: "size-3.5"
                }, undefined, false, undefined, this),
                "Stop"
              ]
            }, undefined, true, undefined, this)
          ]
        }, undefined, true, undefined, this),
        !isUser && (message.trace?.length || message.isLoading) ? /* @__PURE__ */ jsxDEV(TraceAccordion, {
          trace: message.trace || EMPTY_ARRAY,
          runState: message.runState,
          stats: traceStats
        }, undefined, false, undefined, this) : null,
        !isUser && !message.isLoading && message.externalSessions && message.externalSessions.length > 0 && /* @__PURE__ */ jsxDEV(ExternalSessionRail, {
          sessions: message.externalSessions
        }, undefined, false, undefined, this),
        !isUser && (message.supervisorPlan || message.specialists && message.specialists.length > 0) && /* @__PURE__ */ jsxDEV(SupervisorRail, {
          summary: message.supervisorPlan?.summary,
          steps: message.supervisorPlan?.steps || EMPTY_ARRAY,
          specialists: message.specialists || EMPTY_ARRAY
        }, undefined, false, undefined, this),
        !isUser && message.toolCalls && !message.isExecuted && /* @__PURE__ */ jsxDEV(PlanConfirmationCard, {
          toolCalls: message.toolCalls,
          onExecute: handleExecuteToolCalls,
          isExecuting
        }, undefined, false, undefined, this),
        !isUser && message.plan && /* @__PURE__ */ jsxDEV(PlanCard, {
          plan: message.plan,
          onExecute: handleExecutePlan,
          isExecuting
        }, undefined, false, undefined, this),
        !isUser && message.result && /* @__PURE__ */ jsxDEV("div", {
          className: "w-full bg-green-500/10 border border-green-500/20 rounded-xl p-4 text-sm",
          children: [
            /* @__PURE__ */ jsxDEV("p", {
              className: "font-semibold text-green-600 dark:text-green-400 mb-2",
              children: "Execution Result"
            }, undefined, false, undefined, this),
            /* @__PURE__ */ jsxDEV("div", {
              className: "space-y-1 text-muted-foreground",
              children: [
                /* @__PURE__ */ jsxDEV("p", {
                  children: [
                    "Total steps: ",
                    message.result.totalSteps
                  ]
                }, undefined, true, undefined, this),
                /* @__PURE__ */ jsxDEV("p", {
                  children: [
                    "Changes made: ",
                    message.result.totalChanges
                  ]
                }, undefined, true, undefined, this),
                message.result.errors.length > 0 && /* @__PURE__ */ jsxDEV("div", {
                  className: "mt-2 p-2 bg-red-500/10 text-red-500 rounded text-xs",
                  children: message.result.errors.map((e) => /* @__PURE__ */ jsxDEV("div", {
                    children: e
                  }, e, false, undefined, this))
                }, undefined, false, undefined, this)
              ]
            }, undefined, true, undefined, this)
          ]
        }, undefined, true, undefined, this),
        /* @__PURE__ */ jsxDEV("div", {
          className: "flex items-center gap-2 px-1",
          children: [
            !isUser && (message.supervisorPlan || message.specialists && message.specialists.length > 0) && /* @__PURE__ */ jsxDEV("span", {
              className: "rounded-full border border-primary/20 bg-primary/10 px-2 py-0.5 text-[10px] font-medium uppercase tracking-wide text-primary",
              children: "Supervisor"
            }, undefined, false, undefined, this),
            !isUser && message.modelUsed?.name && /* @__PURE__ */ jsxDEV("span", {
              className: "text-[10px] text-muted-foreground/70 font-medium",
              children: message.modelUsed.name
            }, undefined, false, undefined, this),
            /* @__PURE__ */ jsxDEV("span", {
              className: "text-[10px] text-muted-foreground/50",
              children: message.timestamp.toLocaleTimeString([], {
                hour: "2-digit",
                minute: "2-digit"
              })
            }, undefined, false, undefined, this)
          ]
        }, undefined, true, undefined, this)
      ]
    }, undefined, true, undefined, this)
  }, undefined, false, undefined, this);
}
var MessageBubble = React.memo(MessageBubbleComponent, (prev, next) => prev.message === next.message && prev.isExecuting === next.isExecuting && prev.workspaceId === next.workspaceId);
var SupervisorRail = React.memo(function SupervisorRail2({
  summary,
  steps,
  specialists
}) {
  const roleLabel = {
    research: "Research",
    executor: "Executor",
    verifier: "Verifier",
    memory_scribe: "Memory Scribe"
  };
  const statusTone = {
    pending: "text-muted-foreground",
    planning: "text-amber-400",
    running: "text-cyan-500",
    waiting_on_airlock: "text-orange-500",
    verifying: "text-emerald-500",
    completed: "text-green-500",
    failed: "text-red-500",
    cancelled: "text-muted-foreground"
  };
  const formatDuration = (specialist) => {
    if (!specialist.startedAtMs)
      return null;
    if (!specialist.finishedAtMs)
      return null;
    const elapsedMs = Math.max(0, specialist.finishedAtMs - specialist.startedAtMs);
    if (elapsedMs < 1000)
      return `${elapsedMs}ms`;
    return `${(elapsedMs / 1000).toFixed(1)}s`;
  };
  return /* @__PURE__ */ jsxDEV("div", {
    className: "w-full rounded-2xl border border-primary/15 bg-background/50 p-4 backdrop-blur-md",
    children: [
      summary && /* @__PURE__ */ jsxDEV("div", {
        className: "mb-3",
        children: [
          /* @__PURE__ */ jsxDEV("p", {
            className: "text-xs uppercase tracking-[0.18em] text-primary/70",
            children: "Supervisor"
          }, undefined, false, undefined, this),
          /* @__PURE__ */ jsxDEV("p", {
            className: "mt-1 text-sm text-foreground",
            children: summary
          }, undefined, false, undefined, this)
        ]
      }, undefined, true, undefined, this),
      steps.length > 0 && /* @__PURE__ */ jsxDEV("div", {
        className: "mb-3 flex flex-wrap gap-2",
        children: steps.map((step) => /* @__PURE__ */ jsxDEV("span", {
          className: "rounded-full border border-border/40 bg-background/70 px-3 py-1 text-[11px] text-muted-foreground",
          children: step
        }, step, false, undefined, this))
      }, undefined, false, undefined, this),
      specialists.length > 0 && /* @__PURE__ */ jsxDEV("div", {
        className: "grid gap-2 xl:grid-cols-3",
        children: specialists.map((specialist) => /* @__PURE__ */ jsxDEV("div", {
          className: "min-w-0 rounded-xl border border-border/30 bg-background/70 p-3",
          children: [
            /* @__PURE__ */ jsxDEV("div", {
              className: "flex items-center justify-between gap-2",
              children: [
                /* @__PURE__ */ jsxDEV("span", {
                  className: "text-sm font-medium text-foreground",
                  children: roleLabel[specialist.role]
                }, undefined, false, undefined, this),
                /* @__PURE__ */ jsxDEV("span", {
                  className: `text-[11px] font-medium uppercase tracking-wide ${statusTone[specialist.status]}`,
                  children: specialist.status.replace(/_/g, " ")
                }, undefined, false, undefined, this)
              ]
            }, undefined, true, undefined, this),
            specialist.detail && /* @__PURE__ */ jsxDEV("p", {
              className: "mt-2 break-words text-xs text-muted-foreground",
              children: specialist.detail
            }, undefined, false, undefined, this),
            specialist.dependsOn && specialist.dependsOn.length > 0 && /* @__PURE__ */ jsxDEV("p", {
              className: "mt-2 break-words text-[11px] text-muted-foreground",
              children: [
                "Depends on: ",
                specialist.dependsOn.join(", ")
              ]
            }, undefined, true, undefined, this),
            specialist.activeTool && /* @__PURE__ */ jsxDEV("p", {
              className: "mt-2 text-xs text-primary/80",
              children: [
                "Tool: ",
                specialist.activeTool
              ]
            }, undefined, true, undefined, this),
            (specialist.toolCount || formatDuration(specialist) || specialist.writeLikeUsed) && /* @__PURE__ */ jsxDEV("p", {
              className: "mt-2 text-[11px] text-muted-foreground",
              children: [
                specialist.toolCount ? `${specialist.toolCount} tool${specialist.toolCount === 1 ? "" : "s"}` : "No tools yet",
                formatDuration(specialist) ? ` · ${formatDuration(specialist)}` : "",
                specialist.writeLikeUsed ? " · write-like actions" : ""
              ]
            }, undefined, true, undefined, this),
            specialist.error && /* @__PURE__ */ jsxDEV("p", {
              className: "mt-2 text-xs text-red-500",
              children: specialist.error
            }, undefined, false, undefined, this)
          ]
        }, specialist.agentId, true, undefined, this))
      }, undefined, false, undefined, this)
    ]
  }, undefined, true, undefined, this);
});
var ExternalSessionRail = React.memo(function ExternalSessionRail2({
  sessions
}) {
  const statusTone = {
    pending: "text-muted-foreground",
    running: "text-cyan-500",
    completed: "text-emerald-500",
    failed: "text-red-500",
    cancelled: "text-amber-500"
  };
  const formatTimestamp = (value) => {
    if (!value)
      return null;
    return new Intl.DateTimeFormat(undefined, {
      month: "short",
      day: "numeric",
      hour: "numeric",
      minute: "2-digit"
    }).format(new Date(value));
  };
  const formatElapsed = (session) => {
    if (!session.startedAt || !session.finishedAt)
      return null;
    const elapsedMs = Math.max(session.finishedAt - session.startedAt, 0);
    const totalSeconds = Math.round(elapsedMs / 1000);
    if (totalSeconds < 60)
      return `${totalSeconds}s`;
    const minutes = Math.floor(totalSeconds / 60);
    const seconds = totalSeconds % 60;
    return seconds === 0 ? `${minutes}m` : `${minutes}m ${seconds}s`;
  };
  return /* @__PURE__ */ jsxDEV("div", {
    className: "w-full rounded-2xl border border-border/30 bg-background/45 p-4 backdrop-blur-md",
    children: [
      /* @__PURE__ */ jsxDEV("div", {
        className: "mb-3 flex items-center gap-2",
        children: [
          /* @__PURE__ */ jsxDEV(TerminalSquare, {
            className: "size-4 text-primary"
          }, undefined, false, undefined, this),
          /* @__PURE__ */ jsxDEV("p", {
            className: "text-xs uppercase tracking-[0.18em] text-primary/70",
            children: "External Sessions"
          }, undefined, false, undefined, this)
        ]
      }, undefined, true, undefined, this),
      /* @__PURE__ */ jsxDEV("div", {
        className: "grid gap-2 xl:grid-cols-2",
        children: sessions.map((session) => {
          const artifactPaths = new Set(session.artifacts.map((artifact) => artifact.path));
          const inspectionPaths = session.touchedPaths.filter((path) => !artifactPaths.has(path));
          const startedLabel = formatTimestamp(session.startedAt ?? session.createdAt);
          const finishedLabel = formatTimestamp(session.finishedAt);
          const elapsedLabel = formatElapsed(session);
          return /* @__PURE__ */ jsxDEV("div", {
            className: "min-w-0 rounded-xl border border-border/30 bg-background/70 p-3",
            children: [
              /* @__PURE__ */ jsxDEV("div", {
                className: "flex items-center justify-between gap-3",
                children: [
                  /* @__PURE__ */ jsxDEV("div", {
                    className: "min-w-0",
                    children: [
                      /* @__PURE__ */ jsxDEV("div", {
                        className: "flex items-center gap-2",
                        children: [
                          /* @__PURE__ */ jsxDEV(Bot, {
                            className: "size-3.5 text-primary/80"
                          }, undefined, false, undefined, this),
                          /* @__PURE__ */ jsxDEV("span", {
                            className: "truncate text-sm font-medium text-foreground",
                            children: session.runtimeKind === "codex" ? "Codex" : "Claude Code"
                          }, undefined, false, undefined, this)
                        ]
                      }, undefined, true, undefined, this),
                      /* @__PURE__ */ jsxDEV("p", {
                        className: "mt-1 line-clamp-2 text-xs text-muted-foreground",
                        children: session.taskSummary
                      }, undefined, false, undefined, this)
                    ]
                  }, undefined, true, undefined, this),
                  /* @__PURE__ */ jsxDEV("span", {
                    className: `shrink-0 text-[11px] font-medium uppercase tracking-wide ${statusTone[session.status]}`,
                    children: session.status
                  }, undefined, false, undefined, this)
                ]
              }, undefined, true, undefined, this),
              /* @__PURE__ */ jsxDEV("div", {
                className: "mt-3 flex flex-wrap gap-1.5",
                children: [
                  /* @__PURE__ */ jsxDEV("span", {
                    className: "rounded-full border border-border/40 bg-background/70 px-2.5 py-1 text-[11px] text-muted-foreground",
                    children: [
                      session.artifacts.length,
                      " deliverable",
                      session.artifacts.length === 1 ? "" : "s"
                    ]
                  }, undefined, true, undefined, this),
                  /* @__PURE__ */ jsxDEV("span", {
                    className: "rounded-full border border-border/40 bg-background/70 px-2.5 py-1 text-[11px] text-muted-foreground",
                    children: [
                      inspectionPaths.length,
                      " inspected path",
                      inspectionPaths.length === 1 ? "" : "s"
                    ]
                  }, undefined, true, undefined, this),
                  /* @__PURE__ */ jsxDEV("span", {
                    className: "rounded-full border border-border/40 bg-background/70 px-2.5 py-1 text-[11px] text-muted-foreground",
                    children: [
                      session.auditEvents.length,
                      " event",
                      session.auditEvents.length === 1 ? "" : "s"
                    ]
                  }, undefined, true, undefined, this),
                  typeof session.exitCode === "number" && /* @__PURE__ */ jsxDEV("span", {
                    className: "rounded-full border border-border/40 bg-background/70 px-2.5 py-1 text-[11px] text-muted-foreground",
                    children: [
                      "exit ",
                      session.exitCode
                    ]
                  }, undefined, true, undefined, this)
                ]
              }, undefined, true, undefined, this),
              (startedLabel || finishedLabel || elapsedLabel) && /* @__PURE__ */ jsxDEV("div", {
                className: "mt-3 rounded-lg border border-border/30 bg-background/55 px-2.5 py-2 text-[11px] text-muted-foreground",
                children: [
                  startedLabel && /* @__PURE__ */ jsxDEV("p", {
                    children: [
                      "Started: ",
                      startedLabel
                    ]
                  }, undefined, true, undefined, this),
                  finishedLabel && /* @__PURE__ */ jsxDEV("p", {
                    children: [
                      "Finished: ",
                      finishedLabel
                    ]
                  }, undefined, true, undefined, this),
                  elapsedLabel && /* @__PURE__ */ jsxDEV("p", {
                    children: [
                      "Duration: ",
                      elapsedLabel
                    ]
                  }, undefined, true, undefined, this)
                ]
              }, undefined, true, undefined, this),
              session.error && /* @__PURE__ */ jsxDEV("p", {
                className: "mt-3 text-xs leading-relaxed text-red-500",
                children: session.error
              }, undefined, false, undefined, this),
              session.launchCommandPreview && /* @__PURE__ */ jsxDEV("p", {
                className: "mt-3 truncate rounded-lg border border-border/30 bg-background/60 px-2.5 py-2 font-mono text-[11px] text-muted-foreground",
                children: session.launchCommandPreview
              }, undefined, false, undefined, this),
              session.artifacts.length > 0 && /* @__PURE__ */ jsxDEV("div", {
                className: "mt-3",
                children: [
                  /* @__PURE__ */ jsxDEV("p", {
                    className: "mb-1.5 text-[11px] font-medium uppercase tracking-[0.14em] text-primary/75",
                    children: "Deliverables"
                  }, undefined, false, undefined, this),
                  /* @__PURE__ */ jsxDEV("div", {
                    className: "flex items-center gap-2 text-[11px] text-primary/85",
                    children: [
                      /* @__PURE__ */ jsxDEV(FileOutput, {
                        className: "size-3.5"
                      }, undefined, false, undefined, this),
                      /* @__PURE__ */ jsxDEV("span", {
                        className: "truncate",
                        children: [
                          session.artifacts.slice(0, 2).map((artifact) => artifact.filename).join(" · "),
                          session.artifacts.length > 2 ? " · ..." : ""
                        ]
                      }, undefined, true, undefined, this)
                    ]
                  }, undefined, true, undefined, this)
                ]
              }, undefined, true, undefined, this),
              inspectionPaths.length > 0 && /* @__PURE__ */ jsxDEV("div", {
                className: "mt-3",
                children: [
                  /* @__PURE__ */ jsxDEV("p", {
                    className: "mb-1.5 text-[11px] font-medium uppercase tracking-[0.14em] text-muted-foreground/80",
                    children: "Inspected Paths"
                  }, undefined, false, undefined, this),
                  /* @__PURE__ */ jsxDEV("p", {
                    className: "break-all text-[11px] leading-relaxed text-muted-foreground",
                    children: [
                      inspectionPaths.slice(0, 2).join(" · "),
                      inspectionPaths.length > 2 ? " · ..." : ""
                    ]
                  }, undefined, true, undefined, this)
                ]
              }, undefined, true, undefined, this)
            ]
          }, session.sessionId, true, undefined, this);
        })
      }, undefined, false, undefined, this)
    ]
  }, undefined, true, undefined, this);
});
var TraceAccordion = React.memo(function TraceAccordion2({
  trace,
  runState,
  stats
}) {
  const [isOpen, setIsOpen] = React.useState(false);
  const [visibleCount, setVisibleCount] = React.useState(40);
  const stateTone = runState === "failed" ? "text-red-500" : runState === "cancelled" ? "text-amber-500" : runState === "completed" ? "text-emerald-500" : "text-cyan-500";
  const visibleTrace = isOpen ? trace.slice(0, visibleCount) : [];
  const hasHiddenTrace = visibleCount < trace.length;
  return /* @__PURE__ */ jsxDEV("details", {
    className: "w-full rounded-xl border border-border/30 bg-background/40 px-3 py-2",
    onToggle: (event) => {
      const nextOpen = event.currentTarget.open;
      setIsOpen(nextOpen);
      if (!nextOpen) {
        setVisibleCount(40);
      }
    },
    children: [
      /* @__PURE__ */ jsxDEV("summary", {
        className: "flex cursor-pointer list-none items-center justify-between gap-2 text-xs text-muted-foreground",
        children: [
          /* @__PURE__ */ jsxDEV("span", {
            className: "flex items-center gap-2",
            children: [
              /* @__PURE__ */ jsxDEV(ChevronDown, {
                className: "size-3.5"
              }, undefined, false, undefined, this),
              /* @__PURE__ */ jsxDEV("span", {
                className: `font-medium ${stateTone}`,
                children: [
                  "Runtime Trace ",
                  runState ? `(${runState})` : ""
                ]
              }, undefined, true, undefined, this),
              /* @__PURE__ */ jsxDEV("span", {
                children: [
                  "calls ",
                  stats.toolCalls
                ]
              }, undefined, true, undefined, this),
              /* @__PURE__ */ jsxDEV("span", {
                children: [
                  "retries ",
                  stats.retries
                ]
              }, undefined, true, undefined, this),
              /* @__PURE__ */ jsxDEV("span", {
                children: [
                  "errors ",
                  stats.errors
                ]
              }, undefined, true, undefined, this),
              /* @__PURE__ */ jsxDEV("span", {
                children: [
                  "approvals ",
                  stats.approvals
                ]
              }, undefined, true, undefined, this)
            ]
          }, undefined, true, undefined, this),
          /* @__PURE__ */ jsxDEV("span", {
            children: [
              stats.total,
              " events"
            ]
          }, undefined, true, undefined, this)
        ]
      }, undefined, true, undefined, this),
      /* @__PURE__ */ jsxDEV("div", {
        className: "mt-2 max-h-52 overflow-y-auto rounded-lg border border-border/20 bg-background/30 p-2 font-mono text-[11px]",
        children: !isOpen ? /* @__PURE__ */ jsxDEV("div", {
          className: "text-muted-foreground",
          children: [
            "Expand to inspect ",
            trace.length,
            " runtime event",
            trace.length === 1 ? "" : "s",
            "."
          ]
        }, undefined, true, undefined, this) : trace.length === 0 ? /* @__PURE__ */ jsxDEV("div", {
          className: "text-muted-foreground",
          children: "Waiting for runtime events..."
        }, undefined, false, undefined, this) : /* @__PURE__ */ jsxDEV(Fragment, {
          children: [
            visibleTrace.map((item) => /* @__PURE__ */ jsxDEV("div", {
              className: "mb-1.5 rounded-md border border-border/10 bg-background/40 p-2",
              children: [
                /* @__PURE__ */ jsxDEV("div", {
                  className: "flex items-center justify-between gap-2 text-muted-foreground",
                  children: [
                    /* @__PURE__ */ jsxDEV("span", {
                      className: "uppercase tracking-wide",
                      children: item.phase
                    }, undefined, false, undefined, this),
                    /* @__PURE__ */ jsxDEV("span", {
                      children: item.timestamp.toLocaleTimeString([], {
                        hour: "2-digit",
                        minute: "2-digit",
                        second: "2-digit"
                      })
                    }, undefined, false, undefined, this)
                  ]
                }, undefined, true, undefined, this),
                /* @__PURE__ */ jsxDEV("div", {
                  className: "mt-1 text-foreground",
                  children: item.label
                }, undefined, false, undefined, this),
                item.preview && /* @__PURE__ */ jsxDEV("div", {
                  className: "mt-1 line-clamp-2 text-muted-foreground",
                  children: item.preview
                }, undefined, false, undefined, this)
              ]
            }, item.id, true, undefined, this)),
            hasHiddenTrace && /* @__PURE__ */ jsxDEV("div", {
              className: "mt-2 flex justify-center",
              children: /* @__PURE__ */ jsxDEV(Button, {
                size: "sm",
                variant: "ghost",
                className: "h-7 px-2 text-xs text-muted-foreground hover:text-foreground",
                onClick: () => setVisibleCount((count) => Math.min(trace.length, count + 40)),
                children: [
                  "Show ",
                  Math.min(40, trace.length - visibleCount),
                  " more"
                ]
              }, undefined, true, undefined, this)
            }, undefined, false, undefined, this)
          ]
        }, undefined, true, undefined, this)
      }, undefined, false, undefined, this)
    ]
  }, undefined, true, undefined, this);
});
var PlanCard = React.memo(function PlanCard2({
  plan,
  onExecute,
  isExecuting
}) {
  return /* @__PURE__ */ jsxDEV(Card, {
    className: "w-full max-w-md md:max-w-lg lg:max-w-xl p-4 space-y-4 border-l-4 border-l-purple-500 bg-purple-50/50 dark:bg-purple-900/10",
    children: [
      /* @__PURE__ */ jsxDEV("div", {
        className: "flex items-center justify-between",
        children: [
          /* @__PURE__ */ jsxDEV("h3", {
            className: "font-medium text-sm flex items-center gap-2",
            children: [
              /* @__PURE__ */ jsxDEV(FileCode, {
                className: "size-4 text-purple-500"
              }, undefined, false, undefined, this),
              "Proposed Plan"
            ]
          }, undefined, true, undefined, this),
          /* @__PURE__ */ jsxDEV("span", {
            className: "text-xs text-muted-foreground bg-background/50 px-2 py-1 rounded",
            children: [
              plan.steps.length,
              " steps"
            ]
          }, undefined, true, undefined, this)
        ]
      }, undefined, true, undefined, this),
      /* @__PURE__ */ jsxDEV("div", {
        className: "space-y-2",
        children: plan.steps.map((step) => {
          const Icon = stepIcons[step.type] || stepIcons.default;
          return /* @__PURE__ */ jsxDEV("div", {
            className: "flex gap-3 items-start text-xs p-2 rounded bg-background/40 hover:bg-background/80 transition-colors border border-transparent hover:border-border/30",
            children: [
              /* @__PURE__ */ jsxDEV(Icon, {
                className: "size-3.5 mt-0.5 text-muted-foreground shrink-0"
              }, undefined, false, undefined, this),
              /* @__PURE__ */ jsxDEV("span", {
                className: "text-foreground/80",
                children: step.description
              }, undefined, false, undefined, this)
            ]
          }, `${step.type}-${step.description}`, true, undefined, this);
        })
      }, undefined, false, undefined, this),
      plan.warnings.length > 0 && /* @__PURE__ */ jsxDEV("div", {
        className: "bg-orange-500/10 text-orange-600 dark:text-orange-400 p-3 rounded-lg text-xs space-y-1",
        children: [
          /* @__PURE__ */ jsxDEV("p", {
            className: "font-semibold flex items-center gap-1",
            children: "⚠️ Warnings"
          }, undefined, false, undefined, this),
          /* @__PURE__ */ jsxDEV("ul", {
            className: "list-disc list-inside opacity-90",
            children: plan.warnings.map((w) => /* @__PURE__ */ jsxDEV("li", {
              children: w
            }, w, false, undefined, this))
          }, undefined, false, undefined, this)
        ]
      }, undefined, true, undefined, this),
      /* @__PURE__ */ jsxDEV("div", {
        className: "flex gap-2 pt-2",
        children: /* @__PURE__ */ jsxDEV(Button, {
          className: "flex-1 bg-purple-600 hover:bg-purple-700 text-white shadow-lg shadow-purple-500/20",
          size: "sm",
          onClick: () => onExecute?.(plan.id),
          disabled: isExecuting,
          children: [
            /* @__PURE__ */ jsxDEV(Play, {
              className: "size-3.5"
            }, undefined, false, undefined, this),
            "Execute Plan"
          ]
        }, undefined, true, undefined, this)
      }, undefined, false, undefined, this)
    ]
  }, undefined, true, undefined, this);
});
var AIRLOCK_BADGE_CONFIG = {
  0: { label: "L0 Safe", className: "border-emerald-500/30 text-emerald-500 bg-emerald-500/10" },
  1: { label: "L1 Notifying", className: "border-amber-500/30 text-amber-500 bg-amber-500/10" },
  2: { label: "L2 Approval", className: "border-red-500/30 text-red-500 bg-red-500/10" }
};
var NeuralStatus = React.memo(({
  state,
  toolName,
  airlockLevel
}) => {
  const config = useMemo(() => getNeuralStateConfig(state), [state]);
  const badge = airlockLevel !== undefined ? AIRLOCK_BADGE_CONFIG[airlockLevel] ?? AIRLOCK_BADGE_CONFIG[0] : undefined;
  const Icon = config.icon;
  return /* @__PURE__ */ jsxDEV("div", {
    className: `flex items-center gap-3 py-1 ${config.color}`,
    children: [
      /* @__PURE__ */ jsxDEV("div", {
        className: `p-2 rounded-full ${config.bgColor}`,
        children: /* @__PURE__ */ jsxDEV(Icon, {
          className: "size-4 animate-pulse"
        }, undefined, false, undefined, this)
      }, undefined, false, undefined, this),
      /* @__PURE__ */ jsxDEV("div", {
        className: "flex flex-col gap-0.5",
        children: [
          /* @__PURE__ */ jsxDEV("span", {
            className: "text-sm font-medium",
            children: toolName || config.text
          }, undefined, false, undefined, this),
          /* @__PURE__ */ jsxDEV("div", {
            className: "flex gap-0.5 h-1 mt-1 overflow-hidden rounded-full w-16",
            children: [0, 1, 2, 3].map((barIndex) => /* @__PURE__ */ jsxDEV("div", {
              className: `flex-1 h-full rounded-full ${config.bgColor} animate-[neural-bar_1.2s_ease-in-out_infinite]`,
              style: { animationDelay: `${barIndex * 0.15}s` }
            }, `neural-bar-${barIndex}`, false, undefined, this))
          }, undefined, false, undefined, this)
        ]
      }, undefined, true, undefined, this),
      badge && /* @__PURE__ */ jsxDEV("span", {
        className: `ml-auto rounded-full border px-2 py-0.5 text-[9px] uppercase tracking-[0.14em] animate-in zoom-in-95 duration-300 ${badge.className}`,
        children: badge.label
      }, airlockLevel, false, undefined, this)
    ]
  }, undefined, true, undefined, this);
});
export {
  MessageBubble,
  MessageBubble as MemoizedMessageBubble
};
