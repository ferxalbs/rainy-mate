import React from "react";
import { Card, Button, Chip } from "@heroui/react";
import { FileCode, Play, FileText, FolderOpen, Search } from "lucide-react";

interface PlanConfirmationCardProps {
  toolCalls: Array<{
    skill: string;
    method: string;
    params: Record<string, any>;
  }>;
  onExecute: () => void;
  isExecuting?: boolean;
}

const methodIcons: Record<string, React.ElementType> = {
  write_file: FileCode,
  append_file: FileCode,
  read_file: FileText,
  list_files: FolderOpen,
  search_files: Search,
  default: FileCode,
};

const methodColors: Record<string, string> = {
  write_file: "text-purple-400 bg-purple-400/10 border-purple-400/20",
  append_file: "text-green-400 bg-green-400/10 border-green-400/20",
  read_file: "text-blue-400 bg-blue-400/10 border-blue-400/20",
  list_files: "text-yellow-400 bg-yellow-400/10 border-yellow-400/20",
  search_files: "text-pink-400 bg-pink-400/10 border-pink-400/20",
  default: "text-gray-400 bg-gray-400/10 border-gray-400/20",
};

export function PlanConfirmationCard({
  toolCalls,
  onExecute,
  isExecuting,
}: PlanConfirmationCardProps) {
  if (!toolCalls || toolCalls.length === 0) return null;

  return (
    <Card className="w-full max-w-md p-4 space-y-4 border-l-4 border-l-purple-500 bg-purple-50/50 dark:bg-purple-900/10 mt-4">
      <div className="flex items-center justify-between">
        <h3 className="font-medium text-sm flex items-center gap-2">
          <Play className="size-4 text-purple-500" />
          Proposed Actions
        </h3>
        <Chip size="sm" variant="soft" color="warning">
          {toolCalls.length} operation{toolCalls.length !== 1 ? "s" : ""}
        </Chip>
      </div>

      <div className="space-y-2 max-h-60 overflow-y-auto pr-1">
        {toolCalls.map((call, idx) => {
          const Icon = methodIcons[call.method] || methodIcons.default;
          const colorClass = methodColors[call.method] || methodColors.default;

          return (
            <div
              key={idx}
              className={`flex gap-3 items-start text-xs p-2.5 rounded-lg border ${colorClass} transition-all`}
            >
              <Icon className="size-4 mt-0.5 shrink-0" />
              <div className="flex flex-col gap-0.5 overflow-hidden">
                <span className="font-semibold font-mono text-[11px] uppercase opacity-70">
                  {call.method.replace("_", " ")}
                </span>
                <span className="truncate font-mono" title={call.params.path}>
                  {call.params.path || call.params.query || "unknown"}
                </span>
              </div>
            </div>
          );
        })}
      </div>

      <div className="flex gap-2 pt-2">
        <Button
          className="flex-1 bg-purple-600 hover:bg-purple-700 text-white shadow-lg shadow-purple-500/20"
          size="sm"
          onPress={onExecute}
          isDisabled={isExecuting}
          isPending={isExecuting}
        >
          <Play className="size-3.5 fill-current" />
          Execute Plan
        </Button>
      </div>
    </Card>
  );
}
