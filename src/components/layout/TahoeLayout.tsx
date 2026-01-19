import { ReactNode, useState, useEffect } from "react";
import { FloatingSidebar } from "./FloatingSidebar";
import { Button, Avatar, Switch } from "@heroui/react";
import { Settings, Moon, Sun, Maximize2, Minus, X, FolderOpen } from "lucide-react";
import type { Folder } from "../../types";

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
    const [isDark, setIsDark] = useState(false);
    const [isWindows, setIsWindows] = useState(false);

    useEffect(() => {
        // Detect OS
        const platform = navigator.platform.toLowerCase();
        setIsWindows(platform.includes("win"));

        // Check system preference
        const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
        setIsDark(prefersDark);
        document.documentElement.classList.toggle("dark", prefersDark);
    }, []);

    const toggleTheme = (selected: boolean) => {
        setIsDark(selected);
        document.documentElement.classList.toggle("dark", selected);
    };

    return (
        <div className="flex flex-col h-screen bg-background overflow-hidden">
            {/* Drag region - covers top area for window movement */}
            <div
                data-tauri-drag-region
                className="absolute top-0 right-0 h-10 z-10"
                style={{ left: 78 }}
            />

            {/* Header with controls - inline, not floating */}
            <header className="flex items-center justify-end h-10 px-4 shrink-0">
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
                        {/* Workspace Title */}
                        {workspacePath && (
                            <div className="flex items-center gap-2 mb-6 pb-4 border-b border-border/50">
                                <FolderOpen className="size-5 text-primary" />
                                <h1 className="text-lg font-semibold">
                                    Rainy Cowork
                                    <span className="text-muted-foreground font-normal"> in </span>
                                    <span className="text-foreground/80 font-mono text-sm bg-muted/50 px-2 py-0.5 rounded">
                                        {workspacePath}
                                    </span>
                                </h1>
                            </div>
                        )}
                        {children}
                    </div>
                </main>
            </div>
        </div>
    );
}
