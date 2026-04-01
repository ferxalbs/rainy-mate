import { useState } from "react";
import type { ReactNode } from "react";
import { BackgroundManager } from "../backgrounds/BackgroundManager";
import { AppSidebar } from "./AppSidebar";
import { AnimatedThemeToggler } from "../ui/animated-theme-toggler";
import { Maximize2, Minus, X } from "lucide-react";
import type { Folder } from "../../types";
import type { ChatSession } from "../../services/tauri";
import { useTheme } from "../../hooks/useTheme";


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
  useTheme();
  const [isWindows] = useState(() =>
    navigator.platform.toLowerCase().includes("win"),
  );
  const [isSidebarCollapsed, setIsSidebarCollapsed] = useState(false);


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
        {/* Floating header — absolute overlay so it takes no layout space */}
        <header className="absolute top-0 right-0 left-0 z-50 flex items-center justify-end h-12 px-5 pointer-events-none">
          {/* Drag region covers full header width */}
          <div
            data-tauri-drag-region
            className="absolute inset-0 pointer-events-auto"
          />
          {/* Right controls */}
          <div className="window-no-drag flex items-center gap-3 pointer-events-auto relative z-50">
            {activeSection === "settings" && <AnimatedThemeToggler />}

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

        {/* Content Area — fills full height, header floats above */}
        <div className="flex flex-1 min-w-0 overflow-hidden relative">
          <main className={`flex-1 overflow-auto relative ${shouldBeImmersive ? "p-0" : "p-4"}`}>
            <div className={`w-full h-full select-text`}>
              {children}
            </div>
          </main>
        </div>
      </div>
    </div>
  );
}
