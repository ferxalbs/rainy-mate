import { ReactNode, useState, useEffect } from "react";
import { BackgroundManager } from "../backgrounds/BackgroundManager";
import { AppSidebar } from "./AppSidebar";
import { InspectorPanel } from "./InspectorPanel";
import { MacOSToggle } from "./MacOSToggle";
import { Button, Tooltip } from "@heroui/react";
import { Maximize2, Minus, X, FolderOpen, PanelRight } from "lucide-react";
import type { Folder } from "../../types";
import { useTheme } from "../../hooks/useTheme";

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
  // Inspector Props
  inspectorTitle?: string;
  inspectorType?: "preview" | "info" | "process" | "links";
  inspectorContent?: string;
  inspectorFilename?: string;
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
  taskCounts,
  inspectorTitle,
  inspectorType,
  inspectorContent,
  inspectorFilename,
}: TahoeLayoutProps) {
  const { mode, setMode } = useTheme();
  const [isWindows, setIsWindows] = useState(false);
  const [isSidebarCollapsed, setIsSidebarCollapsed] = useState(false);
  const [isInspectorOpen, setIsInspectorOpen] = useState(false);

  useEffect(() => {
    // Detect OS
    const platform = navigator.platform.toLowerCase();
    setIsWindows(platform.includes("win"));
  }, []);

  const isDark = mode === "dark";

  const toggleTheme = (selected: boolean) => {
    setMode(selected ? "dark" : "light");
  };

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
        taskCounts={taskCounts}
        isCollapsed={isSidebarCollapsed}
        onToggleCollapse={() => setIsSidebarCollapsed(!isSidebarCollapsed)}
      />

      {/* 2nd & 3rd Column Wrapper */}
      <div className="flex flex-col flex-1 min-w-0 h-full relative z-10 transition-all duration-300">
        {/* Universal Header - Glass Effect with Mode Specifics */}
        <header
          className={`flex items-center justify-between h-14 px-6 shrink-0 border-b border-border/10 backdrop-blur-2xl transition-colors duration-300 ${isDark ? "bg-background/30" : "bg-background/60"}`}
        >
          {/* Drag region */}
          <div data-tauri-drag-region className="absolute inset-0 h-10 -z-10" />

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
          </div>

          {/* Right Side: Global Controls */}
          <div className="window-no-drag flex items-center gap-4">
            {/* Custom macOS Toggle */}
            <MacOSToggle
              isDark={isDark}
              onToggle={(checked) => toggleTheme(checked)}
            />

            {/* Inspector Toggle */}
            <Tooltip delay={500}>
              <Button
                variant={isInspectorOpen ? "secondary" : "ghost"}
                size="sm"
                isIconOnly
                onPress={() => setIsInspectorOpen(!isInspectorOpen)}
                className="rounded-full data-[hover=true]:bg-muted/30"
              >
                <PanelRight className="size-4 opacity-80" />
              </Button>
              <Tooltip.Content placement="bottom">
                Toggle Inspector
              </Tooltip.Content>
            </Tooltip>

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

        {/* Content Area + 3rd Column (Inspector) */}
        <div className="flex flex-1 min-w-0 overflow-hidden relative">
          {/* Main Content */}
          <main className="flex-1 overflow-auto relative p-6">
            <div className="max-w-6xl mx-auto w-full h-full select-text">
              {children}
            </div>
          </main>

          {/* 3rd Column: Inspector */}
          <InspectorPanel
            isOpen={isInspectorOpen}
            onClose={() => setIsInspectorOpen(false)}
            title={inspectorTitle}
            type={inspectorType}
            content={inspectorContent}
            filename={inspectorFilename}
          />
        </div>
      </div>
    </div>
  );
}
