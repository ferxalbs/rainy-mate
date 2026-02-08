import { ReactNode, useState, useEffect } from "react";
import { BackgroundManager } from "../backgrounds/BackgroundManager";
import { AppSidebar } from "./AppSidebar";
import { MacOSToggle } from "./MacOSToggle";
import { Button, Chip } from "@heroui/react";
import { Maximize2, Minus, X, FolderOpen, Cloud, CloudOff } from "lucide-react";
import type { Folder } from "../../types";
import { useTheme } from "../../hooks/useTheme";
import { useCloudBridgeStatus } from "../../hooks/useCloudBridgeStatus";

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
}: TahoeLayoutProps & { isImmersive?: boolean }) {
  const { mode, setMode } = useTheme();
  const [isWindows, setIsWindows] = useState(false);
  const [isSidebarCollapsed, setIsSidebarCollapsed] = useState(false);
  const cloudStatus = useCloudBridgeStatus();

  useEffect(() => {
    // Detect OS
    const platform = navigator.platform.toLowerCase();
    setIsWindows(platform.includes("win"));
  }, []);

  const isDark = mode === "dark";

  const toggleTheme = (selected: boolean) => {
    setMode(selected ? "dark" : "light");
  };

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
      />

      {/* 2nd Column Wrapper (Inspector Removed) */}
      <div className="flex flex-col flex-1 min-w-0 h-full relative z-10 transition-all duration-300">
        {/* Universal Header - Glass Effect with Mode Specifics */}
        {!shouldBeImmersive && (
          <header
            className={`flex items-center justify-between h-14 px-6 shrink-0 border-b border-border/10 backdrop-blur-2xl transition-colors duration-300 ${isDark ? "bg-background/30" : "bg-background/60"}`}
          >
            {/* Drag region */}
            <div
              data-tauri-drag-region
              className="absolute inset-0 h-10 -z-10"
            />

            {/* Left Side: Workspace Info */}
            <div className="window-no-drag flex items-center gap-3 min-w-0">
              {workspacePath && (
                <div className="flex items-center gap-2.5 px-3 py-1.5 rounded-xl bg-white/5 border border-white/10 animate-appear shadow-sm">
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
              <Chip
                size="sm"
                variant="soft"
                title={cloudStatus.message}
                className={
                  cloudStatus.connected
                    ? "bg-green-500/10 text-green-500 border border-green-500/20"
                    : "bg-amber-500/10 text-amber-500 border border-amber-500/20"
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
              </Chip>
            </div>

            {/* Right Side: Global Controls */}
            <div className="window-no-drag flex items-center gap-4">
              {/* Custom macOS Toggle */}
              <MacOSToggle
                isDark={isDark}
                onToggle={(checked) => toggleTheme(checked)}
              />

              {/* Windows Controls */}
              {isWindows && (
                <div className="windows-controls flex items-center gap-1">
                  <Button
                    variant="ghost"
                    size="sm"
                    isIconOnly
                    aria-label="Minimize"
                  >
                    <Minus className="size-4" />
                  </Button>
                  <Button
                    variant="ghost"
                    size="sm"
                    isIconOnly
                    aria-label="Maximize"
                  >
                    <Maximize2 className="size-4" />
                  </Button>
                  <Button
                    variant="ghost"
                    size="sm"
                    isIconOnly
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
