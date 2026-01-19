import { Separator } from "@heroui/react";
import {
    FolderOpen,
    Download,
    FileCode,
    ChevronDown,
    ChevronRight,
    CheckCircle2,
    Timer,
    ListTodo,
    Sparkles,
    Shield,
    Palette,
    Plus,
    FileText,
    Search,
} from "lucide-react";
import { useState } from "react";
import type { Folder } from "../../types";

interface FloatingSidebarProps {
    folders?: Folder[];
    activeFolderId?: string;
    onFolderSelect?: (folder: Folder) => void;
    onNavigate?: (section: string) => void;
    activeSection?: string;
    taskCounts?: {
        completed: number;
        running: number;
        queued: number;
    };
    onAddFolder?: () => void;
}

const defaultFolders: Folder[] = [
    { id: "1", path: "~/Documents", name: "Documents", accessType: "full-access" },
    { id: "2", path: "~/Downloads", name: "Downloads", accessType: "read-only" },
    { id: "3", path: "~/Projects", name: "Projects", accessType: "full-access" },
];

export function FloatingSidebar({
    folders = defaultFolders,
    activeFolderId,
    onFolderSelect,
    onNavigate,
    activeSection = "running",
    taskCounts = { completed: 0, running: 0, queued: 0 },
    onAddFolder,
}: FloatingSidebarProps) {
    const [expandedSections, setExpandedSections] = useState({
        folders: true,
        tasks: true,
        aiStudio: true,
        settings: false,
    });

    const toggleSection = (section: keyof typeof expandedSections) => {
        setExpandedSections((prev) => ({ ...prev, [section]: !prev[section] }));
    };

    return (
        <aside className="w-52 h-fit max-h-[calc(100vh-48px)] overflow-y-auto overflow-x-hidden select-none rounded-2xl bg-sidebar backdrop-blur-[20px] backdrop-saturate-150 shadow-lg border border-sidebar-border animate-sidebar">
            {/* Folders Section */}
            <div className="p-2">
                <SectionHeader
                    label="Folders"
                    isExpanded={expandedSections.folders}
                    onToggle={() => toggleSection("folders")}
                    action={
                        <button
                            className="p-0.5 rounded hover:bg-accent transition-colors"
                            onClick={(e) => {
                                e.stopPropagation();
                                onAddFolder?.();
                            }}
                            aria-label="Add folder"
                        >
                            <Plus className="size-3" />
                        </button>
                    }
                />
                {expandedSections.folders && (
                    <div className="space-y-0.5 mt-1">
                        {folders.map((folder) => (
                            <SidebarItem
                                key={folder.id}
                                icon={getFolderIcon(folder.name)}
                                label={folder.name}
                                isActive={folder.id === activeFolderId}
                                onClick={() => onFolderSelect?.(folder)}
                            />
                        ))}
                    </div>
                )}
            </div>

            <Separator className="my-1 mx-2" />

            {/* Tasks Section */}
            <div className="p-2 pt-1">
                <SectionHeader
                    label="Tasks"
                    isExpanded={expandedSections.tasks}
                    onToggle={() => toggleSection("tasks")}
                />
                {expandedSections.tasks && (
                    <div className="space-y-0.5 mt-1">
                        <SidebarItem
                            icon={<Timer className="size-4" />}
                            label="Running"
                            badge={taskCounts.running > 0 ? taskCounts.running : undefined}
                            isActive={activeSection === "running"}
                            onClick={() => onNavigate?.("running")}
                        />
                        <SidebarItem
                            icon={<ListTodo className="size-4" />}
                            label="Queued"
                            badge={taskCounts.queued > 0 ? taskCounts.queued : undefined}
                            isActive={activeSection === "queued"}
                            onClick={() => onNavigate?.("queued")}
                        />
                        <SidebarItem
                            icon={<CheckCircle2 className="size-4" />}
                            label="Completed"
                            badge={taskCounts.completed > 0 ? taskCounts.completed : undefined}
                            isActive={activeSection === "completed"}
                            onClick={() => onNavigate?.("completed")}
                        />
                    </div>
                )}
            </div>

            <Separator className="my-1 mx-2" />

            {/* AI Studio Section */}
            <div className="p-2 pt-1">
                <SectionHeader
                    label="AI Studio"
                    isExpanded={expandedSections.aiStudio}
                    onToggle={() => toggleSection("aiStudio")}
                />
                {expandedSections.aiStudio && (
                    <div className="space-y-0.5 mt-1">
                        <SidebarItem
                            icon={<FileText className="size-4" />}
                            label="Documents"
                            isActive={activeSection === "documents"}
                            onClick={() => onNavigate?.("documents")}
                        />
                        <SidebarItem
                            icon={<Search className="size-4" />}
                            label="Research"
                            isActive={activeSection === "research"}
                            onClick={() => onNavigate?.("research")}
                        />
                    </div>
                )}
            </div>

            <Separator className="my-1 mx-2" />

            {/* Settings Section */}
            <div className="p-2 pt-1">
                <SectionHeader
                    label="Settings"
                    isExpanded={expandedSections.settings}
                    onToggle={() => toggleSection("settings")}
                />
                {expandedSections.settings && (
                    <div className="space-y-0.5 mt-1">
                        <SidebarItem icon={<Sparkles className="size-4" />} label="AI Provider" />
                        <SidebarItem icon={<Shield className="size-4" />} label="Permissions" />
                        <SidebarItem icon={<Palette className="size-4" />} label="Appearance" />
                    </div>
                )}
            </div>
        </aside>
    );
}

/* Helper Components */

function SectionHeader({
    label,
    isExpanded,
    onToggle,
    action,
}: {
    label: string;
    isExpanded: boolean;
    onToggle: () => void;
    action?: React.ReactNode;
}) {
    return (
        <div className="flex items-center justify-between w-full px-2 py-1">
            <button
                className="flex items-center gap-1 text-xs font-medium text-muted-foreground uppercase tracking-wider hover:text-foreground transition-colors"
                onClick={onToggle}
            >
                {isExpanded ? <ChevronDown className="size-3" /> : <ChevronRight className="size-3" />}
                <span>{label}</span>
            </button>
            {action}
        </div>
    );
}

function SidebarItem({
    icon,
    label,
    badge,
    badgeColor = "default",
    isActive,
    onClick,
}: {
    icon: React.ReactNode;
    label: string;
    badge?: number;
    badgeColor?: "default" | "blue";
    isActive?: boolean;
    onClick?: () => void;
}) {
    return (
        <button
            className={`w-full flex items-center gap-2 px-3 py-2 rounded-lg text-sm transition-colors group ${isActive
                ? "bg-accent text-foreground font-medium"
                : "text-foreground/80 hover:bg-accent/50 hover:text-foreground"
                }`}
            data-selected={isActive}
            onClick={onClick}
        >
            <span className={`size-4 shrink-0 transition-discrete ${isActive ? "text-primary" : "text-muted-foreground group-hover:text-foreground"}`}>
                {icon}
            </span>
            <span className="truncate flex-1 text-left">{label}</span>
            {badge !== undefined && (
                <span
                    className={`text-xs px-1.5 py-0.5 rounded-full ${badgeColor === "blue"
                        ? "bg-blue-500/20 text-blue-500"
                        : "bg-muted text-muted-foreground"
                        }`}
                >
                    {badge}
                </span>
            )}
        </button>
    );
}

function getFolderIcon(name: string) {
    switch (name) {
        case "Documents":
            return <FileCode className="size-4" />;
        case "Downloads":
            return <Download className="size-4" />;
        case "Projects":
            return <FolderOpen className="size-4" />;
        default:
            return <FolderOpen className="size-4" />;
    }
}
