import { useState } from "react";
import type { ReactNode } from "react";
import { BackgroundManager } from "../backgrounds/BackgroundManager";
import { AppSidebar } from "./AppSidebar";
import { AnimatedThemeToggler } from "../ui/animated-theme-toggler";
import { Maximize2, Minus, X, FolderOpen, Cloud, CloudOff } from "lucide-react";
import type { Folder } from "../../types";
import type { ChatSession } from "../../services/tauri";
import { useTheme } from "../../hooks/useTheme";
import { useCloudBridgeStatus } from "../../hooks/useCloudBridgeStatus";
import { Badge } from "../ui/badge";
import { Button } from "@heroui/react";

interface TahoeLayoutProps {
  children: ReactNode;
  folders?: Folder[];
  activeFolderId?: string;
  workspacePath?: string;
  onFolderSelect?: (folder: Folder) => void;
  onAddFolder?: () => void;
  onNavigate?: (section: string) => void;
  onSettingsClick?: () => void;
  activeSection?: string;
  taskCounts?: {
    completed: number;
    running: number;
    queued: number;
  };

  // Multi-chat thread props
  chatSessionsByWorkspace?: Record<string, ChatSession[]>;
  activeWorkspacePath?: string;
  activeChatId?: string | null;
  activeRunChatIds?: Set<string>;
  onSelectChatForFolder?: (folder: Folder, chatId: string) => void;
  onRefreshWorkspaceChats?: (workspaceId: string) => Promise<void> | void;
  onDeleteChat?: (workspaceId: string, chatId: string) => void;
}

export function TahoeLayout({
  children,
  folders,
  activeFolderId,
  workspacePath,
  onFolderSelect,
  onAddFolder,
  onNavigate,
  onSettingsClick,
  activeSection,
  isImmersive,
  chatSessionsByWorkspace,
  activeWorkspacePath,
  activeChatId,
  activeRunChatIds,
  onSelectChatForFolder,
  onRefreshWorkspaceChats,
  onDeleteChat,
}: TahoeLayoutProps & { isImmersive?: boolean }) {
  const { mode } = useTheme();
  const [isWindows] = useState(() =>
    navigator.platform.toLowerCase().includes("win"),
  );
  const [isSidebarCollapsed, setIsSidebarCollapsed] = useState(false);
  const cloudStatus = useCloudBridgeStatus();

  const isDark = mode === "dark";

  const shouldBeImmersive = isImmersive ?? activeSection === "agent-chat";

  return (
    <div className="flex h-screen bg-transparent overflow-hidden relative font-sans">
      <BackgroundManager />

      {/* 1st Column: Integrated Sidebar */}
      <AppSidebar
        folders={folders}
        activeFolderId={activeFolderId}
        onFolderSelect={onFolderSelect}
        onAddFolder={onAddFolder}
        onNavigate={onNavigate}
        onSettingsClick={onSettingsClick}
        activeSection={activeSection}
        isCollapsed={isSidebarCollapsed}
        onToggleCollapse={() => setIsSidebarCollapsed(!isSidebarCollapsed)}
        chatSessionsByWorkspace={chatSessionsByWorkspace}
        activeWorkspacePath={activeWorkspacePath}
        activeChatId={activeChatId}
        activeRunChatIds={activeRunChatIds}
        onSelectChatForFolder={onSelectChatForFolder}
        onRefreshWorkspaceChats={onRefreshWorkspaceChats}
        onDeleteChat={onDeleteChat}
      />

      {/* 2nd Column Wrapper (Inspector Removed) */}
      <div className="flex flex-col flex-1 min-w-0 h-full relative z-10 transition-all duration-300">
        {/* Universal Header - Glass Effect with Mode Specifics */}
        {!shouldBeImmersive && (
          <header
            className={`flex items-center justify-between h-16 px-6 shrink-0 border-b border-border/10 backdrop-blur-2xl backdrop-saturate-150 transition-colors duration-300 ${isDark ? "bg-background/30" : "bg-background/60"}`}
          >
            {/* Drag region */}
            <div
              data-tauri-drag-region
              className="absolute inset-0 h-10 -z-10"
            />

            {/* Left Side: Workspace Info */}
            <div className="window-no-drag flex items-center gap-3 min-w-0">
              {workspacePath && (
                <div className="flex items-center gap-2.5 rounded-2xl border border-white/10 bg-background/80 px-3 py-2 shadow-sm backdrop-blur-md backdrop-saturate-150 dark:bg-background/20">
                  <FolderOpen className="size-4 text-primary shrink-0" />
                  <div className="flex flex-col min-w-0">
                    <span
                      className="text-xs font-semibold text-foreground truncate max-w-[200px]"
                      title={workspacePath}
                    >
                      {workspacePath.split("/").pop() || workspacePath}
                    </span>
                  </div>
                </div>
              )}
              <Badge
                variant="outline"
                title={cloudStatus.message}
                className={
                  cloudStatus.connected
                    ? "rounded-full border-emerald-500/20 bg-emerald-500/10 px-3 py-1 text-emerald-700 dark:text-emerald-300"
                    : "rounded-full border-amber-500/20 bg-amber-500/10 px-3 py-1 text-amber-700 dark:text-amber-300"
                }
              >
                <span className="flex items-center gap-1.5">
                  {cloudStatus.connected ? (
                    <Cloud className="size-3" />
                  ) : (
                    <CloudOff className="size-3" />
                  )}
                  {cloudStatus.connected ? "Bridge Online" : "Bridge Offline"}
                </span>
              </Badge>
            </div>

            {/* Right Side: Global Controls */}
            <div className="window-no-drag flex items-center gap-4">
              {/* Custom macOS Toggle */}
              <AnimatedThemeToggler />

              {/* Windows Controls */}
              {isWindows && (
                <div className="windows-controls flex items-center gap-1">
                  <Button variant="ghost" aria-label="Minimize">
                    <Minus className="size-4" />
                  </Button>
                  <Button variant="ghost" aria-label="Maximize">
                    <Maximize2 className="size-4" />
                  </Button>
                  <Button
                    variant="ghost"
                    aria-label="Close"
                    className="hover:bg-red-500 hover:text-white"
                  >
                    <X className="size-4" />
                  </Button>
                </div>
              )}
            </div>
          </header>
        )}

        {/* Content Area */}
        <div className="flex flex-1 min-w-0 overflow-hidden relative">
          {/* Main Content */}
          <main
            className={`flex-1 overflow-auto relative ${shouldBeImmersive ? "p-0" : "p-6"}`}
          >
            <div
              className={`w-full h-full select-text ${shouldBeImmersive ? "" : "max-w-6xl mx-auto"}`}
            >
              {children}
            </div>
          </main>
        </div>
      </div>
    </div>
  );
}
