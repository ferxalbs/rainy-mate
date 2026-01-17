import { Button, Avatar, Switch } from "@heroui/react";
import { Settings, Moon, Sun, CloudRain, PanelLeftClose, PanelLeft } from "lucide-react";
import { useEffect, useState } from "react";

interface HeaderProps {
    onSettingsClick?: () => void;
    isSidebarCollapsed?: boolean;
    onToggleSidebar?: () => void;
}

export function Header({ onSettingsClick, isSidebarCollapsed, onToggleSidebar }: HeaderProps) {
    const [isDark, setIsDark] = useState(false);

    useEffect(() => {
        const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
        setIsDark(prefersDark);
        document.documentElement.classList.toggle("dark", prefersDark);
    }, []);

    const toggleTheme = (selected: boolean) => {
        setIsDark(selected);
        document.documentElement.classList.toggle("dark", selected);
    };

    return (
        <header className="flex items-center justify-between h-14 px-4 border-b border-border bg-background/80 backdrop-blur-md sticky top-0 z-50">
            {/* Left side */}
            <div className="flex items-center gap-3">
                {/* Sidebar Toggle */}
                <Button
                    variant="ghost"
                    size="sm"
                    onPress={onToggleSidebar}
                    aria-label={isSidebarCollapsed ? "Expand sidebar" : "Collapse sidebar"}
                    className="lg:hidden"
                >
                    {isSidebarCollapsed ? <PanelLeft className="size-5" /> : <PanelLeftClose className="size-5" />}
                </Button>

                {/* App Title */}
                <div className="flex items-center gap-2">
                    <CloudRain className="size-6 text-primary" />
                    <h1 className="text-lg font-semibold tracking-tight hidden sm:block">Rainy Cowork</h1>
                </div>
            </div>

            {/* Right side actions */}
            <div className="flex items-center gap-2 sm:gap-3">
                {/* Theme Toggle */}
                <div className="flex items-center gap-1.5 sm:gap-2">
                    <Sun className="size-4 text-muted-foreground hidden sm:block" />
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
                    <Moon className="size-4 text-muted-foreground hidden sm:block" />
                </div>

                {/* Settings Button */}
                <Button
                    variant="ghost"
                    size="sm"
                    onPress={onSettingsClick}
                    aria-label="Open settings"
                >
                    <Settings className="size-4" />
                </Button>

                {/* User Avatar */}
                <Avatar size="sm">
                    <Avatar.Fallback>RC</Avatar.Fallback>
                </Avatar>
            </div>
        </header>
    );
}
