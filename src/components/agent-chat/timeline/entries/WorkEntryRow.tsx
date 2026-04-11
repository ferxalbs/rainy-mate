import { useState } from "react";
import {
  AlertCircle,
  Bot,
  ChevronDown,
  ChevronRight,
  LoaderCircle,
  Sparkles,
  TerminalSquare,
} from "lucide-react";

import type { TimelineWorkEntry } from "../MessagesTimeline.logic";
import { cn } from "../../../../lib/utils";
import type { AgentMessage } from "../../../../types/agent";

interface WorkEntryRowProps {
  entries: TimelineWorkEntry[];
  message?: AgentMessage;
  defaultExpanded?: boolean;
}

function phaseMeta(message?: AgentMessage): { label: string; detail?: string } | null {
  switch (message?.runPhase) {
    case "starting":
      return { label: "Session boot", detail: message.statusText };
    case "planning":
      return { label: "Planning", detail: message.statusText };
    case "streaming":
      return { label: "Streaming draft", detail: message.statusText };
    case "awaiting_approval":
      return { label: "Awaiting approval", detail: message.statusText };
    case "tool_waiting":
      return { label: message.activeToolName || "Tool queued", detail: message.statusText };
    case "tool_running":
      return { label: message.activeToolName || "Tool execution", detail: message.statusText };
    case "responding":
      return { label: "Finalizing response", detail: message.statusText };
    default:
      return null;
  }
}

function toneIcon(entry: TimelineWorkEntry) {
  if (entry.status === "running") {
    return <LoaderCircle className="size-3.5 animate-spin text-primary" />;
  }
  if (entry.tone === "thinking") {
    return <Bot className="size-3.5 text-primary" />;
  }
  if (entry.tone === "tool") {
    return <TerminalSquare className="size-3.5 text-foreground/75" />;
  }
  if (entry.tone === "error") {
    return <AlertCircle className="size-3.5 text-destructive" />;
  }
  return <Sparkles className="size-3.5 text-muted-foreground" />;
}

export function WorkEntryRow({
  entries,
  message,
  defaultExpanded = false,
}: WorkEntryRowProps) {
  const [isExpanded, setIsExpanded] = useState(defaultExpanded);

  if (!entries.length) {
    return null;
  }

  const runningCount = entries.filter((entry) => entry.status === "running").length;
  const failedCount = entries.filter((entry) => entry.status === "failed").length;
  const phase = phaseMeta(message);
  const headerLabel =
    phase?.label || (runningCount > 0 ? "Working" : failedCount > 0 ? "Attention required" : "Work log");

  return (
    <div className="mb-5 flex w-full justify-start">
      <div className="w-full max-w-[min(100%,42rem)] rounded-[24px] border border-border/70 bg-[linear-gradient(180deg,color-mix(in_srgb,var(--card)_86%,transparent),color-mix(in_srgb,var(--background)_72%,transparent))] px-3.5 py-3.5 shadow-[0_28px_80px_-64px_rgba(0,0,0,0.85)] backdrop-blur-xl">
        <div className="flex items-center justify-between gap-3">
          <div className="flex min-w-0 items-center gap-2">
            <span className="rounded-full border border-border/60 bg-background/70 px-2 py-1 text-[10px] font-semibold uppercase tracking-[0.16em] text-muted-foreground">
              {headerLabel}
            </span>
            <span className="text-xs text-muted-foreground">
              {entries.length} event{entries.length === 1 ? "" : "s"}
            </span>
            {runningCount > 0 ? (
              <span className="text-[11px] text-primary">{runningCount} active</span>
            ) : null}
          </div>
          <button
            type="button"
            onClick={() => setIsExpanded((current) => !current)}
            className="inline-flex shrink-0 items-center gap-1 rounded-full px-2 py-1 text-[11px] text-muted-foreground transition-colors hover:bg-muted/70 hover:text-foreground"
          >
            {isExpanded ? "Hide" : "Inspect"}
            {isExpanded ? <ChevronDown className="size-3.5" /> : <ChevronRight className="size-3.5" />}
          </button>
        </div>

        {phase?.detail ? (
          <div className="mt-2 px-0.5 text-[12px] text-muted-foreground/78">{phase.detail}</div>
        ) : null}

        {isExpanded ? (
          <div className="mt-3 space-y-2">
            {entries.map((entry) => (
              <div
                key={entry.id}
                className="rounded-2xl border border-border/60 bg-background/72 px-3 py-2.5"
              >
                <div className="flex items-start gap-2.5">
                  <div className="mt-0.5 shrink-0">{toneIcon(entry)}</div>
                  <div className="min-w-0 flex-1">
                    <div className="flex flex-wrap items-center gap-2">
                      {entry.command ? (
                        <span className="rounded-md border border-border/60 bg-card px-1.5 py-0.5 font-mono text-[10px] text-foreground/85">
                          {entry.command}
                        </span>
                      ) : null}
                      <span
                        className={cn(
                          "text-[10px] uppercase tracking-[0.14em]",
                          entry.status === "failed"
                            ? "text-destructive"
                            : entry.status === "running"
                              ? "text-primary"
                              : "text-muted-foreground/70",
                        )}
                      >
                        {entry.status}
                      </span>
                    </div>
                    <div
                      className={cn(
                        "mt-1 whitespace-pre-wrap break-words text-[13px] leading-relaxed",
                        entry.tone === "thinking" ? "text-foreground/78 italic" : "text-foreground/88",
                      )}
                    >
                      {entry.detail}
                    </div>
                    {entry.rawCommand ? (
                      <pre className="mt-2 overflow-x-auto rounded-xl border border-border/60 bg-card/80 px-3 py-2 text-[11px] leading-relaxed text-muted-foreground">
                        {entry.rawCommand}
                      </pre>
                    ) : null}
                  </div>
                </div>
              </div>
            ))}
          </div>
        ) : (
          <div className="mt-3 flex flex-wrap gap-2">
            {entries.slice(0, 4).map((entry) => (
              <div
                key={entry.id}
                className="inline-flex max-w-full items-center gap-2 rounded-full border border-border/60 bg-background/72 px-2.5 py-1 text-[11px] text-muted-foreground"
              >
                {toneIcon(entry)}
                <span className="truncate">{entry.command || entry.detail}</span>
              </div>
            ))}
            {entries.length > 4 ? (
              <div className="inline-flex items-center rounded-full border border-border/60 bg-background/72 px-2.5 py-1 text-[11px] text-muted-foreground">
                +{entries.length - 4} more
              </div>
            ) : null}
          </div>
        )}
      </div>
    </div>
  );
}
