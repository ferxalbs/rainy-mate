import { useState, useCallback, useEffect } from "react";
import { TahoeLayout, TaskInput, TaskCard, FileTable, SettingsPanel } from "./components";
import { Separator, Button } from "@heroui/react";
import { Zap, CheckCircle2, ListTodo, Settings, AlertCircle } from "lucide-react";
import { useTauriTask, useAIProvider } from "./hooks";
import type { Task, ProviderType, Folder, FileChange } from "./types";
import * as tauri from "./services/tauri";

function App() {
  // Tauri hooks
  const {
    tasks,
    isLoading,
    error: taskError,
    createTask,
    executeTask,
    pauseTask,
    resumeTask,
    cancelTask,
    refreshTasks
  } = useTauriTask();

  const { hasApiKey, refreshProviders } = useAIProvider();

  const [activeSection, setActiveSection] = useState("running");
  const [fileChanges, setFileChanges] = useState<FileChange[]>([]);
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
    running: tasks.filter((t) => t.status === "running" || t.status === "paused").length,
    queued: tasks.filter((t) => t.status === "queued").length,
  };

  // Handle new task submission
  const handleTaskSubmit = useCallback(async (
    description: string,
    provider: ProviderType,
    model: string
  ) => {
    setSubmitError(null);

    // Check if API key is configured
    const providerId = provider === 'rainyApi' ? 'rainy_api' : 'gemini';
    if (!hasApiKey(providerId)) {
      setSubmitError(`Please configure your ${provider === 'rainyApi' ? 'Rainy API' : 'Gemini'} API key in Settings first.`);
      setSettingsOpen(true);
      return;
    }

    try {
      // Create task via Tauri
      const task = await createTask(description, provider, model);

      // Execute the task
      await executeTask(task.id);

      // Refresh file changes after task completes
      const changes = await tauri.listFileChanges(task.id);
      if (changes.length > 0) {
        setFileChanges(prev => [...changes.map(c => ({
          ...c,
          timestamp: new Date(c.timestamp),
        })), ...prev]);
      }
    } catch (err) {
      setSubmitError(err instanceof Error ? err.message : String(err));
    }
  }, [createTask, executeTask, hasApiKey]);

  // Handle task pause
  const handleTaskPause = useCallback(async (taskId: string) => {
    const task = tasks.find(t => t.id === taskId);
    if (!task) return;

    try {
      if (task.status === 'paused') {
        await resumeTask(taskId);
      } else {
        await pauseTask(taskId);
      }
    } catch (err) {
      console.error('Failed to pause/resume task:', err);
    }
  }, [tasks, pauseTask, resumeTask]);

  // Handle task stop
  const handleTaskStop = useCallback(async (taskId: string) => {
    try {
      await cancelTask(taskId);
    } catch (err) {
      console.error('Failed to cancel task:', err);
    }
  }, [cancelTask]);

  // Handle folder selection
  const handleFolderSelect = useCallback(async (folder: Folder) => {
    try {
      await tauri.setWorkspace(folder.path, folder.name);
      console.log("Workspace set:", folder);
    } catch (err) {
      console.error('Failed to set workspace:', err);
    }
  }, []);

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
        return tasks.filter((t) => t.status === "running" || t.status === "paused");
      case "queued":
        return tasks.filter((t) => t.status === "queued");
      default:
        return tasks.filter((t) => t.status === "running" || t.status === "paused");
    }
  };

  // Convert Tauri tasks to local Task type for TaskCard
  const convertTask = (t: typeof tasks[0]): Task => ({
    ...t,
    createdAt: new Date(t.createdAt),
    startedAt: t.startedAt ? new Date(t.startedAt) : undefined,
    completedAt: t.completedAt ? new Date(t.completedAt) : undefined,
  });

  const displayTasks = getDisplayTasks();
  const sectionTitle = {
    running: { icon: <Zap className="size-5 text-blue-500 shrink-0" />, label: "Active Tasks" },
    completed: { icon: <CheckCircle2 className="size-5 text-green-500 shrink-0" />, label: "Completed Tasks" },
    queued: { icon: <ListTodo className="size-5 text-orange-500 shrink-0" />, label: "Queued Tasks" },
  }[activeSection] || { icon: <Zap className="size-5 text-blue-500 shrink-0" />, label: "Tasks" };

  return (
    <>
      <TahoeLayout
        onFolderSelect={handleFolderSelect}
        onNavigate={handleNavigate}
        onSettingsClick={handleSettingsClick}
        activeSection={activeSection}
        taskCounts={taskCounts}
      >
        <div className="space-y-6">
          {/* Error Display */}
          {(submitError || taskError) && (
            <div className="floating-card p-4 border-red-200 dark:border-red-800 bg-red-50 dark:bg-red-950/30 animate-appear">
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

          {/* Task Input Section */}
          <div className="floating-card p-5 animate-appear">
            <TaskInput onSubmit={handleTaskSubmit} isLoading={isLoading} />
          </div>

          {/* Tasks Section */}
          {displayTasks.length > 0 && (
            <section className="space-y-4">
              <div className="flex items-center gap-2 px-1">
                {sectionTitle.icon}
                <h2 className="text-base font-semibold">{sectionTitle.label}</h2>
                <span className="text-sm text-muted-foreground">({displayTasks.length})</span>
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
          )}

          {/* Divider */}
          {displayTasks.length > 0 && fileChanges.length > 0 && <Separator />}

          {/* File Changes Section */}
          {fileChanges.length > 0 && <FileTable changes={fileChanges} />}

          {/* Empty State */}
          {tasks.length === 0 && (
            <div className="floating-card p-8 text-center animate-appear">
              <div className="space-y-3">
                <div className="size-14 mx-auto bg-muted/50 rounded-2xl flex items-center justify-center">
                  <Zap className="size-7 text-muted-foreground" />
                </div>
                <p className="text-base font-medium text-foreground">
                  No tasks yet
                </p>
                <p className="text-sm text-muted-foreground max-w-xs mx-auto">
                  Type a task above to get started with your AI assistant. Press ⌘+Enter to submit.
                </p>
                <Button
                  variant="secondary"
                  size="sm"
                  onPress={() => setSettingsOpen(true)}
                  className="mt-2"
                >
                  <Settings className="size-4" />
                  Configure API Keys
                </Button>
              </div>
            </div>
          )}

          {/* Empty section state */}
          {tasks.length > 0 && displayTasks.length === 0 && (
            <div className="floating-card p-6 text-center animate-appear">
              <p className="text-sm text-muted-foreground">
                No {activeSection} tasks
              </p>
            </div>
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

export default App;
