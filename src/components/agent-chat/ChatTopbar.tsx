import {
  ChevronDown,
  Compass,
  FolderPlus,
  RefreshCw,
  Settings2,
  SquarePen,
} from "lucide-react";

import type { ChatSession } from "../../services/tauri";
import type { Folder } from "../../types";
import { cn } from "../../lib/utils";
import { AnimatedThemeToggler } from "../ui/animated-theme-toggler";
import { Button } from "@heroui/react";
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
}: ChatTopbarProps) {
  const workspaceName = getWorkspaceName(workspacePath);
  const title = resolveTitle(chatSession, titleStatus);

  return (
    <div className="pointer-events-none absolute left-0 right-0 top-0 z-40 px-4 pt-4 md:px-6">
      <div data-tauri-drag-region className="absolute inset-x-0 top-0 h-20" />

      <div className="pointer-events-auto mx-auto flex w-full max-w-5xl items-center justify-between gap-3 rounded-full border border-black/5 bg-background/90 px-3 py-1.5 shadow-sm backdrop-blur-md dark:border-white/10 dark:bg-background/20">
        <div className="flex min-w-0 items-center gap-2 md:gap-3">
          <div className="flex items-center gap-2 pl-1">
            <Compass className="size-4 text-primary" />
            <span className="text-sm font-medium tracking-tight">
              {titleStatus === "generating" ? "Titling..." : title}
            </span>
          </div>

          <Popover>
            <PopoverTrigger>
              <button
                type="button"
                className="ml-1 flex min-w-0 items-center gap-1 rounded-full px-2 py-1 text-xs font-medium text-muted-foreground transition-colors hover:bg-foreground/5 hover:text-foreground"
              >
                <span className="truncate uppercase tracking-wide">{workspaceName}</span>
                <ChevronDown className="size-3.5" />
              </button>
            </PopoverTrigger>
            <PopoverContent
              align="start"
              sideOffset={12}
              className="w-[280px] overflow-hidden rounded-2xl border border-white/10 bg-background/30 p-1.5 shadow-2xl backdrop-blur-2xl"
            >
              <div className="px-2.5 pb-2 pt-1">
                <div className="text-[10px] font-bold uppercase tracking-[0.18em] text-muted-foreground/60">
                  Recent workspaces
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
                        "flex w-full items-start gap-3 rounded-xl px-3 py-2.5 text-left transition-colors",
                        isActive
                          ? "bg-white/10 text-foreground"
                          : "text-muted-foreground hover:bg-white/5 hover:text-foreground",
                      )}
                    >
                      <div className="mt-0.5 size-2.5 shrink-0 rounded-full bg-primary/70" />
                      <div className="min-w-0">
                        <div className="truncate text-sm font-medium">{folder.name}</div>
                        <div className="truncate text-[11px] text-muted-foreground">{folder.path}</div>
                      </div>
                    </button>
                  );
                })}
              </div>
              <div className="px-1 pb-1 pt-2">
                <Button
                  variant="ghost"
                  className="w-full justify-start rounded-xl text-muted-foreground hover:bg-white/5 hover:text-foreground"
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
          <div className="mx-2 hidden h-4 w-px bg-border/50 sm:block" />
          <TooltipProvider delay={0}>
            <Tooltip>
              <TooltipTrigger
                render={
                  <button
                    type="button"
                    className="rounded-full p-2 text-muted-foreground transition-colors hover:bg-foreground/5 hover:text-foreground"
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
                    className="rounded-full p-2 text-muted-foreground transition-colors hover:bg-foreground/5 hover:text-foreground"
                    onClick={onNewChat}
                  />
                }
              >
                <SquarePen className="size-4" />
              </TooltipTrigger>
              <TooltipContent>New chat</TooltipContent>
            </Tooltip>
          </TooltipProvider>
          {onOpenSettings && (
            <button
              type="button"
              className="ml-1 rounded-full p-2 text-muted-foreground transition-colors hover:bg-foreground/5 hover:text-foreground"
              onClick={onOpenSettings}
            >
              <Settings2 className="size-4" />
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
