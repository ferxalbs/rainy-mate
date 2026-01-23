import { useState, useCallback, useEffect } from "react";
import {
  TahoeLayout,
  TaskCard,
  SettingsPanel,
  AIDocumentPanel,
  AIResearchPanel,
} from "./components";
import { SettingsPage } from "./components/settings";
import { CoworkPanel } from "./components/cowork";
import { Button, Card } from "@heroui/react";
import {
  Zap,
  CheckCircle2,
  ListTodo,
  AlertCircle,
  FileText,
  Search,
  FolderPlus,
} from "lucide-react";
import { useTauriTask, useAIProvider, useFolderManager } from "./hooks";
import type { Task, Folder } from "./types";
import * as tauri from "./services/tauri";

function App() {
  // Tauri hooks
  const {
    tasks,
    error: taskError,
    pauseTask,
    resumeTask,
    cancelTask,
    refreshTasks,
  } = useTauriTask();

  const { refreshProviders } = useAIProvider();

  // Folder management hook
  const {
    folders: userFolders,
    addFolder,
    refreshFolders,
  } = useFolderManager();

  // Convert UserFolder to Folder type for sidebar
  const folders: Folder[] = userFolders.map((uf) => ({
    id: uf.id,
    path: uf.path,
    name: uf.name,
    accessType: uf.accessType,
  }));

  const [activeSection, setActiveSection] = useState("running");
  const [activeFolder, setActiveFolder] = useState<Folder | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [submitError, setSubmitError] = useState<string | null>(null);

  // Load tasks on mount
  useEffect(() => {
    refreshTasks();
    refreshProviders();
  }, [refreshTasks, refreshProviders]);

  // Calculate task counts for sidebar
  const taskCounts = {
    completed: tasks.filter((t) => t.status === "completed").length,
    running: tasks.filter(
      (t) => t.status === "running" || t.status === "paused",
    ).length,
    queued: tasks.filter((t) => t.status === "queued").length,
  };

  // Handle task pause
  const handleTaskPause = useCallback(
    async (taskId: string) => {
      const task = tasks.find((t) => t.id === taskId);
      if (!task) return;

      try {
        if (task.status === "paused") {
          await resumeTask(taskId);
        } else {
          await pauseTask(taskId);
        }
      } catch (err) {
        console.error("Failed to pause/resume task:", err);
      }
    },
    [tasks, pauseTask, resumeTask],
  );

  // Handle task stop
  const handleTaskStop = useCallback(
    async (taskId: string) => {
      try {
        await cancelTask(taskId);
      } catch (err) {
        console.error("Failed to cancel task:", err);
      }
    },
    [cancelTask],
  );

  // Handle folder selection
  const handleFolderSelect = useCallback(
    async (folder: Folder) => {
      try {
        await tauri.setWorkspace(folder.path, folder.name);
        await tauri.updateFolderAccess(folder.id);
        setActiveFolder(folder);
        // Refresh folders to get new ordering (most recent first)
        refreshFolders();
        console.log("Workspace set:", folder);
      } catch (err) {
        console.error("Failed to set workspace:", err);
      }
    },
    [refreshFolders],
  );

  // Handle navigation
  const handleNavigate = useCallback((section: string) => {
    setActiveSection(section);
  }, []);

  // Handle settings click from sidebar
  const handleSettingsClick = useCallback(() => {
    setSettingsOpen(true);
  }, []);

  // Filter tasks based on active section
  const getDisplayTasks = () => {
    switch (activeSection) {
      case "completed":
        return tasks.filter((t) => t.status === "completed");
      case "running":
        return tasks.filter(
          (t) => t.status === "running" || t.status === "paused",
        );
      case "queued":
        return tasks.filter((t) => t.status === "queued");
      default:
        return tasks.filter(
          (t) => t.status === "running" || t.status === "paused",
        );
    }
  };

  // Convert Tauri tasks to local Task type for TaskCard
  const convertTask = (t: (typeof tasks)[0]): Task => ({
    ...t,
    createdAt: new Date(t.createdAt),
    startedAt: t.startedAt ? new Date(t.startedAt) : undefined,
    completedAt: t.completedAt ? new Date(t.completedAt) : undefined,
  });

  const displayTasks = getDisplayTasks();
  const sectionTitle = {
    running: {
      icon: <Zap className="size-5 text-blue-500 shrink-0" />,
      label: "Active Tasks",
    },
    completed: {
      icon: <CheckCircle2 className="size-5 text-green-500 shrink-0" />,
      label: "Completed Tasks",
    },
    queued: {
      icon: <ListTodo className="size-5 text-orange-500 shrink-0" />,
      label: "Queued Tasks",
    },
    documents: {
      icon: <FileText className="size-5 text-accent shrink-0" />,
      label: "AI Documents",
    },
    research: {
      icon: <Search className="size-5 text-accent shrink-0" />,
      label: "AI Research",
    },
  }[activeSection] || {
    icon: <Zap className="size-5 text-blue-500 shrink-0" />,
    label: "Tasks",
  };

  // Check if we're in AI Studio section
  const isAIStudioSection =
    activeSection === "documents" ||
    activeSection === "research" ||
    activeSection === "cowork";

  // Check if we're in Settings section
  const isSettingsSection = activeSection.startsWith("settings-");
  const settingsTab = isSettingsSection
    ? activeSection.replace("settings-", "")
    : "models";

  return (
    <>
      <TahoeLayout
        folders={folders}
        activeFolderId={activeFolder?.id}
        workspacePath={activeFolder?.path}
        onFolderSelect={handleFolderSelect}
        onAddFolder={addFolder}
        onNavigate={handleNavigate}
        onSettingsClick={handleSettingsClick}
        activeSection={activeSection}
        taskCounts={taskCounts}
      >
        <div className="space-y-6">
          {/* Error Display */}
          {(submitError || taskError) && (
            <div className="p-4 border border-red-200 dark:border-red-800 bg-red-50 dark:bg-red-950/30 animate-appear rounded-xl">
              <div className="flex items-center gap-2 text-red-600 dark:text-red-400">
                <AlertCircle className="size-4 shrink-0" />
                <p className="text-sm">{submitError || taskError}</p>
                <Button
                  variant="secondary"
                  size="sm"
                  className="ml-auto"
                  onPress={() => {
                    setSubmitError(null);
                  }}
                >
                  Dismiss
                </Button>
              </div>
            </div>
          )}

          {/* AI Studio Sections - Require active folder */}
          {activeSection === "documents" && (
            <div className="animate-appear">
              {activeFolder ? (
                <AIDocumentPanel />
              ) : (
                <NoFolderGate onAddFolder={addFolder} />
              )}
            </div>
          )}

          {activeSection === "research" && (
            <div className="animate-appear">
              {activeFolder ? (
                <AIResearchPanel />
              ) : (
                <NoFolderGate onAddFolder={addFolder} />
              )}
            </div>
          )}

          {activeSection === "cowork" && (
            <div className="animate-appear h-[calc(100vh-120px)]">
              {activeFolder ? (
                <CoworkPanel workspacePath={activeFolder.path} />
              ) : (
                <NoFolderGate onAddFolder={addFolder} />
              )}
            </div>
          )}

          {/* Settings Section */}
          {isSettingsSection && (
            <div className="animate-appear h-full">
              <SettingsPage
                initialTab={settingsTab}
                onBack={() => handleNavigate("running")}
              />
            </div>
          )}

          {/* Default View - CoworkPanel when no special section selected */}
          {!isAIStudioSection && !isSettingsSection && (
            <>
              {/* Folder Gate - Show prompt if no folder is selected */}
              {!activeFolder ? (
                <NoFolderGate onAddFolder={addFolder} />
              ) : (
                <>
                  {/* Task Queue Sections - Show when Running/Queued/Completed selected */}
                  {(activeSection === "running" ||
                    activeSection === "queued" ||
                    activeSection === "completed") &&
                  displayTasks.length > 0 ? (
                    <section className="space-y-4 animate-appear">
                      <div className="flex items-center gap-2 px-1">
                        {sectionTitle.icon}
                        <h2 className="text-base font-semibold">
                          {sectionTitle.label}
                        </h2>
                        <span className="text-sm text-muted-foreground">
                          ({displayTasks.length})
                        </span>
                      </div>
                      <div className="space-y-3">
                        {displayTasks.map((task) => (
                          <TaskCard
                            key={task.id}
                            task={convertTask(task)}
                            onPause={handleTaskPause}
                            onStop={handleTaskStop}
                          />
                        ))}
                      </div>
                    </section>
                  ) : (
                    /* CoworkPanel - Central Default View */
                    <div className="animate-appear h-[calc(100vh-120px)]">
                      <CoworkPanel workspacePath={activeFolder.path} />
                    </div>
                  )}
                </>
              )}
            </>
          )}
        </div>
      </TahoeLayout>

      {/* Settings Modal */}
      <SettingsPanel
        isOpen={settingsOpen}
        onClose={() => setSettingsOpen(false)}
      />
    </>
  );
}

/**
 * Gate component shown when no folder is selected.
 * Prompts user to select a project folder before using the system.
 */
function NoFolderGate({ onAddFolder }: { onAddFolder: () => void }) {
  return (
    <Card className="p-8 text-center animate-appear">
      <div className="space-y-4">
        <div className="size-16 mx-auto bg-primary/10 rounded-2xl flex items-center justify-center">
          <FolderPlus className="size-8 text-primary" />
        </div>
        <div className="space-y-2">
          <h2 className="text-lg font-semibold text-foreground">
            Select a Project Folder
          </h2>
          <p className="text-sm text-muted-foreground max-w-sm mx-auto">
            To get started, select a folder where Rainy Cowork will work. All
            files, documents, and AI-generated content will be saved there.
          </p>
        </div>
        <Button
          variant="primary"
          size="md"
          onPress={onAddFolder}
          className="mt-2"
        >
          <FolderPlus className="size-4" />
          Add Folder
        </Button>
      </div>
    </Card>
  );
}

export default App;
