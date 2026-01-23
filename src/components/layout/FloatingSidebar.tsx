import { Separator, Button } from "@heroui/react";
import {
  FolderOpen,
  Download,
  FileCode,
  ChevronDown,
  ChevronRight,
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
  {
    id: "1",
    path: "~/Documents",
    name: "Documents",
    accessType: "full-access",
  },
  { id: "2", path: "~/Downloads", name: "Downloads", accessType: "read-only" },
  { id: "3", path: "~/Projects", name: "Projects", accessType: "full-access" },
];

export function FloatingSidebar({
  folders = defaultFolders,
  activeFolderId,
  onFolderSelect,
  onNavigate,
  activeSection = "running",
  taskCounts: _taskCounts = { completed: 0, running: 0, queued: 0 },
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
    <aside className="w-52 h-fit max-h-full overflow-y-auto overflow-x-hidden select-none rounded-2xl bg-sidebar/20 dark:bg-black/10 backdrop-blur-2xl backdrop-saturate-200 shadow-2xl border border-white/10 dark:border-white/5 animate-sidebar">
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

      {/* Cowork - Main Action */}
      <div className="p-2 pt-1">
        <SidebarItem
          icon={<Sparkles className="size-4" />}
          label="Cowork"
          isActive={activeSection === "cowork" || activeSection === "running"}
          onClick={() => onNavigate?.("cowork")}
        />
      </div>

      <Separator className="my-1 mx-2" />

      {/* Tools Section */}
      <div className="p-2 pt-1">
        <SectionHeader
          label="Tools"
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
            <SidebarItem
              icon={<Sparkles className="size-4" />}
              label="AI Provider"
              isActive={activeSection === "settings-models"}
              onClick={() => onNavigate?.("settings-models")}
            />
            <SidebarItem
              icon={<Shield className="size-4" />}
              label="Permissions"
              isActive={activeSection === "settings-permissions"}
              onClick={() => onNavigate?.("settings-permissions")}
            />
            <SidebarItem
              icon={<Palette className="size-4" />}
              label="Appearance"
              isActive={activeSection === "settings-appearance"}
              onClick={() => onNavigate?.("settings-appearance")}
            />
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
        {isExpanded ? (
          <ChevronDown className="size-3" />
        ) : (
          <ChevronRight className="size-3" />
        )}
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
  // HeroUI v3: Use "secondary" for active state (or primary if preferred), "ghost" for inactive
  // visual variants: solid, faded, bordered, light, flat, ghost
  return (
    <Button
      variant={isActive ? "secondary" : "ghost"}
      className={`w-full justify-start gap-2 h-9 px-3 text-sm font-normal group ${
        isActive ? "font-medium" : "text-foreground/80 hover:text-foreground"
      }`}
      onPress={onClick}
    >
      <span
        className={`size-4 shrink-0 transition-colors ${isActive ? "text-foreground" : "text-muted-foreground group-hover:text-foreground"}`}
      >
        {icon}
      </span>
      <span className="truncate flex-1 text-left">{label}</span>
      {badge !== undefined && (
        <span
          className={`text-xs px-1.5 py-0.5 rounded-full ${
            badgeColor === "blue"
              ? "bg-blue-500/20 text-blue-500"
              : "bg-default-200 text-default-500"
          }`}
        >
          {badge}
        </span>
      )}
    </Button>
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
