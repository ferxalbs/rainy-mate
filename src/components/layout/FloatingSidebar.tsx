import { Separator } from "@heroui/react";
import {
    FileText,
    Clock,
    Users,
    FolderOpen,
    Download,
    FileCode,
    HardDrive,
    Home,
    Cloud,
    Trash2,
    ChevronDown,
    ChevronRight,
    CheckCircle2,
    Timer,
    ListTodo,
    Sparkles,
    Shield,
    Palette,
} from "lucide-react";
import { useState } from "react";
import type { Folder } from "../../types";

interface FloatingSidebarProps {
    folders?: Folder[];
    onFolderSelect?: (folder: Folder) => void;
    onNavigate?: (section: string) => void;
    activeSection?: string;
    taskCounts?: {
        completed: number;
        running: number;
        queued: number;
    };
}

const defaultFolders: Folder[] = [
    { id: "1", path: "~/Desktop", name: "Desktop", accessType: "full-access" },
    { id: "2", path: "~/Documents", name: "Documents", accessType: "full-access" },
    { id: "3", path: "~/Downloads", name: "Downloads", accessType: "read-only" },
];

const locations: { id: string; name: string; icon: React.ReactNode }[] = [
    { id: "icloud", name: "iCloud Drive", icon: <Cloud className="size-4" /> },
    { id: "home", name: "Home", icon: <Home className="size-4" /> },
    { id: "macintosh", name: "Macintosh HD", icon: <HardDrive className="size-4" /> },
    { id: "trash", name: "Trash", icon: <Trash2 className="size-4" /> },
];

export function FloatingSidebar({
    folders = defaultFolders,
    onFolderSelect,
    onNavigate,
    activeSection = "running",
    taskCounts = { completed: 0, running: 0, queued: 0 },
}: FloatingSidebarProps) {
    const [expandedSections, setExpandedSections] = useState({
        favorites: true,
        locations: false,
        tasks: true,
        tags: false,
    });

    const toggleSection = (section: keyof typeof expandedSections) => {
        setExpandedSections((prev) => ({ ...prev, [section]: !prev[section] }));
    };

    return (
        <aside className="floating-sidebar w-56 h-fit max-h-[calc(100vh-48px)] overflow-y-auto animate-sidebar select-none">
            <div className="p-2 space-y-0.5">
                {/* Quick Access */}
                <SidebarItem icon={<FileText className="size-4 text-blue-500" />} label="Tasks" detail="Detail" />
                <SidebarItem icon={<Clock className="size-4 text-muted-foreground" />} label="Recents" detail="Detail" />
                <SidebarItem icon={<Users className="size-4 text-muted-foreground" />} label="Shared" detail="Detail" />
            </div>

            <Separator className="my-1 mx-2" />

            {/* Favorites Section */}
            <div className="p-2 pt-1">
                <SectionHeader
                    label="Favorites"
                    isExpanded={expandedSections.favorites}
                    onToggle={() => toggleSection("favorites")}
                />
                {expandedSections.favorites && (
                    <div className="space-y-0.5 mt-1">
                        {folders.map((folder) => (
                            <SidebarItem
                                key={folder.id}
                                icon={getFolderIcon(folder.name)}
                                label={folder.name}
                                detail="Detail"
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
                            icon={<CheckCircle2 className="size-4 text-green-500" />}
                            label="Completed"
                            badge={taskCounts.completed > 0 ? taskCounts.completed : undefined}
                            isActive={activeSection === "completed"}
                            onClick={() => onNavigate?.("completed")}
                        />
                        <SidebarItem
                            icon={<Timer className="size-4 text-blue-500" />}
                            label="Running"
                            badge={taskCounts.running > 0 ? taskCounts.running : undefined}
                            badgeColor="blue"
                            isActive={activeSection === "running"}
                            onClick={() => onNavigate?.("running")}
                        />
                        <SidebarItem
                            icon={<ListTodo className="size-4 text-orange-500" />}
                            label="Queued"
                            badge={taskCounts.queued > 0 ? taskCounts.queued : undefined}
                            isActive={activeSection === "queued"}
                            onClick={() => onNavigate?.("queued")}
                        />
                    </div>
                )}
            </div>

            <Separator className="my-1 mx-2" />

            {/* Locations Section */}
            <div className="p-2 pt-1">
                <SectionHeader
                    label="Locations"
                    detail="Detail"
                    isExpanded={expandedSections.locations}
                    onToggle={() => toggleSection("locations")}
                />
                {expandedSections.locations && (
                    <div className="space-y-0.5 mt-1">
                        {locations.map((loc) => (
                            <SidebarItem
                                key={loc.id}
                                icon={loc.icon}
                                label={loc.name}
                                detail="Detail"
                            />
                        ))}
                    </div>
                )}
            </div>

            <Separator className="my-1 mx-2" />

            {/* Settings/Tags Section */}
            <div className="p-2 pt-1">
                <SectionHeader
                    label="Settings"
                    detail="Detail"
                    isExpanded={expandedSections.tags}
                    onToggle={() => toggleSection("tags")}
                />
                {expandedSections.tags && (
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
    detail,
    isExpanded,
    onToggle,
}: {
    label: string;
    detail?: string;
    isExpanded: boolean;
    onToggle: () => void;
}) {
    return (
        <button
            className="flex items-center justify-between w-full px-2 py-1 text-xs font-medium text-muted-foreground uppercase tracking-wider hover:text-foreground transition-colors"
            onClick={onToggle}
        >
            <span>{label}</span>
            <span className="flex items-center gap-1">
                {detail && <span className="text-muted-foreground/60 normal-case">{detail}</span>}
                {isExpanded ? <ChevronDown className="size-3" /> : <ChevronRight className="size-3" />}
            </span>
        </button>
    );
}

function SidebarItem({
    icon,
    label,
    detail,
    badge,
    badgeColor = "default",
    isActive,
    onClick,
}: {
    icon: React.ReactNode;
    label: string;
    detail?: string;
    badge?: number;
    badgeColor?: "default" | "blue";
    isActive?: boolean;
    onClick?: () => void;
}) {
    return (
        <button
            className={`sidebar-item w-full ${isActive ? "bg-accent text-foreground font-medium" : ""}`}
            data-selected={isActive}
            onClick={onClick}
        >
            <span className="sidebar-item-icon">{icon}</span>
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
            {detail && !badge && <span className="sidebar-detail">{detail}</span>}
        </button>
    );
}

function getFolderIcon(name: string) {
    switch (name) {
        case "Desktop":
            return <FolderOpen className="size-4 text-blue-500" />;
        case "Documents":
            return <FileCode className="size-4 text-blue-500" />;
        case "Downloads":
            return <Download className="size-4 text-blue-500" />;
        default:
            return <FolderOpen className="size-4 text-muted-foreground" />;
    }
}
