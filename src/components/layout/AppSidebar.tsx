import { Tooltip, Avatar, Button, Separator } from "@heroui/react";
import {
  FolderOpen,
  Download,
  FileCode,
  Sparkles,
  Palette,
  ChevronLeft,
  ChevronRight,
  MessageSquare,
  FileText,
  Search,
  Plus,
  LayoutGrid,
  Settings,
  Network,
  Bot,
} from "lucide-react";
import { MacOSToggle } from "./MacOSToggle";
import { useTheme } from "../../hooks/useTheme";
import type { Folder } from "../../types";

interface AppSidebarProps {
  folders?: Folder[];
  onFolderSelect?: (folder: Folder) => void;
  onAddFolder?: () => void;
  onNavigate?: (section: string) => void;
  activeSection?: string;
  activeFolderId?: string;

  isCollapsed?: boolean;
  onToggleCollapse?: () => void;
  onSettingsClick?: () => void;
}

const folderIcons: Record<string, any> = {
  Documents: FileText,
  Downloads: Download,
  Projects: FileCode,
};

export function AppSidebar({
  folders = [],
  onFolderSelect,
  onAddFolder,
  onNavigate,
  activeSection = "running",
  activeFolderId,
  isCollapsed = false,
  onToggleCollapse,
  onSettingsClick,
}: AppSidebarProps) {
  const { mode, setMode } = useTheme();

  const NavItem = ({
    id,
    label,
    icon: Icon,
    colorClass,
    badge,
  }: {
    id: string;
    label: string;
    icon: any;
    colorClass?: string;
    badge?: number;
  }) => {
    const isActive = activeSection === id;

    const content = (
      <Button
        variant={isActive ? "secondary" : "ghost"}
        isIconOnly={isCollapsed}
        className={`transition-all duration-200 group relative ${
          isCollapsed
            ? "w-10 h-10 justify-center mx-auto rounded-xl mb-1"
            : "w-full justify-start gap-3 h-10 px-3"
        } ${
          isActive
            ? "bg-primary/10 text-primary font-medium shadow-sm"
            : "text-muted-foreground hover:text-foreground hover:bg-muted/50"
        }`}
        onPress={() => onNavigate?.(id)}
      >
        <Icon
          className={`size-4 shrink-0 ${colorClass && !isActive ? colorClass : ""}`}
        />
        {!isCollapsed && (
          <>
            <span className="truncate flex-1 text-left">{label}</span>
            {badge !== undefined && badge > 0 && (
              <span
                className={`text-[10px] font-medium px-1.5 py-0.5 rounded-full ${
                  id === "running"
                    ? "bg-blue-500/10 text-blue-500"
                    : "bg-default-100 text-default-500"
                }`}
              >
                {badge}
              </span>
            )}
          </>
        )}
      </Button>
    );

    if (isCollapsed) {
      return (
        <Tooltip delay={0}>
          {content}
          <Tooltip.Content placement="right">{label}</Tooltip.Content>
        </Tooltip>
      );
    }
    return content;
  };

  return (
    <aside
      className={`flex flex-col h-full border-r border-border/50 transition-all duration-300 ease-in-out z-30 ${
        isCollapsed ? "w-16" : "w-64"
      } bg-sidebar`}
    >
      {/* Sidebar Header / Logo */}
      <div
        data-tauri-drag-region
        className={`mt-8 px-4 pb-4 flex items-center shrink-0 overflow-hidden ${isCollapsed ? "justify-center" : "gap-3"}`}
      >
        <div className="size-8 rounded-xl bg-primary flex items-center justify-center shadow-lg shadow-primary/20 shrink-0">
          <LayoutGrid className="size-5 text-white" />
        </div>
        {!isCollapsed && (
          <div className="flex flex-col min-w-0">
            <span className="font-bold text-sm tracking-tight truncate">
              RAINY COWORK
            </span>
            <span className="text-[10px] text-muted-foreground font-medium uppercase tracking-[0.2em]">
              Agent Platform
            </span>
          </div>
        )}
      </div>

      <div className="flex-1 overflow-y-auto overflow-x-hidden p-3 space-y-6 scrollbar-hide">
        {/* Folders Section */}
        <div className="space-y-1">
          {!isCollapsed && (
            <div className="flex items-center justify-between px-3 py-2 mb-1">
              <span className="text-[10px] font-bold text-muted-foreground/60 uppercase tracking-widest">
                Workspace
              </span>
              <Button
                variant="ghost"
                size="sm"
                isIconOnly
                onPress={onAddFolder}
                className="size-5 min-w-5 h-5 opacity-40 hover:opacity-100"
              >
                <Plus className="size-3" />
              </Button>
            </div>
          )}

          {folders.length > 0 ? (
            <div className="space-y-0.5">
              {folders.map((folder) => {
                const isActive = folder.id === activeFolderId;
                const Icon = folderIcons[folder.name] || FolderOpen;

                const folderBtn = (
                  <Button
                    key={folder.id}
                    variant={isActive ? "secondary" : "ghost"}
                    isIconOnly={isCollapsed}
                    className={`transition-all duration-200 group relative ${
                      isCollapsed
                        ? "w-10 h-10 justify-center mx-auto rounded-xl mb-1"
                        : "w-full justify-start gap-3 h-10 px-3"
                    } ${
                      isActive
                        ? "bg-primary/10 text-primary font-medium shadow-sm"
                        : "text-muted-foreground hover:text-foreground hover:bg-muted/50"
                    }`}
                    onPress={() => onFolderSelect?.(folder)}
                  >
                    <div
                      className={`flex items-center justify-center transition-colors ${isActive ? "text-primary" : "text-muted-foreground"}`}
                    >
                      <Icon className="size-4" />
                    </div>
                    {!isCollapsed && (
                      <span className="truncate flex-1 text-left">
                        {folder.name}
                      </span>
                    )}
                  </Button>
                );

                return isCollapsed ? (
                  <Tooltip key={folder.id} delay={0}>
                    {folderBtn}
                    <Tooltip.Content placement="right">
                      {folder.name}
                    </Tooltip.Content>
                  </Tooltip>
                ) : (
                  folderBtn
                );
              })}
            </div>
          ) : (
            !isCollapsed && (
              <div className="px-3 py-4 text-center rounded-xl border border-dashed border-border/50 bg-muted/20">
                <p className="text-[10px] text-muted-foreground mb-2">
                  No projects yet
                </p>
                <Button
                  size="sm"
                  variant="secondary"
                  onPress={onAddFolder}
                  className="h-7 text-[10px]"
                >
                  Add First
                </Button>
              </div>
            )
          )}
        </div>

        <Separator className="bg-border/30" />

        {/* AI Studio */}
        <div className="space-y-1">
          {!isCollapsed && (
            <div className="px-3 py-2 mb-1">
              <span className="text-[10px] font-bold text-muted-foreground/60 uppercase tracking-widest">
                AI Studio
              </span>
            </div>
          )}
          <NavItem
            id="agent-chat"
            label="Agent Chat"
            icon={MessageSquare}
            colorClass="text-purple-500"
          />
          <NavItem
            id="documents"
            label="Documents"
            icon={FileText}
            colorClass="text-blue-500"
          />
          <NavItem
            id="research"
            label="Research"
            icon={Search}
            colorClass="text-green-500"
          />
          <NavItem
            id="neural-link"
            label="Neural Link"
            icon={Network}
            colorClass="text-purple-500"
          />
          <NavItem
            id="agent-builder"
            label="Agent Builder"
            icon={Bot}
            colorClass="text-orange-500"
          />
        </div>
      </div>

      <div className="mt-auto p-3 space-y-2">
        <Separator className="bg-border/30" />

        {/* Settings Submenu */}
        <div className="space-y-1 pt-2">
          <NavItem id="settings-models" label="AI Provider" icon={Sparkles} />
          <NavItem id="settings-appearance" label="Appearance" icon={Palette} />
        </div>

        {/* User / Settings Footer */}
        <div
          className={`mt-2 flex items-center transition-all ${isCollapsed ? "flex-col gap-4 py-2" : "px-1 gap-3 py-2"}`}
        >
          <Avatar size="sm">
            <Avatar.Image src="https://api.dicebear.com/7.x/avataaars/svg?seed=Fernando" />
          </Avatar>
          {!isCollapsed && (
            <div className="flex flex-col min-w-0 flex-1">
              <span className="text-xs font-semibold truncate">Fernando</span>
              <span className="text-[10px] text-muted-foreground truncate opacity-70 italic">
                Premium Plan
              </span>
            </div>
          )}

          <div className="flex items-center gap-1">
            <MacOSToggle
              isDark={mode === "dark"}
              onToggle={(checked) => setMode(checked ? "dark" : "light")}
            />
          </div>

          <Tooltip delay={0}>
            <Button
              variant="ghost"
              size="sm"
              isIconOnly
              onPress={onSettingsClick}
              className="text-muted-foreground hover:bg-muted/50"
            >
              <Settings className="size-4" />
            </Button>
            <Tooltip.Content>Settings</Tooltip.Content>
          </Tooltip>
          <Button
            variant="ghost"
            size="sm"
            isIconOnly
            onPress={onToggleCollapse}
            className="text-muted-foreground hover:bg-muted/50"
          >
            {isCollapsed ? (
              <ChevronRight className="size-4" />
            ) : (
              <ChevronLeft className="size-4" />
            )}
          </Button>
        </div>
      </div>
    </aside>
  );
}
