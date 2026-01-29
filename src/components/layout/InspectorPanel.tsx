import { Button, Tooltip, ScrollShadow, Separator } from "@heroui/react";
import {
  X,
  FileText,
  Info,
  Settings,
  Activity,
  ExternalLink,
  ChevronRight,
  ChevronLeft,
  Eye,
} from "lucide-react";
import { MarkdownRenderer } from "../shared/MarkdownRenderer";

interface InspectorPanelProps {
  isOpen: boolean;
  onClose: () => void;
  title?: string;
  type?: "preview" | "info" | "process" | "links";
  content?: string;
  filename?: string;
}

export function InspectorPanel({
  isOpen,
  onClose,
  title = "Inspector",
  type = "preview",
  content,
  filename,
}: InspectorPanelProps) {
  if (!isOpen)
    return (
      <div className="w-12 h-screen border-l border-border/50 bg-sidebar flex flex-col items-center py-4 shrink-0 transition-all">
        <Tooltip delay={0}>
          <Button variant="ghost" size="sm" isIconOnly onPress={onClose}>
            <ChevronLeft className="size-4" />
          </Button>
          <Tooltip.Content placement="left">Open Inspector</Tooltip.Content>
        </Tooltip>

        <div className="mt-8 space-y-4 flex flex-col items-center">
          <Eye className="size-5 text-muted-foreground opacity-40" />
          <Activity className="size-5 text-muted-foreground opacity-40" />
          <Info className="size-5 text-muted-foreground opacity-40" />
        </div>
      </div>
    );

  return (
    <aside className="w-80 h-full border-l border-border/50 bg-sidebar flex flex-col shrink-0 animate-in slide-in-from-right duration-300">
      {/* Header */}
      <div className="h-14 px-4 flex items-center justify-between border-b border-border/50">
        <div className="flex items-center gap-2">
          {type === "preview" && <Eye className="size-4 text-primary" />}
          {type === "info" && <Info className="size-4 text-blue-500" />}
          {type === "process" && (
            <Activity className="size-4 text-orange-500" />
          )}
          {type === "links" && (
            <ExternalLink className="size-4 text-green-500" />
          )}
          <span className="font-semibold text-sm truncate">{title}</span>
        </div>
        <Button variant="ghost" size="sm" isIconOnly onPress={onClose}>
          <X className="size-4" />
        </Button>
      </div>

      {/* Content */}
      <ScrollShadow className="flex-1 p-4">
        {type === "preview" && content ? (
          <div className="space-y-4">
            {filename && (
              <div className="flex items-center gap-2 p-2 rounded-lg bg-muted/30 border border-border/50">
                <FileText className="size-4 text-primary" />
                <span className="text-xs font-medium truncate">{filename}</span>
              </div>
            )}
            <div className="prose prose-sm dark:prose-invert max-w-none">
              <MarkdownRenderer content={content} />
            </div>
          </div>
        ) : type === "info" ? (
          <div className="space-y-6">
            <div className="space-y-2">
              <h3 className="text-xs font-bold uppercase tracking-wider text-muted-foreground">
                General Info
              </h3>
              <p className="text-sm text-foreground">Rainy Cowork v0.5.3</p>
              <p className="text-xs text-muted-foreground">
                Connected to local workspace.
              </p>
            </div>

            <Separator />

            <div className="space-y-3">
              <h3 className="text-xs font-bold uppercase tracking-wider text-muted-foreground">
                System Status
              </h3>
              <div className="flex items-center justify-between text-xs">
                <span>Memory usage</span>
                <span className="text-green-500 font-medium">Optimal</span>
              </div>
              <div className="flex items-center justify-between text-xs">
                <span>Tauri Bridge</span>
                <span className="text-green-500 font-medium">Active</span>
              </div>
            </div>
          </div>
        ) : (
          <div className="flex flex-col items-center justify-center h-48 text-muted-foreground space-y-2">
            <Settings className="size-8 opacity-20" />
            <p className="text-xs">No active {type} content</p>
          </div>
        )}
      </ScrollShadow>

      {/* Footer */}
      <div className="p-4 border-t border-border/50 bg-muted/5">
        <Button variant="outline" size="sm" className="w-full justify-between">
          <span className="text-xs">View Full Details</span>
          <ChevronRight className="size-3" />
        </Button>
      </div>
    </aside>
  );
}
