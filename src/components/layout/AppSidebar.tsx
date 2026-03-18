import { useReducer, useState } from "react";
import { Tooltip, Button, Separator } from "@heroui/react";
import {
  FolderOpen,
  Download,
  FileCode,
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  SquarePen,
  FileText,
  Plus,
  Settings,
  Clock,
  Bot,
  Library,
  LayoutGrid,
  RefreshCw,
  Check,
  AlertCircle,
  BrainCircuit,
} from "lucide-react";
import { ChatThreadList } from "./ChatThreadList";
import type { ChatSession } from "../../services/tauri";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import type { Folder } from "../../types";
import { MandatoryUpdateOverlay } from "../updater/MandatoryUpdateOverlay";
import { Collapsible, CollapsibleContent } from "../ui/collapsible";

interface AppSidebarProps {
  folders?: Folder[];
  onFolderSelect?: (folder: Folder) => void;
  onAddFolder?: () => void;
  onNavigate?: (section: string) => void;
  activeSection?: string;
  activeFolderId?: string;

  // Multi-chat thread props
  chatSessionsByWorkspace?: Record<string, ChatSession[]>;
  activeWorkspacePath?: string;
  activeChatId?: string | null;
  onSelectChatForFolder?: (folder: Folder, chatId: string) => void;
  onRefreshWorkspaceChats?: (workspaceId: string) => Promise<void> | void;
  onDeleteChat?: (workspaceId: string, chatId: string) => void;

  isCollapsed?: boolean;
  onToggleCollapse?: () => void;
  onSettingsClick?: () => void;
}

const EMPTY_FOLDERS: Folder[] = [];
const EMPTY_CHAT_SESSIONS_BY_WORKSPACE: Record<string, ChatSession[]> = {};

const folderIcons: Record<string, any> = {
  Documents: FileText,
  Downloads: Download,
  Projects: FileCode,
};

type UpdateStatus =
  | "idle"
  | "checking"
  | "available"
  | "downloading"
  | "up-to-date"
  | "error";

const NavItem = ({
  id,
  label,
  icon: Icon,
  colorClass,
  badge,
  isActive,
  isCollapsed,
  onNavigate,
}: {
  id: string;
  label: string;
  icon: any;
  colorClass?: string;
  badge?: number;
  isActive: boolean;
  isCollapsed: boolean;
  onNavigate?: (id: string) => void;
}) => {
  const content = (
    <Button
      variant={isActive ? "secondary" : "ghost"}
      isIconOnly={isCollapsed}
      className={`transition-all duration-200 group relative ${
        isCollapsed
          ? "w-9 h-9 justify-center mx-auto rounded-xl"
          : "w-full justify-start gap-3 h-9 px-3 rounded-xl"
      } ${
        isActive
          ? "bg-primary/10 text-primary font-medium shadow-none"
          : "text-muted-foreground hover:text-foreground hover:bg-white/10"
      }`}
      onPress={() => onNavigate?.(id)}
    >
      <Icon
        className={`size-4 shrink-0 ${colorClass && !isActive ? colorClass : ""}`}
      />
      {!isCollapsed && (
        <>
          <span className="truncate flex-1 text-left text-[13px]">{label}</span>
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

export function AppSidebar({
  folders = EMPTY_FOLDERS,
  onFolderSelect,
  onAddFolder,
  onNavigate,
  activeSection = "running",
  activeFolderId,
  chatSessionsByWorkspace = EMPTY_CHAT_SESSIONS_BY_WORKSPACE,
  activeWorkspacePath,
  activeChatId,
  onSelectChatForFolder,
  onRefreshWorkspaceChats,
  onDeleteChat,
  isCollapsed = false,
  onToggleCollapse,
  onSettingsClick,
}: AppSidebarProps) {
  const [expandedWorkspaces, setExpandedWorkspaces] = useState<Record<string, boolean>>({});

  type UpdateState = {
    status: UpdateStatus;
    version: string;
    currentVersion: string;
    pendingUpdate: Awaited<ReturnType<typeof check>> | null;
    progress: { downloaded: number; total: number | null };
    error: string;
  };

  type UpdateAction =
    | { type: "checking" }
    | { type: "available"; update: NonNullable<Awaited<ReturnType<typeof check>>> }
    | { type: "up-to-date" }
    | { type: "downloading" }
    | { type: "download-started"; total: number | null }
    | { type: "download-progress"; chunk: number }
    | { type: "error"; message: string }
    | { type: "reset" };

  const [updater, dispatch] = useReducer(
    (state: UpdateState, action: UpdateAction): UpdateState => {
      switch (action.type) {
        case "checking":
          return { ...state, status: "checking", error: "" };
        case "available":
          return { ...state, status: "available", version: action.update.version, currentVersion: action.update.currentVersion, pendingUpdate: action.update };
        case "up-to-date":
          return { ...state, status: "up-to-date" };
        case "downloading":
          return { ...state, status: "downloading", progress: { downloaded: 0, total: null } };
        case "download-started":
          return { ...state, progress: { downloaded: 0, total: action.total } };
        case "download-progress":
          return { ...state, progress: { ...state.progress, downloaded: state.progress.downloaded + action.chunk } };
        case "error":
          return { ...state, status: "error", error: action.message };
        case "reset":
          return { ...state, status: "idle", error: "" };
        default:
          return state;
      }
    },
    { status: "idle" as UpdateStatus, version: "", currentVersion: "", pendingUpdate: null, progress: { downloaded: 0, total: null }, error: "" },
  );

  const handleCheckUpdate = async () => {
    if (updater.status === "checking" || updater.status === "downloading") return;
    dispatch({ type: "checking" });
    try {
      const update = await check();
      if (update) {
        dispatch({ type: "available", update });
      } else {
        dispatch({ type: "up-to-date" });
        setTimeout(() => dispatch({ type: "reset" }), 3000);
      }
    } catch (err) {
      dispatch({ type: "error", message: err instanceof Error ? err.message : String(err) });
      setTimeout(() => dispatch({ type: "reset" }), 3000);
    }
  };

  const handleInstallUpdate = async () => {
    if (!updater.pendingUpdate) return;
    dispatch({ type: "downloading" });
    try {
      await updater.pendingUpdate.downloadAndInstall((event) => {
        if (event.event === "Started") {
          dispatch({ type: "download-started", total: event.data.contentLength ?? null });
        } else if (event.event === "Progress") {
          dispatch({ type: "download-progress", chunk: event.data.chunkLength ?? 0 });
        }
      });
      await relaunch();
    } catch (err) {
      dispatch({ type: "error", message: err instanceof Error ? err.message : String(err) });
    }
  };

  const handleRetryUpdate = async () => {
    dispatch({ type: "reset" });
    await handleCheckUpdate();
  };

  const showMandatoryOverlay =
    updater.status === "available" ||
    updater.status === "downloading" ||
    (updater.status === "error" && updater.pendingUpdate !== null);

  const progressPercent =
    updater.progress.total && updater.progress.total > 0
      ? Math.round((updater.progress.downloaded / updater.progress.total) * 100)
      : null;

  const toggleWorkspace = (folder: Folder, nextOpen: boolean) => {
    setExpandedWorkspaces((prev) => ({
      ...prev,
      [folder.path]: nextOpen,
    }));

    if (nextOpen) {
      void onRefreshWorkspaceChats?.(folder.path);
    }
  };

  return (
    <>
      <aside
        className={`flex flex-col h-full border-r border-border/50 transition-all duration-300 ease-in-out z-30 ${
          isCollapsed ? "w-16" : "w-64"
        } bg-sidebar`}
      >
        {/* Sidebar Header: Logo & Title */}
        <div
          data-tauri-drag-region
          className={`h-[72px] px-4 pt-8 pb-2 flex items-center shrink-0 overflow-hidden ${isCollapsed ? "justify-center" : "gap-3"}`}
        >
          <div
            className="size-8 bg-foreground shrink-0"
            style={{
              maskImage: `url(/whale-dnf.png)`,
              maskSize: "contain",
              maskRepeat: "no-repeat",
              maskPosition: "center",
              WebkitMaskImage: `url(/whale-dnf.png)`,
              WebkitMaskSize: "contain",
              WebkitMaskRepeat: "no-repeat",
              WebkitMaskPosition: "center",
            }}
          />
          {!isCollapsed && (
            <div className="flex flex-col min-w-0">
              <span className="font-bold text-sm tracking-tight truncate">
                Rainy MaTE
              </span>
            </div>
          )}
        </div>

        <div className="flex-1 overflow-y-auto overflow-x-hidden p-2.5 space-y-5 scrollbar-hide">
          {/* AI Studio Navigation */}
          <div className="space-y-1">
            <NavItem
              id="agent-chat"
              label="Agent Chat"
              icon={SquarePen}
              isActive={activeSection === "agent-chat"}
              isCollapsed={isCollapsed}
              onNavigate={onNavigate}
            />

            <NavItem
              id="neural-link"
              label="Agents ATM"
              icon={Clock}
              isActive={activeSection === "neural-link"}
              isCollapsed={isCollapsed}
              onNavigate={onNavigate}
            />
            {/* Keeping Agent Builder and Store as secondary premium items if user allows, but standardizing naming */}
            <NavItem
              id="agent-builder"
              label="Agent Builder"
              icon={Bot}
              isActive={activeSection === "agent-builder"}
              isCollapsed={isCollapsed}
              onNavigate={onNavigate}
            />
            <NavItem
              id="agent-store"
              label="Agents Store"
              icon={Library}
              isActive={activeSection === "agent-store"}
              isCollapsed={isCollapsed}
              onNavigate={onNavigate}
            />
            <NavItem
              id="wasm-skills"
              label="Wasm Skills"
              icon={LayoutGrid}
              isActive={activeSection === "wasm-skills"}
              isCollapsed={isCollapsed}
              onNavigate={onNavigate}
            />
            <NavItem
              id="memory-vault"
              label="Memory Vault"
              icon={BrainCircuit}
              isActive={activeSection === "memory-vault"}
              isCollapsed={isCollapsed}
              onNavigate={onNavigate}
            />
          </div>

          {folders.length > 0 && (
            <>
              <Separator className="bg-border/20 mx-1" />

              {/* Projects Section */}
              <div className="space-y-1">
                {!isCollapsed ? (
                  <div className="mb-1 flex items-center justify-between px-3 py-1">
                    <span className="text-[11px] font-semibold text-muted-foreground/70">
                      Projects
                    </span>
                    <Button
                      variant="ghost"
                      size="sm"
                      isIconOnly
                      className="size-7 rounded-xl text-muted-foreground/50 hover:bg-white/10 hover:text-foreground"
                      onPress={onAddFolder}
                    >
                      <Plus className="size-4" />
                    </Button>
                  </div>
                ) : null}

                <div className="space-y-0.5">
                  {folders.map((folder) => {
                    const isActive = folder.id === activeFolderId;
                    const Icon = folderIcons[folder.name] || FolderOpen;
                    const isExpanded = expandedWorkspaces[folder.path] ??
                      (folder.path === activeWorkspacePath || isActive);
                    const sessions = chatSessionsByWorkspace[folder.path] || [];

                    return isCollapsed ? (
                      <Tooltip key={folder.id} delay={0}>
                        <Button
                          variant={isActive ? "secondary" : "ghost"}
                          isIconOnly
                          className={`w-9 h-9 justify-center mx-auto rounded-xl transition-all duration-200 ${
                            isActive
                              ? "bg-primary/10 text-primary font-medium shadow-none"
                              : "text-muted-foreground hover:text-foreground hover:bg-white/10"
                          }`}
                          onPress={() => onFolderSelect?.(folder)}
                        >
                          <Icon className="size-4" />
                        </Button>
                        <Tooltip.Content placement="right">
                          {folder.name}
                        </Tooltip.Content>
                      </Tooltip>
                    ) : (
                      <Collapsible
                        key={folder.id}
                        open={isExpanded}
                        onOpenChange={(open) => toggleWorkspace(folder, open)}
                      >
                        <div className="rounded-2xl border border-white/6 bg-background/15">
                          <div
                            className={`flex w-full items-center gap-3 rounded-2xl px-3 py-2.5 text-left transition-colors ${
                              isActive
                                ? "bg-primary/10 text-primary"
                                : "text-muted-foreground hover:bg-white/8 hover:text-foreground"
                            }`}
                          >
                            <button
                              type="button"
                              className="flex min-w-0 flex-1 items-center gap-3 text-left"
                              onClick={() => void onFolderSelect?.(folder)}
                            >
                              <div className="flex items-center justify-center">
                                <Icon className="size-4" />
                              </div>
                              <div className="min-w-0 flex-1">
                                <div className="truncate text-[13px] font-medium">
                                  {folder.name}
                                </div>
                                <div className="truncate text-[10px] text-muted-foreground/70">
                                  {sessions.length} chat{sessions.length === 1 ? "" : "s"}
                                </div>
                              </div>
                            </button>
                            {isActive && (
                              <Button
                                variant="ghost"
                                size="sm"
                                isIconOnly
                                className="size-7 rounded-xl text-muted-foreground/55 hover:bg-white/10 hover:text-foreground"
                                onPress={() => void onRefreshWorkspaceChats?.(folder.path)}
                              >
                                <RefreshCw className="size-3.5" />
                              </Button>
                            )}
                            <Button
                              variant="ghost"
                              size="sm"
                              isIconOnly
                              className="size-7 rounded-xl text-muted-foreground/55 hover:bg-white/10 hover:text-foreground"
                              onPress={() => toggleWorkspace(folder, !isExpanded)}
                            >
                              <ChevronDown
                                className={`size-4 shrink-0 transition-transform ${
                                  isExpanded ? "rotate-180" : ""
                                }`}
                              />
                            </Button>
                          </div>

                          <CollapsibleContent className="overflow-hidden data-[state=closed]:animate-collapsible-up data-[state=open]:animate-collapsible-down">
                            <div className="space-y-2 px-2 pb-2 pt-1">
                              <ChatThreadList
                                sessions={sessions}
                                activeChatId={activeChatId ?? null}
                                onSwitchChat={(chatId) => onSelectChatForFolder?.(folder, chatId)}
                                onRefresh={isActive ? () => void onRefreshWorkspaceChats?.(folder.path) : undefined}
                                onDeleteChat={onDeleteChat ? (chatId) => onDeleteChat(folder.path, chatId) : undefined}
                                emptyLabel="No chats in this workspace"
                                showRefresh={isActive}
                              />
                            </div>
                          </CollapsibleContent>
                        </div>
                      </Collapsible>
                    );
                  })}

                  {isCollapsed && (
                    <Button
                      variant="ghost"
                      isIconOnly
                      className="w-9 h-9 justify-center mx-auto rounded-xl text-muted-foreground/50 hover:text-foreground hover:bg-white/10"
                      onPress={onAddFolder}
                    >
                      <Plus className="size-4 shrink-0" />
                    </Button>
                  )}
                </div>
              </div>
            </>
          )}
          </div>

        <div className="mt-auto p-2.5 space-y-1">
          <Separator className="bg-border/20 mx-1" />

          {/* Update Check Button */}
          {(() => {
            const isChecking = updater.status === "checking";
            const isAvailable = updater.status === "available";
            const isDownloading = updater.status === "downloading";
            const isUpToDate = updater.status === "up-to-date";
            const isError = updater.status === "error";

            const UpdateIcon = isUpToDate
              ? Check
              : isError
                ? AlertCircle
                : isAvailable
                  ? Download
                  : RefreshCw;

            const label = isChecking
              ? "Checking…"
              : isAvailable
                ? `Update v${updater.version}`
                : isDownloading
                  ? "Installing…"
                  : isUpToDate
                    ? "Up to date"
                    : isError
                      ? "Check failed"
                      : "Check Updates";

            const handlePress = isAvailable
              ? handleInstallUpdate
              : handleCheckUpdate;
            const isBusy = isChecking || isDownloading;

            const btn = (
              <Button
                variant={isAvailable ? "secondary" : "ghost"}
                isIconOnly={isCollapsed}
                isDisabled={isBusy}
                className={`transition-all duration-200 ${
                  isCollapsed
                    ? "w-9 h-9 justify-center mx-auto rounded-xl"
                    : "w-full justify-start gap-3 h-9 px-3 rounded-xl"
                } ${
                  isUpToDate
                    ? "text-green-500"
                    : isError
                      ? "text-red-400"
                      : isAvailable
                        ? "text-primary bg-primary/10 font-medium shadow-none"
                        : "text-muted-foreground hover:text-foreground hover:bg-white/10"
                }`}
                onPress={handlePress}
              >
                <UpdateIcon
                  className={`size-4 shrink-0 ${isBusy ? "animate-spin" : ""}`}
                />
                {!isCollapsed && (
                  <span className="truncate flex-1 text-left text-[13px] font-medium tracking-tight">
                    {label}
                  </span>
                )}
              </Button>
            );

            return isCollapsed ? (
              <Tooltip delay={0}>
                {btn}
                <Tooltip.Content placement="right">{label}</Tooltip.Content>
              </Tooltip>
            ) : (
              btn
            );
          })()}

          {/* User / Settings Footer - The "monstrosity" fix */}
          <div
            className={`mt-1 flex items-center transition-all ${isCollapsed ? "flex-col gap-3 py-1" : "justify-between gap-1 px-1"}`}
          >
            <Button
              variant="ghost"
              size="sm"
              isIconOnly={isCollapsed}
              onPress={onSettingsClick}
              className={`transition-all duration-200 ${
                isCollapsed
                  ? "w-9 h-9 justify-center mx-auto rounded-xl"
                  : "flex-1 justify-start gap-3 h-9 px-3 rounded-xl"
              } text-muted-foreground hover:text-foreground hover:bg-white/10`}
            >
              <Settings className="size-4 shrink-0" />
              {!isCollapsed && <span className="truncate text-[13px] font-medium tracking-tight">Settings</span>}
            </Button>
            
            <Button
              variant="ghost"
              size="sm"
              isIconOnly
              onPress={onToggleCollapse}
              className="text-muted-foreground/40 hover:bg-white/10 h-8 w-8 rounded-lg shrink-0"
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

      {/* Mandatory Update Overlay — non-dismissable, blocks the entire app */}
      {showMandatoryOverlay && (
        <MandatoryUpdateOverlay
          phase={
            updater.status === "downloading"
              ? "downloading"
              : updater.status === "error"
                ? "error"
                : "available"
          }
          currentVersion={updater.currentVersion}
          newVersion={updater.version}
          progressPercent={progressPercent}
          errorMsg={updater.error}
          onInstall={handleInstallUpdate}
          onRetry={handleRetryUpdate}
        />
      )}
    </>
  );
}
