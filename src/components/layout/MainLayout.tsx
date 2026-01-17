import { ReactNode, useState } from "react";
import { Header } from "./Header";
import { Sidebar } from "./Sidebar";
import type { Folder } from "../../types";

interface MainLayoutProps {
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

export function MainLayout({
    children,
    onFolderSelect,
    onNavigate,
    activeSection,
    taskCounts,
}: MainLayoutProps) {
    const [isSidebarCollapsed, setIsSidebarCollapsed] = useState(false);

    const toggleSidebar = () => {
        setIsSidebarCollapsed((prev) => !prev);
    };

    return (
        <div className="flex flex-col h-screen bg-background text-foreground overflow-hidden">
            <Header
                isSidebarCollapsed={isSidebarCollapsed}
                onToggleSidebar={toggleSidebar}
            />
            <div className="flex flex-1 overflow-hidden">
                <Sidebar
                    onFolderSelect={onFolderSelect}
                    onNavigate={onNavigate}
                    activeSection={activeSection}
                    taskCounts={taskCounts}
                    isCollapsed={isSidebarCollapsed}
                    onToggleCollapse={toggleSidebar}
                />
                <main className="flex-1 overflow-y-auto p-4 sm:p-6 lg:p-8">
                    <div className="max-w-4xl mx-auto">
                        {children}
                    </div>
                </main>
            </div>
        </div>
    );
}
