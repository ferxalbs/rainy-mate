import { useState } from "react";
import { Lightbulb, ChevronDown, ChevronUp, Brain } from "lucide-react";
import { Button } from "@heroui/react";

interface ThoughtDisplayProps {
  thought: string;
  thinkingLevel?: "minimal" | "low" | "medium" | "high";
  modelName?: string;
  className?: string;
}

/**
 * Enterprise Thought Display Component
 * Shows AI reasoning/thinking process in an elegant, collapsible format
 * Designed for production-grade applications with thinking-capable models
 */
export function ThoughtDisplay({
  thought,
  thinkingLevel = "medium",
  modelName,
  className,
}: ThoughtDisplayProps) {
  const [isExpanded, setIsExpanded] = useState(false);

  // Map thinking level to display attributes
  const levelConfig = {
    minimal: { color: "text-gray-500", bg: "bg-gray-500/5", label: "Quick" },
    low: { color: "text-blue-500", bg: "bg-blue-500/5", label: "Light" },
    medium: { color: "text-amber-500", bg: "bg-amber-500/5", label: "Standard" },
    high: { color: "text-purple-500", bg: "bg-purple-500/5", label: "Deep" },
  };

  const config = levelConfig[thinkingLevel];

  return (
    <div className={`w-full ${className}`}>
      {/* Thought Header */}
      <div
        className={`flex items-center justify-between p-3 rounded-lg border ${config.bg} border-opacity-20 cursor-pointer transition-all hover:bg-opacity-10`}
        style={{ borderColor: "currentColor" }}
        onClick={() => setIsExpanded(!isExpanded)}
      >
        <div className="flex items-center gap-2">
          <div className={`${config.color}`}>
            <Brain className="size-4" />
          </div>
          <div className="flex flex-col">
            <span className={`text-xs font-medium ${config.color} flex items-center gap-1`}>
              <Lightbulb className="size-3" />
              AI Reasoning Process
            </span>
            <span className="text-[10px] text-muted-foreground">
              {modelName && `${modelName} · `}
              {config.label} thinking level
            </span>
          </div>
        </div>
        <Button
          size="sm"
          variant="ghost"
          className={`${config.color} hover:${config.bg} h-6 px-2`}
          onPress={() => setIsExpanded(!isExpanded)}
        >
          {isExpanded ? (
            <ChevronUp className="size-3" />
          ) : (
            <ChevronDown className="size-3" />
          )}
        </Button>
      </div>

      {/* Expanded Thought Content */}
      {isExpanded && (
        <div
          className={`mt-2 p-4 rounded-lg ${config.bg} border border-opacity-10 text-sm leading-relaxed`}
          style={{ borderColor: "currentColor" }}
        >
          <div className={`${config.color} font-medium text-xs mb-2 uppercase tracking-wide`}>
            Reasoning Chain
          </div>
          <div className="text-muted-foreground whitespace-pre-wrap font-mono text-xs">
            {thought}
          </div>
          
          {/* Thought Signature */}
          <div className="mt-4 pt-3 border-t border-opacity-10 flex items-center justify-between">
            <span className="text-[10px] text-muted-foreground">
              Powered by advanced reasoning
            </span>
            <div className="flex items-center gap-1">
              <div className={`w-2 h-2 rounded-full ${config.color} bg-current animate-pulse`} />
              <span className={`text-[10px] ${config.color}`}>
                {thinkingLevel.charAt(0).toUpperCase() + thinkingLevel.slice(1)} reasoning
              </span>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

/**
 * Compact Thought Badge - for inline display
 */
export function ThoughtBadge({
  thinkingLevel = "medium",
  onClick,
}: {
  thinkingLevel?: "minimal" | "low" | "medium" | "high";
  onClick?: () => void;
}) {
  const levelConfig = {
    minimal: { color: "text-gray-500", bg: "bg-gray-500/10", label: "Quick" },
    low: { color: "text-blue-500", bg: "bg-blue-500/10", label: "Light" },
    medium: { color: "text-amber-500", bg: "bg-amber-500/10", label: "Reasoning" },
    high: { color: "text-purple-500", bg: "bg-purple-500/10", label: "Deep" },
  };

  const config = levelConfig[thinkingLevel];

  return (
    <button
      onClick={onClick}
      className={`inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-medium ${config.color} ${config.bg} hover:opacity-80 transition-opacity`}
    >
      <Brain className="size-3" />
      {config.label}
    </button>
  );
}
