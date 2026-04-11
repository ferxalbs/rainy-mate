import {
  ChevronDown,
  Compass,
  FolderPlus,
  RefreshCw,
  Rows3,
  Settings2,
  SquarePen,
} from "lucide-react";

import type { ChatSession } from "../../services/tauri";
import type { Folder } from "../../types";
import { cn } from "../../lib/utils";
import { AnimatedThemeToggler } from "../ui/animated-theme-toggler";
import { Button } from "../ui/button";
import { Popover, PopoverContent, PopoverTrigger } from "../ui/popover";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "../ui/tooltip";

interface ChatTopbarProps {
  chatSession: ChatSession | null;
  titleStatus: "idle" | "generating" | "ready" | "fallback";
  workspacePath: string;
  folders: Folder[];
  activeFolderId?: string;
  onSelectFolder?: (folder: Folder) => void;
  onAddFolder?: () => void;
  onNewChat: () => void;
  onRefreshChat: () => void;
  onOpenSettings?: () => void;
  showTelemetryChips?: boolean;
  onToggleTelemetryChips?: () => void;
}

function getWorkspaceName(path: string): string {
  return path.split("/").filter(Boolean).pop() || "workspace";
}

function resolveTitle(chatSession: ChatSession | null, titleStatus: ChatTopbarProps["titleStatus"]): string {
  const title = chatSession?.title?.trim();
  if (title) return title;
  if (titleStatus === "generating") return "Generating title";
  return "New thread";
}

export function ChatTopbar({
  chatSession,
  titleStatus,
  workspacePath,
  folders,
  activeFolderId,
  onSelectFolder,
  onAddFolder,
  onNewChat,
  onRefreshChat,
  onOpenSettings,
  showTelemetryChips = false,
  onToggleTelemetryChips,
}: ChatTopbarProps) {
  const workspaceName = getWorkspaceName(workspacePath);
  const title = resolveTitle(chatSession, titleStatus);

  return (
    <div className="pointer-events-none absolute inset-x-0 top-0 z-40 px-4 pt-3 md:px-6">
      <div data-tauri-drag-region className="absolute inset-x-0 top-0 h-16" />

      <div className="pointer-events-auto mx-auto flex w-full max-w-6xl items-center justify-between gap-3 rounded-full border border-white/8 bg-[linear-gradient(180deg,rgba(9,13,20,0.82),rgba(9,13,20,0.66))] px-3 py-1.5 shadow-[0_18px_60px_-38px_rgba(0,0,0,0.45)] backdrop-blur-2xl">
        <div className="flex min-w-0 items-center gap-2">
          <div className="flex size-8 shrink-0 items-center justify-center rounded-full border border-white/8 bg-white/[0.03] text-primary">
            <Compass className="size-3.5" />
          </div>
          <div className="min-w-0">
            <div className="flex min-w-0 items-center gap-2">
              <span className="truncate text-sm font-medium text-foreground">{title}</span>
              <span className="hidden rounded-full border border-white/8 bg-background/24 px-2 py-0.5 text-[10px] font-medium uppercase tracking-[0.14em] text-muted-foreground sm:inline-flex">
                {titleStatus === "generating" ? "titling" : "session"}
              </span>
            </div>
          </div>

          <Popover>
            <PopoverTrigger
              render={
                <button
                  type="button"
                  className="ml-2 hidden min-w-0 items-center gap-1 rounded-full border border-transparent bg-transparent px-2.5 py-1 text-xs text-muted-foreground transition-colors hover:border-white/8 hover:bg-background/24 hover:text-foreground md:flex"
                />
              }
            >
              <span className="truncate uppercase tracking-[0.12em]">{workspaceName}</span>
              <ChevronDown className="size-3.5" />
            </PopoverTrigger>
            <PopoverContent
              align="start"
              sideOffset={12}
              className="w-[320px] overflow-hidden rounded-3xl border border-border/70 bg-popover/94 p-1.5 shadow-2xl backdrop-blur-xl"
            >
              <div className="px-3 pb-2 pt-1">
                <div className="text-[10px] font-semibold uppercase tracking-[0.18em] text-muted-foreground/70">
                  Workspaces
                </div>
              </div>
              <div className="space-y-1">
                {folders.map((folder) => {
                  const isActive = folder.id === activeFolderId;
                  return (
                    <button
                      key={folder.id}
                      type="button"
                      onClick={() => onSelectFolder?.(folder)}
                      className={cn(
                        "flex w-full items-start gap-3 rounded-2xl px-3 py-2.5 text-left transition-colors",
                        isActive
                          ? "bg-primary/10 text-foreground"
                          : "text-muted-foreground hover:bg-muted/60 hover:text-foreground",
                      )}
                    >
                      <div className="mt-0.5 size-2.5 shrink-0 rounded-full bg-primary/80" />
                      <div className="min-w-0">
                        <div className="truncate text-sm font-medium">{folder.name}</div>
                        <div className="truncate text-[11px] text-muted-foreground/80">{folder.path}</div>
                      </div>
                    </button>
                  );
                })}
              </div>
              <div className="px-1 pt-2">
                <Button
                  variant="ghost"
                  className="w-full justify-start rounded-2xl text-muted-foreground hover:bg-muted/70 hover:text-foreground"
                  onClick={onAddFolder}
                >
                  <FolderPlus className="size-4" />
                  Add workspace
                </Button>
              </div>
            </PopoverContent>
          </Popover>
        </div>

        <div className="flex items-center gap-1">
          <AnimatedThemeToggler />
          <div className="mx-1 hidden h-4 w-px bg-white/8 sm:block" />
          <TooltipProvider delay={0}>
            <Tooltip>
              <TooltipTrigger
                render={
                  <button
                    type="button"
                    className={cn(
                      "rounded-full p-2 text-muted-foreground transition-colors hover:bg-background/24 hover:text-foreground",
                      showTelemetryChips && "bg-background/28 text-foreground",
                    )}
                    onClick={onToggleTelemetryChips}
                  />
                }
              >
                <Rows3 className="size-4" />
              </TooltipTrigger>
              <TooltipContent>{showTelemetryChips ? "Hide telemetry chips" : "Show telemetry chips"}</TooltipContent>
            </Tooltip>
            <Tooltip>
              <TooltipTrigger
                render={
                  <button
                    type="button"
                    className="rounded-full p-2 text-muted-foreground transition-colors hover:bg-background/24 hover:text-foreground"
                    onClick={onRefreshChat}
                  />
                }
              >
                <RefreshCw className="size-4" />
              </TooltipTrigger>
              <TooltipContent>Refresh active chat</TooltipContent>
            </Tooltip>
            <Tooltip>
              <TooltipTrigger
                render={
                  <button
                    type="button"
                    className="rounded-full p-2 text-muted-foreground transition-colors hover:bg-background/24 hover:text-foreground"
                    onClick={onNewChat}
                  />
                }
              >
                <SquarePen className="size-4" />
              </TooltipTrigger>
              <TooltipContent>New chat</TooltipContent>
            </Tooltip>
          </TooltipProvider>
          {onOpenSettings ? (
            <button
              type="button"
              className="ml-1 rounded-full p-2 text-muted-foreground transition-colors hover:bg-background/24 hover:text-foreground"
              onClick={onOpenSettings}
            >
              <Settings2 className="size-4" />
            </button>
          ) : null}
        </div>
      </div>
    </div>
  );
}
