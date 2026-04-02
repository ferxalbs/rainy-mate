import { useState, useCallback, useEffect, useMemo, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { UpdateChecker } from "./components/updater/UpdateChecker";
import { TahoeLayout, AIDocumentPanel, AIResearchPanel } from "./components";
import { SettingsPage } from "./components/settings";
import { AgentChatPanel } from "./components/agent-chat/AgentChatPanel";
import { NeuralPanel, AirlockEvents, McpApprovalEvents } from "./components/neural";
import { WorkspaceLaunchpad, WorkspaceRecurringRuns } from "./components/workspace";
import { AgentBuilder } from "./components/agents/builder/AgentBuilder";
import { AgentStorePage } from "./components/agents/store/AgentStorePage";
import { WasmSkillsPage } from "./components/wasm-skills/WasmSkillsPage";
import { MemoryExplorerPanel } from "./components/memory/MemoryExplorerPanel";
import { Toaster } from "sonner";
import { AlertCircle, FolderPlus } from "lucide-react";
import { useAIProvider, useFolderManager } from "./hooks";
import { useChatSessions } from "./hooks/useChatSessions";
import { useDesktopNotifications } from "./hooks/useDesktopNotifications";
import type { Folder } from "./types";
import type { AgentSpec } from "./types/agent-spec";
import * as tauri from "./services/tauri";
import { useCloudEvents } from "./hooks/useCloudEvents";
import { Button } from "./components/ui/button";

function deriveFolderNameFromPath(path: string): string {
  const segments = path.split(/[\\/]/).filter(Boolean);
  return segments[segments.length - 1] || path;
}

function App() {
  const { refreshProviders } = useAIProvider();
  useCloudEvents();
  useDesktopNotifications();

  // Folder management hook
  const {
    folders: userFolders,
    addFolder,
    refreshFolders,
  } = useFolderManager();

  // Convert UserFolder to Folder type for sidebar — memoized to stabilize effect dependencies
  const folders: Folder[] = useMemo(
    () => userFolders.map((uf) => ({
      id: uf.id,
      path: uf.path,
      name: uf.name,
      accessType: uf.accessType,
    })),
    [userFolders],
  );

  const [activeSection, setActiveSection] = useState("agent-chat");
  const [activeFolder, setActiveFolder] = useState<Folder | null>(null);
  const [agentBuilderInitialSpec, setAgentBuilderInitialSpec] = useState<
    AgentSpec | undefined
  >(undefined);
  const [remoteSessionBinding, setRemoteSessionBinding] = useState<tauri.RemoteSessionBinding | null>(null);

  const [submitError, setSubmitError] = useState<string | null>(null);
  const [pendingWorkspaceLaunch, setPendingWorkspaceLaunch] = useState<{
    requestId: string;
    prompt: string;
    preflight: tauri.WorkspaceLaunchPreflight;
    scenarioId: string;
    workspaceId: string;
    chatId: string;
  } | null>(null);
  const consumedLaunchRequestIdsRef = useRef<Set<string>>(new Set());

  const {
    sessionsByWorkspace,
    activeChatId,
    activeRunChatIds,
    createNewChat,
    switchToChat,
    deleteChat,
    refreshWorkspaceSessions,
  } = useChatSessions({
    activeWorkspaceId: activeFolder?.path || "default",
  });

  // Inspector State Removed

  // Load providers on mount
  useEffect(() => {
    refreshProviders();
  }, [refreshProviders]);

  // Handle folder selection
  const handleFolderSelect = useCallback(
    async (folder: Folder) => {
      try {
        await tauri.setWorkspace(folder.path, folder.name);
        await tauri.updateFolderAccess(folder.id);
        setActiveFolder(folder);
        refreshFolders();
        console.log("Workspace set:", folder);
      } catch (err) {
        console.error("Failed to set workspace:", err);
      }
    },
    [refreshFolders],
  );

  // Auto-select first folder when folders are loaded — uses async callback, not synchronous setState
  const didAutoSelect = useRef(false);
  useEffect(() => {
    if (folders.length > 0 && !didAutoSelect.current) {
      didAutoSelect.current = true;
      const folder = folders[0];
      tauri.setWorkspace(folder.path, folder.name)
        .then(() => tauri.updateFolderAccess(folder.id))
        .then(() => {
          setActiveFolder(folder);
          refreshFolders();
        })
        .catch((err) => console.error("Failed to auto-select workspace:", err));
    }
  }, [folders, refreshFolders]);

  // Handle navigation
  const handleNavigate = useCallback((section: string) => {
    if (section === "agent-builder") {
      setAgentBuilderInitialSpec(undefined);
    }
    setActiveSection(section);
  }, []);

  const handleOpenAgentBuilder = useCallback((spec?: AgentSpec) => {
    setAgentBuilderInitialSpec(spec);
    setActiveSection("agent-builder");
  }, []);

  // Handle settings click from sidebar - Redundant now loop logic if needed or remove
  const handleSettingsClick = useCallback(() => {
    // setSettingsOpen(true); Removed modal trigger
    // Maybe navigate to settings page instead?
    handleNavigate("settings-models");
  }, [handleNavigate]);

  const handleSelectChatForFolder = useCallback(async (folder: Folder, chatId: string) => {
    if (activeFolder?.id !== folder.id) {
      await handleFolderSelect(folder);
    }
    switchToChat(chatId);
  }, [activeFolder?.id, handleFolderSelect, switchToChat]);

  const ensureImportedFolder = useCallback(
    async (workspacePath: string, workspaceName?: string): Promise<Folder> => {
      const existing = folders.find((folder) => folder.path === workspacePath);
      if (existing) return existing;

      const imported = await tauri.addUserFolder(
        workspacePath,
        workspaceName || deriveFolderNameFromPath(workspacePath),
      );
      void refreshFolders();
      return {
        id: imported.id,
        path: imported.path,
        name: imported.name,
        accessType: imported.accessType,
      };
    },
    [folders, refreshFolders],
  );

  const handleCreateNewChat = useCallback(async () => {
    const workspaceId = activeFolder?.path || "default";
    const chat = await createNewChat(workspaceId);
    if (chat) {
      switchToChat(chat.id);
    }
  }, [activeFolder?.path, createNewChat, switchToChat]);

  const handleRunWorkspaceScenario = useCallback(async (scenarioId: string) => {
    if (!activeFolder) return;
    try {
      const workspacePath = activeFolder.path;
      const prepared = await tauri.prepareWorkspaceLaunch(workspacePath, scenarioId);
      const chat = await createNewChat(activeFolder.path);
      if (!chat) {
        throw new Error("Failed to create or reuse a chat for the guided run.");
      }
      switchToChat(chat.id);
      setActiveSection("agent-chat");
      setPendingWorkspaceLaunch({
        requestId: prepared.requestId,
        prompt: prepared.prompt,
        preflight: prepared.preflight,
        scenarioId,
        workspaceId: workspacePath,
        chatId: chat.id,
      });
    } catch (error) {
      setSubmitError(error instanceof Error ? error.message : "Failed to launch workspace scenario.");
    }
  }, [activeFolder, createNewChat, switchToChat]);

  // Check if we're in Settings section
  const isSettingsSection = activeSection.startsWith("settings-");
  const settingsTab = isSettingsSection
    ? activeSection.replace("settings-", "")
    : "models";

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | null = null;

    void listen<{ workspaceId: string; chatId: string }>(
      "agent:notification_clicked",
      async (event) => {
        if (cancelled) return;

        const { workspaceId, chatId } = event.payload;
        setActiveSection("agent-chat");
        await refreshWorkspaceSessions(workspaceId);

        const matchingFolder = folders.find((folder) => folder.path === workspaceId);
        if (matchingFolder) {
          await handleSelectChatForFolder(matchingFolder, chatId);
          return;
        }

        switchToChat(chatId);
      },
    ).then((fn) => {
      if (cancelled) {
        fn();
      } else {
        unlisten = fn;
      }
    });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [folders, handleSelectChatForFolder, refreshWorkspaceSessions, switchToChat]);

  useEffect(() => {
    let cancelled = false;
    let startedUnlisten: (() => void) | null = null;
    let finishedUnlisten: (() => void) | null = null;

    void listen<tauri.RemoteSessionStartedEvent>("session://started", async (event) => {
      if (cancelled) return;
      const payload = event.payload;
      if (payload.source !== "remote") return;
      const workspacePath = payload.workspacePath || payload.workspaceId;
      if (!workspacePath) return;

      setActiveSection("agent-chat");
      void ensureImportedFolder(workspacePath, payload.workspaceName)
        .then(async (folder) => {
          if (cancelled) return;

          setRemoteSessionBinding({
            ...payload,
            workspaceId: payload.workspaceId || workspacePath,
            workspacePath,
            workspaceName: payload.workspaceName || folder.name,
          });

          await handleSelectChatForFolder(folder, payload.chatId);
        })
        .catch((error) => {
          console.error("Failed to bind remote session:", error);
        });
    }).then((fn) => {
      if (cancelled) {
        fn();
      } else {
        startedUnlisten = fn;
      }
    });

    void listen<tauri.RemoteSessionFinishedEvent>("session://finished", async (event) => {
      if (cancelled) return;
      const payload = event.payload;
      if (payload.source !== "remote") return;
      const finishedWorkspacePath = payload.workspacePath || payload.workspaceId;
      if (!finishedWorkspacePath) return;

      setRemoteSessionBinding((current) => {
        if (!current) return null;
        const matchesRun = payload.runId && current.runId === payload.runId;
        const matchesChat = current.chatId && current.chatId === payload.chatId;
        const matchesWorkspace =
          current.workspacePath === finishedWorkspacePath ||
          current.workspaceId === payload.workspaceId;
        if (matchesRun || (matchesChat && matchesWorkspace)) {
          return null;
        }
        return current;
      });
    }).then((fn) => {
      if (cancelled) {
        fn();
      } else {
        finishedUnlisten = fn;
      }
    });

    return () => {
      cancelled = true;
      startedUnlisten?.();
      finishedUnlisten?.();
    };
  }, [ensureImportedFolder, handleSelectChatForFolder]);

  return (
    <>
      {/* Mandatory Update Checker — non-dismissable overlay */}
      <UpdateChecker />
      <TahoeLayout
        folders={folders}
        activeFolderId={activeFolder?.id}
        workspacePath={activeFolder?.path}
        onFolderSelect={handleFolderSelect}
        onAddFolder={addFolder}
        onNavigate={handleNavigate}
        onSettingsClick={handleSettingsClick}
        activeSection={activeSection}
        isImmersive={
          activeSection !== "documents" &&
          activeSection !== "research" &&
          activeSection !== "atm-bootstrap" &&
          activeSection !== "workspace-launchpad"
        }
        chatSessionsByWorkspace={sessionsByWorkspace}
        activeWorkspacePath={activeFolder?.path}
        activeChatId={activeChatId}
        activeRunChatIds={activeRunChatIds}
        onSelectChatForFolder={handleSelectChatForFolder}
        onRefreshWorkspaceChats={(workspaceId) => void refreshWorkspaceSessions(workspaceId)}
        onDeleteChat={deleteChat}
      >
        {/* Main Content Area - Dynamic based on section */}
        <div className="h-full w-full flex flex-col">
          {/* Error Display */}
          {submitError && (
            <div className="p-4 border border-red-200 dark:border-red-800 bg-red-50 dark:bg-red-950/30 rounded-xl m-4">
              <div className="flex items-center gap-2 text-red-600 dark:text-red-400">
                <AlertCircle className="size-4 shrink-0" />
                <p className="text-sm">{submitError}</p>
                <Button
                  variant="secondary"
                  size="sm"
                  className="ml-auto"
                  onClick={() => {
                    setSubmitError(null);
                  }}
                >
                  Dismiss
                </Button>
              </div>
            </div>
          )}

          {/* AI Documents */}
          {activeSection === "documents" && (
            <div className="flex-1 p-6">
              {activeFolder ? (
                <AIDocumentPanel />
              ) : (
                <NoFolderGate onAddFolder={addFolder} />
              )}
            </div>
          )}

          {/* AI Research */}
          {activeSection === "research" && (
            <div className="flex-1 p-6">
              {activeFolder ? (
                <AIResearchPanel />
              ) : (
                <NoFolderGate onAddFolder={addFolder} />
              )}
            </div>
          )}

          {/* Neural Link */}
          {activeSection === "neural-link" && (
            <div className="flex-1 overflow-auto bg-content1/50">
              <NeuralPanel onNavigate={handleNavigate} />
            </div>
          )}

          {activeSection === "workspace-launchpad" && (
            <div className="flex-1 h-full min-h-0 overflow-hidden">
              {activeFolder ? (
                <WorkspaceLaunchpad
                  workspacePath={activeFolder.path}
                  onRunScenario={handleRunWorkspaceScenario}
                />
              ) : (
                <NoFolderGate onAddFolder={addFolder} />
              )}
            </div>
          )}

          {activeSection === "workspace-recurring-runs" && (
            <div className="flex-1 h-full min-h-0 overflow-hidden">
              {activeFolder ? (
                <WorkspaceRecurringRuns workspacePath={activeFolder.path} />
              ) : (
                <NoFolderGate onAddFolder={addFolder} />
              )}
            </div>
          )}

          {/* Agent Builder */}
          {activeSection === "agent-builder" && (
            <div className="flex-1 h-full min-h-0">
              <AgentBuilder
                onBack={() => handleNavigate("agent-chat")}
                initialSpec={agentBuilderInitialSpec}
                workspacePath={activeFolder?.path}
              />
            </div>
          )}

          {/* Agents Store */}
          {activeSection === "agent-store" && (
            <div className="flex-1 h-full min-h-0">
              <AgentStorePage
                workspacePath={activeFolder?.path}
                onCreateAgent={() => handleOpenAgentBuilder()}
                onEditInBuilder={(spec) => handleOpenAgentBuilder(spec)}
              />
            </div>
          )}

          {/* Wasm Skills Sandbox */}
          {activeSection === "wasm-skills" && (
            <div className="flex-1 h-full min-h-0 overflow-hidden">
              <WasmSkillsPage />
            </div>
          )}

          {/* Memory Vault Explorer */}
          {activeSection === "memory-vault" && (
            <div className="flex-1 h-full min-h-0 overflow-hidden">
              <MemoryExplorerPanel />
            </div>
          )}

          {/* Settings Section */}
          {isSettingsSection && (
            <div className="flex-1 h-full min-h-0 overflow-hidden">
              <SettingsPage
                initialTab={settingsTab}
                onBack={() => handleNavigate("agent-chat")}
              />
            </div>
          )}

          {/* Agent Chat Main View - Full Height */}
          {activeSection === "agent-chat" && (
            <div className="flex-1 h-full min-h-0">
              {activeFolder ? (
              <AgentChatPanel
                  key={activeChatId || activeFolder.path}
                  workspacePath={activeFolder.path}
                  folders={folders}
                  activeFolderId={activeFolder.id}
                  onSelectWorkspace={handleFolderSelect}
                  onAddWorkspace={addFolder}
                  onOpenSettings={handleSettingsClick}
                  chatScopeId={activeChatId}
                  remoteSessionBinding={remoteSessionBinding}
                  pendingLaunch={pendingWorkspaceLaunch}
                  onPendingLaunchConsumed={(requestId) => {
                    if (consumedLaunchRequestIdsRef.current.has(requestId)) {
                      return false;
                    }

                    consumedLaunchRequestIdsRef.current.add(requestId);
                    setPendingWorkspaceLaunch((current) =>
                      current?.requestId === requestId ? null : current,
                    );
                    return true;
                  }}
                  onPendingLaunchCompleted={async (result) => {
                    await tauri.recordWorkspaceLaunchResult(
                      result.workspaceId,
                      result.requestId,
                      result.scenarioId,
                      result.chatId,
                      result.success,
                      result.actualToolIds,
                      result.actualTouchedPaths,
                      result.producedArtifactPaths,
                    );
                  }}
                  onNewChat={handleCreateNewChat}
                  onRefreshSessions={async () => {
                    await refreshWorkspaceSessions(activeFolder?.path || "default");
                  }}
                />
              ) : (
                <div className="flex items-center justify-center h-full">
                  <NoFolderGate onAddFolder={addFolder} />
                </div>
              )}
            </div>
          )}
        </div>
      </TahoeLayout>

      {/* Toast Container for notifications */}
      {/* Toast Container for notifications */}
      <Toaster richColors position="bottom-right" theme="system" />
      <AirlockEvents />
      <McpApprovalEvents />
    </>
  );
}

/**
 * Gate component shown when no folder is selected.
 * Prompts user to select a project folder before using the system.
 */
function NoFolderGate({ onAddFolder }: { onAddFolder: () => void }) {
  return (
    <div className="mx-auto max-w-lg animate-appear rounded-[32px] border border-white/10 bg-background/80 p-12 text-center shadow-[0_24px_90px_rgba(0,0,0,0.14)] backdrop-blur-2xl backdrop-saturate-150 dark:bg-background/20">
      <div className="space-y-4">
        <div className="size-16 mx-auto bg-primary/10 rounded-2xl flex items-center justify-center">
          <FolderPlus className="size-8 text-primary" />
        </div>
        <div className="space-y-2">
          <h2 className="text-lg font-semibold text-foreground">
            Select a Project Folder
          </h2>
          <p className="text-sm text-muted-foreground max-w-sm mx-auto">
            To get started, select a folder where Rainy MaTE will work. All
            files, documents, and AI-generated content will be saved there.
          </p>
        </div>
        <Button
          size="lg"
          onClick={onAddFolder}
          className="mt-2 rounded-full px-5 font-medium"
        >
          <FolderPlus className="size-4" />
          Add Folder
        </Button>
      </div>
    </div>
  );
}

export default App;
