import { useState } from "react";
import { Tooltip, Button, Separator } from "@heroui/react";
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
  Plus,
  Settings,
  Network,
  Bot,
  Library,
  RefreshCw,
  Check,
  AlertCircle,
} from "lucide-react";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
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

type UpdateStatus =
  | "idle"
  | "checking"
  | "available"
  | "downloading"
  | "up-to-date"
  | "error";

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
  const [updateStatus, setUpdateStatus] = useState<UpdateStatus>("idle");
  const [updateVersion, setUpdateVersion] = useState<string>("");
  const [currentVersion, setCurrentVersion] = useState<string>("");
  const [pendingUpdate, setPendingUpdate] = useState<Awaited<
    ReturnType<typeof check>
  > | null>(null);
  const [downloadProgress, setDownloadProgress] = useState<{
    downloaded: number;
    total: number | null;
  }>({ downloaded: 0, total: null });
  const [errorMsg, setErrorMsg] = useState<string>("");

  const handleCheckUpdate = async () => {
    if (updateStatus === "checking" || updateStatus === "downloading") return;
    setUpdateStatus("checking");
    setErrorMsg("");
    try {
      const update = await check();
      if (update) {
        setUpdateVersion(update.version);
        setCurrentVersion(update.currentVersion);
        setPendingUpdate(update);
        setUpdateStatus("available");
      } else {
        setUpdateStatus("up-to-date");
        setTimeout(() => setUpdateStatus("idle"), 3000);
      }
    } catch (err) {
      setErrorMsg(err instanceof Error ? err.message : String(err));
      setUpdateStatus("error");
      setTimeout(() => setUpdateStatus("idle"), 3000);
    }
  };

  const handleInstallUpdate = async () => {
    if (!pendingUpdate) return;
    setUpdateStatus("downloading");
    setDownloadProgress({ downloaded: 0, total: null });
    try {
      await pendingUpdate.downloadAndInstall((event) => {
        if (event.event === "Started") {
          setDownloadProgress({
            downloaded: 0,
            total: event.data.contentLength ?? null,
          });
        } else if (event.event === "Progress") {
          setDownloadProgress((prev) => ({
            ...prev,
            downloaded: prev.downloaded + (event.data.chunkLength ?? 0),
          }));
        }
      });
      await relaunch();
    } catch (err) {
      setErrorMsg(err instanceof Error ? err.message : String(err));
      setUpdateStatus("error");
    }
  };

  const handleRetryUpdate = async () => {
    setErrorMsg("");
    setDownloadProgress({ downloaded: 0, total: null });
    await handleCheckUpdate();
  };

  const showMandatoryOverlay =
    updateStatus === "available" ||
    updateStatus === "downloading" ||
    (updateStatus === "error" && pendingUpdate !== null);

  const progressPercent =
    downloadProgress.total && downloadProgress.total > 0
      ? Math.round((downloadProgress.downloaded / downloadProgress.total) * 100)
      : null;
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
    <>
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
              id="neural-link"
              label="Rainy ATM"
              icon={Network}
              colorClass="text-purple-500"
            />
            <NavItem
              id="agent-builder"
              label="Agent Builder"
              icon={Bot}
              colorClass="text-orange-500"
            />
            <NavItem
              id="agent-store"
              label="Agents Store"
              icon={Library}
              colorClass="text-amber-500"
            />
          </div>
        </div>

        <div className="mt-auto p-3 space-y-2">
          <Separator className="bg-border/30" />

          {/* Settings Submenu */}
          <div className="space-y-1 pt-2">
            <NavItem id="settings-models" label="AI Provider" icon={Sparkles} />
            <NavItem
              id="settings-appearance"
              label="Appearance"
              icon={Palette}
            />
          </div>

          <Separator className="bg-border/30" />

          {/* Update Check Button */}
          {(() => {
            const isChecking = updateStatus === "checking";
            const isAvailable = updateStatus === "available";
            const isDownloading = updateStatus === "downloading";
            const isUpToDate = updateStatus === "up-to-date";
            const isError = updateStatus === "error";

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
                ? `Update v${updateVersion}`
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
                    ? "w-10 h-10 justify-center mx-auto rounded-xl"
                    : "w-full justify-start gap-3 h-10 px-3"
                } ${
                  isUpToDate
                    ? "text-green-500"
                    : isError
                      ? "text-red-400"
                      : isAvailable
                        ? "text-primary bg-primary/10 font-medium"
                        : "text-muted-foreground hover:text-foreground hover:bg-muted/50"
                }`}
                onPress={handlePress}
              >
                <UpdateIcon
                  className={`size-4 shrink-0 ${isBusy ? "animate-spin" : ""}`}
                />
                {!isCollapsed && (
                  <span className="truncate flex-1 text-left text-xs">
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

          {/* User / Settings Footer */}
          <div
            className={`mt-2 flex items-center transition-all ${isCollapsed ? "flex-col gap-4 py-2" : "px-1 gap-3 py-2"}`}
          >
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

      {/* Mandatory Update Overlay — non-dismissable, blocks the entire app */}
      {showMandatoryOverlay && (
        <div
          style={{
            position: "fixed",
            inset: 0,
            zIndex: 99999,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            backgroundColor: "rgba(0, 0, 0, 0.85)",
            backdropFilter: "blur(12px)",
            WebkitBackdropFilter: "blur(12px)",
          }}
        >
          <div
            style={{
              width: "100%",
              maxWidth: 420,
              padding: "2.5rem 2rem",
              borderRadius: 16,
              background: "linear-gradient(145deg, #0d1117 0%, #161b22 100%)",
              border: "1px solid rgba(74, 222, 128, 0.15)",
              boxShadow:
                "0 25px 50px -12px rgba(0, 0, 0, 0.6), 0 0 80px rgba(74, 222, 128, 0.05)",
              textAlign: "center" as const,
              color: "#e6edf3",
              fontFamily: "'Inter', -apple-system, sans-serif",
            }}
          >
            {/* Icon */}
            <div style={{ marginBottom: "1rem" }}>
              <svg
                width="48"
                height="48"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.5"
                strokeLinecap="round"
                strokeLinejoin="round"
                style={{ color: "#4ade80" }}
              >
                <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                <polyline points="7 10 12 15 17 10" />
                <line x1="12" y1="15" x2="12" y2="3" />
              </svg>
            </div>

            <h2
              style={{
                fontSize: "1.5rem",
                fontWeight: 700,
                margin: "0 0 0.5rem 0",
                color: "#ffffff",
              }}
            >
              Update Required
            </h2>

            {/* Available state */}
            {updateStatus === "available" && (
              <>
                <p
                  style={{
                    fontSize: "0.9rem",
                    color: "#8b949e",
                    margin: "0 0 1.25rem 0",
                    lineHeight: 1.5,
                  }}
                >
                  A new version of <strong>Rainy MaTE</strong> is available.
                </p>
                <div
                  style={{
                    display: "flex",
                    justifyContent: "space-between",
                    padding: "0.5rem 1rem",
                    borderRadius: 8,
                    backgroundColor: "rgba(255,255,255,0.04)",
                    marginBottom: "0.4rem",
                  }}
                >
                  <span style={{ fontSize: "0.85rem", color: "#8b949e" }}>
                    Current
                  </span>
                  <span
                    style={{
                      fontSize: "0.85rem",
                      fontWeight: 600,
                      fontFamily: "'SF Mono', 'Fira Code', monospace",
                      color: "#e6edf3",
                    }}
                  >
                    {currentVersion}
                  </span>
                </div>
                <div
                  style={{
                    display: "flex",
                    justifyContent: "space-between",
                    padding: "0.5rem 1rem",
                    borderRadius: 8,
                    backgroundColor: "rgba(255,255,255,0.04)",
                    marginBottom: "0.4rem",
                  }}
                >
                  <span style={{ fontSize: "0.85rem", color: "#8b949e" }}>
                    New
                  </span>
                  <span
                    style={{
                      fontSize: "0.85rem",
                      fontWeight: 600,
                      fontFamily: "'SF Mono', 'Fira Code', monospace",
                      color: "#4ade80",
                    }}
                  >
                    {updateVersion}
                  </span>
                </div>
                <button
                  style={{
                    marginTop: "1.5rem",
                    width: "100%",
                    padding: "0.75rem 1.5rem",
                    border: "none",
                    borderRadius: 10,
                    background:
                      "linear-gradient(135deg, #22c55e 0%, #16a34a 100%)",
                    color: "#ffffff",
                    fontSize: "0.95rem",
                    fontWeight: 600,
                    cursor: "pointer",
                    boxShadow: "0 4px 14px rgba(34, 197, 94, 0.3)",
                  }}
                  onClick={handleInstallUpdate}
                >
                  Update Now
                </button>
                <p
                  style={{
                    marginTop: "0.75rem",
                    fontSize: "0.7rem",
                    color: "#6e7681",
                    fontStyle: "italic",
                  }}
                >
                  This update is required to continue using the app.
                </p>
              </>
            )}

            {/* Downloading state */}
            {updateStatus === "downloading" && (
              <>
                <p
                  style={{
                    fontSize: "0.9rem",
                    color: "#8b949e",
                    margin: "0 0 1.25rem 0",
                  }}
                >
                  Downloading update...
                </p>
                <div
                  style={{
                    width: "100%",
                    height: 6,
                    borderRadius: 3,
                    backgroundColor: "rgba(255,255,255,0.08)",
                    overflow: "hidden",
                  }}
                >
                  <div
                    style={{
                      height: "100%",
                      borderRadius: 3,
                      background: "linear-gradient(90deg, #22c55e, #4ade80)",
                      transition: "width 0.3s ease",
                      width:
                        progressPercent !== null
                          ? `${progressPercent}%`
                          : "60%",
                      animation:
                        progressPercent === null
                          ? "updater-pulse 1.5s ease-in-out infinite"
                          : "none",
                    }}
                  />
                </div>
                {progressPercent !== null && (
                  <p
                    style={{
                      marginTop: "0.5rem",
                      fontSize: "0.85rem",
                      color: "#8b949e",
                      fontFamily: "'SF Mono', 'Fira Code', monospace",
                    }}
                  >
                    {progressPercent}%
                  </p>
                )}
              </>
            )}

            {/* Error state (within mandatory overlay) */}
            {updateStatus === "error" && (
              <>
                <p
                  style={{
                    fontSize: "0.9rem",
                    color: "#f87171",
                    margin: "0 0 1.25rem 0",
                  }}
                >
                  Update failed
                </p>
                {errorMsg && (
                  <p
                    style={{
                      fontSize: "0.8rem",
                      color: "#f87171",
                      backgroundColor: "rgba(248, 113, 113, 0.08)",
                      padding: "0.5rem 0.75rem",
                      borderRadius: 6,
                      marginBottom: "0.5rem",
                      fontFamily: "'SF Mono', 'Fira Code', monospace",
                      wordBreak: "break-all" as const,
                    }}
                  >
                    {errorMsg}
                  </p>
                )}
                <button
                  style={{
                    marginTop: "1rem",
                    width: "100%",
                    padding: "0.75rem 1.5rem",
                    border: "none",
                    borderRadius: 10,
                    background:
                      "linear-gradient(135deg, #22c55e 0%, #16a34a 100%)",
                    color: "#ffffff",
                    fontSize: "0.95rem",
                    fontWeight: 600,
                    cursor: "pointer",
                    boxShadow: "0 4px 14px rgba(34, 197, 94, 0.3)",
                  }}
                  onClick={handleRetryUpdate}
                >
                  Retry
                </button>
              </>
            )}
          </div>

          <style>{`
            @keyframes updater-pulse {
              0%, 100% { opacity: 0.6; }
              50% { opacity: 1; }
            }
          `}</style>
        </div>
      )}
    </>
  );
}
