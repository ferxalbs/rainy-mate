import { ReactNode, useState, useEffect } from "react";
import { BackgroundManager } from "../backgrounds/BackgroundManager";
import { FloatingSidebar } from "./FloatingSidebar";
import { Button, Avatar, Switch } from "@heroui/react";
import { Settings, Moon, Sun, Maximize2, Minus, X, FolderOpen } from "lucide-react";
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
}: TahoeLayoutProps) {
    const { mode, setMode } = useTheme();
    const [isWindows, setIsWindows] = useState(false);

    useEffect(() => {
        // Detect OS
        const platform = navigator.platform.toLowerCase();
        setIsWindows(platform.includes("win"));
    }, []);

    const isDark = mode === 'dark';

    const toggleTheme = (selected: boolean) => {
        setMode(selected ? 'dark' : 'light');
    };

    return (
        <div className="flex flex-col h-screen bg-background overflow-hidden relative">
            <BackgroundManager />
            {/* Drag region - covers top area for window movement */}
            <div
                data-tauri-drag-region
                className="absolute top-0 right-0 h-10 z-10"
                style={{ left: 78 }}
            />

            {/* Header with controls - inline, not floating */}
            <header className="flex items-center justify-between h-10 px-4 shrink-0 relative">
                {/* Left Side: Workspace Info */}
                <div className={`relative z-20 window-no-drag flex items-center gap-2 max-w-[60%] overflow-hidden transition-[padding] duration-200 pointer-events-none ${!isWindows ? "pl-[78px]" : ""}`}>
                    {workspacePath && (
                        <div className="flex items-center gap-2 text-sm text-muted-foreground animate-appear pointer-events-auto">
                            <FolderOpen className="size-4 shrink-0" />
                            <span className="font-medium text-foreground truncate max-w-[300px]" title={workspacePath}>
                                {workspacePath.split('/').pop() || workspacePath}
                            </span>
                            <span className="opacity-50 text-xs hidden sm:inline-block truncate max-w-[200px]" title={workspacePath}>
                                {workspacePath.replace(workspacePath.split('/').pop() || '', '')}
                            </span>
                        </div>
                    )}
                </div>

                {/* Right Side: Window Controls */}
                <div className="relative z-20 window-no-drag flex items-center gap-2">
                    {/* Theme Toggle */}
                    <div className="flex items-center gap-1.5">
                        <Sun className="size-3.5 text-muted-foreground" />
                        <Switch
                            isSelected={isDark}
                            onChange={toggleTheme}
                            size="sm"
                            aria-label="Toggle dark mode"
                        >
                            <Switch.Control>
                                <Switch.Thumb />
                            </Switch.Control>
                        </Switch>
                        <Moon className="size-3.5 text-muted-foreground" />
                    </div>

                    {/* Settings */}
                    <Button variant="ghost" size="sm" aria-label="Settings" onPress={onSettingsClick}>
                        <Settings className="size-4" />
                    </Button>

                    {/* Avatar */}
                    <Avatar size="sm">
                        <Avatar.Fallback>RC</Avatar.Fallback>
                    </Avatar>

                    {/* Windows Controls - only shown on Windows */}
                    {isWindows && (
                        <div className="windows-controls flex items-center gap-0.5 ml-2">
                            <Button variant="ghost" size="sm" aria-label="Minimize">
                                <Minus className="size-4" />
                            </Button>
                            <Button variant="ghost" size="sm" aria-label="Maximize">
                                <Maximize2 className="size-4" />
                            </Button>
                            <Button variant="ghost" size="sm" aria-label="Close" className="hover:bg-red-500 hover:text-white">
                                <X className="size-4" />
                            </Button>
                        </div>
                    )}
                </div>
            </header>

            {/* Main content area */}
            <div className="flex flex-1 p-3 gap-3 overflow-hidden">
                {/* Floating Sidebar */}
                <FloatingSidebar
                    folders={folders}
                    activeFolderId={activeFolderId}
                    onFolderSelect={onFolderSelect}
                    onAddFolder={onAddFolder}
                    onNavigate={onNavigate}
                    activeSection={activeSection}
                    taskCounts={taskCounts}
                />

                {/* Main Content - Glass Surface */}
                <main className="flex-1 overflow-y-auto p-6 rounded-3xl bg-white/70 dark:bg-black/20 backdrop-blur-2xl backdrop-saturate-150 shadow-2xl border border-white/20 dark:border-white/10">
                    <div className="max-w-3xl mx-auto select-text">
                        {children}
                    </div>
                </main>
            </div>
        </div>
    );
}
