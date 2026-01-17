import { ReactNode, useState, useEffect } from "react";
import { FloatingSidebar } from "./FloatingSidebar";
import { Button, Avatar, Switch } from "@heroui/react";
import { Settings, Moon, Sun, CloudRain, Maximize2, Minus, X } from "lucide-react";
import type { Folder } from "../../types";

interface TahoeLayoutProps {
    children: ReactNode;
    onFolderSelect?: (folder: Folder) => void;
    onNavigate?: (section: string) => void;
    activeSection?: string;
    taskCounts?: {
        completed: number;
        running: number;
        queued: number;
    };
}

export function TahoeLayout({
    children,
    onFolderSelect,
    onNavigate,
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
            {/* Transparent top bar - drag region */}
            <header
                className="window-drag flex items-center justify-between h-12 px-4"
                style={{ paddingLeft: isWindows ? 16 : 78 }}
            >
                {/* Left - App title */}
                <div className="window-no-drag flex items-center gap-2">
                    <CloudRain className="size-5 text-primary" />
                    <h1 className="text-sm font-semibold tracking-tight">Rainy Cowork</h1>
                </div>

                {/* Right - Controls */}
                <div className="window-no-drag flex items-center gap-2">
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
                    <Button variant="ghost" size="sm" aria-label="Settings">
                        <Settings className="size-4" />
                    </Button>

                    {/* Avatar */}
                    <Avatar size="sm">
                        <Avatar.Fallback>RC</Avatar.Fallback>
                    </Avatar>
                </div>

                {/* Windows Controls - only shown on Windows */}
                {isWindows && (
                    <div className="windows-controls flex items-center gap-0.5 ml-4">
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
            </header>

            {/* Main content area */}
            <div className="flex flex-1 p-3 gap-3 overflow-hidden">
                {/* Floating Sidebar */}
                <FloatingSidebar
                    onFolderSelect={onFolderSelect}
                    onNavigate={onNavigate}
                    activeSection={activeSection}
                    taskCounts={taskCounts}
                />

                {/* Main Content - Glass Surface */}
                <main className="flex-1 glass-surface overflow-y-auto p-6">
                    <div className="max-w-3xl mx-auto select-text">
                        {children}
                    </div>
                </main>
            </div>
        </div>
    );
}
