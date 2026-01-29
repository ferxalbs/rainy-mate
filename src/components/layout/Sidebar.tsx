import { ListBox, Separator, Label, Button } from "@heroui/react";
import {
  FolderOpen,
  Download,
  FileCode,
  CheckCircle2,
  Timer,
  ListTodo,
  Clock,
  Sparkles,
  Shield,
  Palette,
  ChevronLeft,
  ChevronRight,
  MessageSquare,
  FileText,
  Search,
} from "lucide-react";
import type { Folder } from "../../types";

interface SidebarProps {
  folders?: Folder[];
  onFolderSelect?: (folder: Folder) => void;
  onNavigate?: (section: string) => void;
  activeSection?: string;
  taskCounts?: {
    completed: number;
    running: number;
    queued: number;
  };
  isCollapsed?: boolean;
  onToggleCollapse?: () => void;
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

const folderIcons: Record<string, React.ReactNode> = {
  Documents: <FolderOpen className="size-4 shrink-0" />,
  Downloads: <Download className="size-4 shrink-0" />,
  Projects: <FileCode className="size-4 shrink-0" />,
};

export function Sidebar({
  folders = defaultFolders,
  onFolderSelect,
  onNavigate,
  activeSection = "tasks",
  taskCounts = { completed: 0, running: 0, queued: 0 },
  isCollapsed = false,
  onToggleCollapse,
}: SidebarProps) {
  if (isCollapsed) {
    return (
      <aside className="hidden lg:flex flex-col w-14 h-full border-r border-border bg-sidebar items-center py-3 gap-2">
        <Button
          variant="ghost"
          size="sm"
          onPress={onToggleCollapse}
          aria-label="Expand sidebar"
          className="mb-2"
        >
          <ChevronRight className="size-4" />
        </Button>

        {/* Collapsed icons */}
        <div className="flex flex-col gap-1">
          {folders.map((folder) => (
            <Button
              key={folder.id}
              variant="ghost"
              size="sm"
              onPress={() => onFolderSelect?.(folder)}
              aria-label={folder.name}
            >
              {folderIcons[folder.name] || <FolderOpen className="size-4" />}
            </Button>
          ))}
        </div>

        <Separator className="my-2 w-8" />

        <Button
          variant="ghost"
          size="sm"
          onPress={() => onNavigate?.("running")}
          aria-label="Running tasks"
        >
          <Timer className="size-4 text-blue-500" />
        </Button>
        <Button
          variant="ghost"
          size="sm"
          onPress={() => onNavigate?.("completed")}
          aria-label="Completed tasks"
        >
          <CheckCircle2 className="size-4 text-green-500" />
        </Button>
      </aside>
    );
  }

  return (
    <aside className="hidden lg:flex flex-col w-56 xl:w-64 h-full border-r border-border bg-sidebar transition-all duration-200">
      {/* Collapse button */}
      <div className="flex justify-end p-2 border-b border-border">
        <Button
          variant="ghost"
          size="sm"
          onPress={onToggleCollapse}
          aria-label="Collapse sidebar"
        >
          <ChevronLeft className="size-4" />
        </Button>
      </div>

      <div className="flex-1 overflow-y-auto p-3 space-y-4">
        {/* Folders Section */}
        <section>
          <Label className="px-2 text-xs font-medium text-muted-foreground uppercase tracking-wider">
            📁 Folders
          </Label>
          <ListBox
            aria-label="Folders"
            className="mt-2"
            selectionMode="single"
            onSelectionChange={(keys) => {
              const selectedId = [...keys][0];
              const folder = folders.find((f) => f.id === selectedId);
              if (folder && onFolderSelect) {
                onFolderSelect(folder);
              }
            }}
          >
            {folders.map((folder) => (
              <ListBox.Item
                key={folder.id}
                id={folder.id}
                textValue={folder.name}
              >
                <div className="flex items-center gap-2">
                  {folderIcons[folder.name] || (
                    <FolderOpen className="size-4" />
                  )}
                  <span className="truncate">{folder.name}</span>
                </div>
              </ListBox.Item>
            ))}
          </ListBox>
        </section>

        <Separator />

        {/* AI Studio Section */}
        <section>
          <Label className="px-2 text-xs font-medium text-muted-foreground uppercase tracking-wider">
            ✨ AI Studio
          </Label>
          <ListBox
            aria-label="AI Studio"
            className="mt-2"
            selectionMode="single"
            selectedKeys={[activeSection]}
            onSelectionChange={(keys) => {
              const selected = [...keys][0] as string;
              if (onNavigate) {
                onNavigate(selected);
              }
            }}
          >
            <ListBox.Item id="cowork" textValue="Cowork Chat">
              <div className="flex items-center gap-2">
                <MessageSquare className="size-4 shrink-0 text-purple-500" />
                <span>Cowork Chat</span>
              </div>
            </ListBox.Item>
            <ListBox.Item id="documents" textValue="Documents">
              <div className="flex items-center gap-2">
                <FileText className="size-4 shrink-0 text-blue-500" />
                <span>Documents</span>
              </div>
            </ListBox.Item>
            <ListBox.Item id="research" textValue="Research">
              <div className="flex items-center gap-2">
                <Search className="size-4 shrink-0 text-green-500" />
                <span>Research</span>
              </div>
            </ListBox.Item>
          </ListBox>
        </section>

        <Separator />

        {/* Tasks Section */}
        <section>
          <Label className="px-2 text-xs font-medium text-muted-foreground uppercase tracking-wider">
            📋 Tasks
          </Label>
          <ListBox
            aria-label="Tasks"
            className="mt-2"
            selectionMode="single"
            selectedKeys={[activeSection]}
            onSelectionChange={(keys) => {
              const selected = [...keys][0] as string;
              if (onNavigate) {
                onNavigate(selected);
              }
            }}
          >
            <ListBox.Item id="completed" textValue="Completed">
              <div className="flex items-center justify-between w-full">
                <div className="flex items-center gap-2">
                  <CheckCircle2 className="size-4 text-green-500 shrink-0" />
                  <span>Completed</span>
                </div>
                {taskCounts.completed > 0 && (
                  <span className="text-xs bg-muted px-1.5 py-0.5 rounded-full">
                    {taskCounts.completed}
                  </span>
                )}
              </div>
            </ListBox.Item>
            <ListBox.Item id="running" textValue="Running">
              <div className="flex items-center justify-between w-full">
                <div className="flex items-center gap-2">
                  <Timer className="size-4 text-blue-500 shrink-0" />
                  <span>Running</span>
                </div>
                {taskCounts.running > 0 && (
                  <span className="text-xs bg-blue-500/20 text-blue-500 px-1.5 py-0.5 rounded-full">
                    {taskCounts.running}
                  </span>
                )}
              </div>
            </ListBox.Item>
            <ListBox.Item id="queued" textValue="Queued">
              <div className="flex items-center justify-between w-full">
                <div className="flex items-center gap-2">
                  <ListTodo className="size-4 text-orange-500 shrink-0" />
                  <span>Queued</span>
                </div>
                {taskCounts.queued > 0 && (
                  <span className="text-xs bg-muted px-1.5 py-0.5 rounded-full">
                    {taskCounts.queued}
                  </span>
                )}
              </div>
            </ListBox.Item>
          </ListBox>
        </section>

        <Separator />

        {/* History Section */}
        <section>
          <Label className="px-2 text-xs font-medium text-muted-foreground uppercase tracking-wider">
            🕐 History
          </Label>
          <ListBox aria-label="History" className="mt-2" selectionMode="single">
            <ListBox.Item id="history-7d" textValue="Last 7 days">
              <div className="flex items-center gap-2">
                <Clock className="size-4 shrink-0" />
                <span>Last 7 days</span>
              </div>
            </ListBox.Item>
          </ListBox>
        </section>

        <Separator />

        {/* Settings Section */}
        <section>
          <Label className="px-2 text-xs font-medium text-muted-foreground uppercase tracking-wider">
            ⚙️ Settings
          </Label>
          <ListBox
            aria-label="Settings"
            className="mt-2"
            selectionMode="single"
          >
            <ListBox.Item id="ai-provider" textValue="AI Provider">
              <div className="flex items-center gap-2">
                <Sparkles className="size-4 shrink-0" />
                <span>AI Provider</span>
              </div>
            </ListBox.Item>
            <ListBox.Item id="permissions" textValue="Permissions">
              <div className="flex items-center gap-2">
                <Shield className="size-4 shrink-0" />
                <span>Permissions</span>
              </div>
            </ListBox.Item>
            <ListBox.Item id="appearance" textValue="Appearance">
              <div className="flex items-center gap-2">
                <Palette className="size-4 shrink-0" />
                <span>Appearance</span>
              </div>
            </ListBox.Item>
          </ListBox>
        </section>
      </div>
    </aside>
  );
}
