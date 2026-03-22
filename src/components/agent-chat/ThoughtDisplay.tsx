import React, { useState, useEffect, useRef } from "react";
import { ChevronDown, Brain, Clock } from "lucide-react";
import { Button } from "@heroui/react";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "../ui/collapsible";

interface ThoughtDisplayProps {
  thought: string;
  thinkingLevel?: "minimal" | "low" | "medium" | "high";
  modelName?: string;
  className?: string;
  isStreaming?: boolean;
  durationMs?: number;
}

const LEVEL_COLORS: Record<string, string> = {
  minimal: "text-slate-500",
  low: "text-blue-500",
  medium: "text-amber-500",
  high: "text-purple-500",
};

const BG_COLORS: Record<string, string> = {
  minimal: "bg-slate-500",
  low: "bg-blue-500",
  medium: "bg-amber-500",
  high: "bg-purple-500",
};

/** Hook that tracks elapsed ms while active, updating every 100ms via interval callback. */
function useElapsedTimer(active: boolean): number {
  const [elapsed, setElapsed] = useState(0);
  const startRef = useRef<number>(0);

  useEffect(() => {
    if (!active) return;
    const start = performance.now();
    startRef.current = start;
    const interval = setInterval(() => {
      setElapsed(Math.round(performance.now() - start));
    }, 100);
    return () => clearInterval(interval);
  }, [active]);

  return elapsed;
}

export const ThoughtDisplay = React.memo(function ThoughtDisplay({
  thought,
  thinkingLevel = "medium",
  modelName,
  className,
  isStreaming = false,
  durationMs,
}: ThoughtDisplayProps) {
  // User can toggle; streaming auto-opens via derived initial state
  const [userClosed, setUserClosed] = useState(false);
  const isExpanded = isStreaming ? !userClosed : false;
  const elapsed = useElapsedTimer(isStreaming);

  const handleOpenChange = (open: boolean) => {
    setUserClosed(!open);
  };

  const displayTime = durationMs || (isStreaming ? elapsed : null);

  const currentLevelColor = LEVEL_COLORS[thinkingLevel] || LEVEL_COLORS.medium;
  const currentBgColor = BG_COLORS[thinkingLevel] || BG_COLORS.medium;
  const headerTitle = modelName ? `${modelName} Thinking` : "Thinking Process";

  return (
    <Collapsible
      open={isExpanded}
      onOpenChange={handleOpenChange}
      className={`w-full font-sans ${className}`}
    >
      <div className="flex items-center gap-2 group select-none py-2">
        <CollapsibleTrigger>
          <div className={`p-1 rounded-md bg-opacity-10 ${currentBgColor} transition-colors cursor-pointer`}>
            <Brain className={`size-3.5 ${currentLevelColor}`} />
          </div>
        </CollapsibleTrigger>

        <CollapsibleTrigger>
          <span className="text-sm font-medium text-foreground/80 group-hover:text-foreground transition-colors cursor-pointer block">
            {headerTitle}
          </span>
        </CollapsibleTrigger>

        {displayTime != null && displayTime > 0 && (
          <span className="text-xs text-muted-foreground font-mono flex items-center gap-1 ml-1 bg-muted/30 px-1.5 py-0.5 rounded">
            <Clock className="size-3" />
            {(displayTime / 1000).toFixed(1)}s
          </span>
        )}

        <div className="flex-1" />

        <CollapsibleTrigger>
          <Button
            variant="ghost"
            className="w-6 h-6 min-w-0 hover:bg-muted/50 text-muted-foreground cursor-pointer"
          >
            <ChevronDown
              className={`size-3.5 transition-transform duration-200 ${isExpanded ? "rotate-180" : ""}`}
            />
          </Button>
        </CollapsibleTrigger>
      </div>

      <CollapsibleContent className="CollapsibleContent overflow-hidden data-[state=closed]:animate-collapsible-up data-[state=open]:animate-collapsible-down">
        <div className="pl-2 border-l-2 border-muted/30 ml-2.5 my-1">
          <div className="pl-4 py-2 text-sm text-muted-foreground/90 whitespace-pre-wrap leading-relaxed font-mono bg-muted/5 rounded-r-lg">
            {thought}
            {isStreaming && (
              <span className="inline-block w-1.5 h-3.5 bg-current ml-1 animate-pulse align-middle" />
            )}
          </div>
        </div>
      </CollapsibleContent>
    </Collapsible>
  );
});

export const ThoughtBadge = React.memo(function ThoughtBadge({
  thinkingLevel = "medium",
}: {
  thinkingLevel?: string;
}) {
  const currentLevelColor = LEVEL_COLORS[thinkingLevel] || LEVEL_COLORS.medium;
  const currentBgColor = BG_COLORS[thinkingLevel] || BG_COLORS.medium;

  return (
    <div className={`inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full bg-opacity-10 ${currentBgColor}`}>
      <Brain className={`size-3 ${currentLevelColor}`} />
      <span className={`text-[10px] font-medium ${currentLevelColor}`}>
        Thinking
      </span>
    </div>
  );
});
