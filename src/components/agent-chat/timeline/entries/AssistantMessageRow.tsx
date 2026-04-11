import { AlertTriangle, Clock3, LoaderCircle } from "lucide-react";

import type { AgentMessage } from "../../../../types/agent";
import { cn } from "../../../../lib/utils";
import { ArtifactBadgeRow } from "../../ArtifactBadgeRow";
import { MarkdownRenderer } from "../../MarkdownRenderer";

interface AssistantMessageRowProps {
  message: AgentMessage;
}

function statusCopy(message: AgentMessage): { label: string; tone: string; icon: typeof LoaderCircle } {
  if (message.runState === "failed") {
    return { label: "Failed", tone: "text-destructive", icon: AlertTriangle };
  }
  if (message.runState === "cancelled") {
    return { label: "Cancelled", tone: "text-muted-foreground", icon: Clock3 };
  }
  if (message.runState === "running") {
    switch (message.runPhase) {
      case "starting":
        return { label: "Starting", tone: "text-primary", icon: LoaderCircle };
      case "planning":
        return { label: "Planning", tone: "text-primary", icon: LoaderCircle };
      case "awaiting_approval":
        return { label: "Awaiting approval", tone: "text-amber-400", icon: LoaderCircle };
      case "tool_waiting":
        return { label: "Preparing tools", tone: "text-orange-300", icon: LoaderCircle };
      case "tool_running":
        return { label: "Using tools", tone: "text-primary", icon: LoaderCircle };
      case "responding":
        return { label: "Continuing response", tone: "text-primary", icon: LoaderCircle };
      default:
        return { label: "Streaming", tone: "text-primary", icon: LoaderCircle };
    }
  }
  return { label: "Delivered", tone: "text-muted-foreground", icon: Clock3 };
}

export function AssistantMessageRow({ message }: AssistantMessageRowProps) {
  const isStreaming = message.runState === "running";
  const status = statusCopy(message);
  const StatusIcon = status.icon;
  const hasBody = Boolean(message.content?.trim());

  return (
    <div className="mb-6 flex w-full justify-start">
      <div className="w-full max-w-[min(100%,56rem)]">
        <div className="mb-2 flex flex-wrap items-center gap-2 px-1">
          <span className={cn("inline-flex items-center gap-1 text-[11px]", status.tone)}>
            <StatusIcon className={cn("size-3", isStreaming && "animate-spin")} />
            {status.label}
          </span>
          {message.statusText && message.runState === "running" ? (
            <span className="text-[11px] text-muted-foreground/78">{message.statusText}</span>
          ) : null}
          {message.modelUsed?.name ? (
            <span className="rounded-full border border-border/60 bg-background/70 px-2 py-0.5 text-[10px] uppercase tracking-[0.14em] text-muted-foreground">
              {message.modelUsed.name}
            </span>
          ) : null}
        </div>

        <div className="max-w-[min(100%,46rem)] rounded-[22px] border border-border/70 bg-card/78 px-5 py-4 shadow-[0_24px_80px_-64px_rgba(0,0,0,0.8)] backdrop-blur-xl">
          {hasBody ? (
            <MarkdownRenderer content={message.content} tone="assistant" isStreaming={isStreaming} />
          ) : (
            <div className="flex items-center gap-2 text-sm text-muted-foreground/70">
              <LoaderCircle className={cn("size-4", isStreaming && "animate-spin")} />
              Awaiting assistant output
            </div>
          )}

          {message.artifacts?.length ? (
            <div className="mt-4 border-t border-border/60 pt-4">
              <ArtifactBadgeRow artifacts={message.artifacts} />
            </div>
          ) : null}
        </div>

        <div className="mt-1.5 px-1 text-[10px] text-muted-foreground/40">
          {message.timestamp.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}
        </div>
      </div>
    </div>
  );
}
