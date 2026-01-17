import { useState, useCallback } from "react";
import { MainLayout, TaskInput, TaskCard, FileTable } from "./components";
import { Card, Separator } from "@heroui/react";
import { Zap, CheckCircle2, ListTodo } from "lucide-react";
import type { Task, ProviderType, Folder, FileChange } from "./types";

// Generate unique IDs
const generateId = () => Math.random().toString(36).substring(2, 11);

function App() {
  const [tasks, setTasks] = useState<Task[]>([]);
  const [activeSection, setActiveSection] = useState("running");
  const [fileChanges, setFileChanges] = useState<FileChange[]>([]);

  // Calculate task counts for sidebar
  const taskCounts = {
    completed: tasks.filter((t) => t.status === "completed").length,
    running: tasks.filter((t) => t.status === "running" || t.status === "paused").length,
    queued: tasks.filter((t) => t.status === "queued").length,
  };

  // Handle new task submission
  const handleTaskSubmit = useCallback((description: string, provider: ProviderType) => {
    const newTask: Task = {
      id: generateId(),
      title: description.length > 50 ? description.substring(0, 47) + "..." : description,
      description,
      status: "running",
      progress: 0,
      provider,
      model: provider === "openai" ? "gpt-4o" : provider === "anthropic" ? "claude-3.5-sonnet" : "llama3.2",
      createdAt: new Date(),
      startedAt: new Date(),
    };

    setTasks((prev) => [newTask, ...prev]);
    simulateTaskProgress(newTask.id);
  }, []);

  // Simulate task progress (demo functionality)
  const simulateTaskProgress = (taskId: string) => {
    let progress = 0;
    const interval = setInterval(() => {
      progress += Math.random() * 15;
      if (progress >= 100) {
        progress = 100;
        setTasks((prev) =>
          prev.map((t) =>
            t.id === taskId
              ? { ...t, progress: 100, status: "completed", completedAt: new Date() }
              : t
          )
        );

        // Add a demo file change
        setFileChanges((prev) => [
          {
            id: generateId(),
            path: "~/Documents/task-output.md",
            filename: "task-output.md",
            operation: "create",
            timestamp: new Date(),
            taskId,
          },
          ...prev,
        ]);

        clearInterval(interval);
      } else {
        setTasks((prev) =>
          prev.map((t) => (t.id === taskId ? { ...t, progress: Math.round(progress) } : t))
        );
      }
    }, 500);
  };

  // Handle task pause
  const handleTaskPause = useCallback((taskId: string) => {
    setTasks((prev) =>
      prev.map((t) =>
        t.id === taskId
          ? { ...t, status: t.status === "paused" ? "running" : "paused" }
          : t
      )
    );
  }, []);

  // Handle task stop
  const handleTaskStop = useCallback((taskId: string) => {
    setTasks((prev) =>
      prev.map((t) =>
        t.id === taskId ? { ...t, status: "cancelled", completedAt: new Date() } : t
      )
    );
  }, []);

  // Handle folder selection
  const handleFolderSelect = useCallback((folder: Folder) => {
    console.log("Selected folder:", folder);
    // TODO: Implement folder navigation
  }, []);

  // Handle navigation
  const handleNavigate = useCallback((section: string) => {
    setActiveSection(section);
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

  const displayTasks = getDisplayTasks();
  const sectionTitle = {
    running: { icon: <Zap className="size-5 text-blue-500 shrink-0" />, label: "Active Tasks" },
    completed: { icon: <CheckCircle2 className="size-5 text-green-500 shrink-0" />, label: "Completed Tasks" },
    queued: { icon: <ListTodo className="size-5 text-orange-500 shrink-0" />, label: "Queued Tasks" },
  }[activeSection] || { icon: <Zap className="size-5 text-blue-500 shrink-0" />, label: "Tasks" };

  return (
    <MainLayout
      onFolderSelect={handleFolderSelect}
      onNavigate={handleNavigate}
      activeSection={activeSection}
      taskCounts={taskCounts}
    >
      <div className="space-y-6 sm:space-y-8">
        {/* Task Input Section */}
        <Card className="p-4 sm:p-6">
          <TaskInput onSubmit={handleTaskSubmit} />
        </Card>

        {/* Tasks Section */}
        {displayTasks.length > 0 && (
          <section className="space-y-4">
            <div className="flex items-center gap-2">
              {sectionTitle.icon}
              <h2 className="text-lg font-semibold">{sectionTitle.label}</h2>
              <span className="text-sm text-muted-foreground">({displayTasks.length})</span>
            </div>
            <div className="space-y-3">
              {displayTasks.map((task) => (
                <TaskCard
                  key={task.id}
                  task={task}
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
          <Card variant="transparent" className="p-6 sm:p-8 text-center">
            <div className="space-y-3">
              <div className="size-16 mx-auto bg-muted rounded-full flex items-center justify-center">
                <Zap className="size-8 text-muted-foreground" />
              </div>
              <p className="text-lg font-medium text-muted-foreground">
                No tasks yet
              </p>
              <p className="text-sm text-muted-foreground max-w-sm mx-auto">
                Type a task above to get started with your AI assistant. Press ⌘+Enter to submit quickly.
              </p>
            </div>
          </Card>
        )}

        {/* Empty section state */}
        {tasks.length > 0 && displayTasks.length === 0 && (
          <Card variant="transparent" className="p-6 text-center">
            <p className="text-sm text-muted-foreground">
              No {activeSection} tasks
            </p>
          </Card>
        )}
      </div>
    </MainLayout>
  );
}

export default App;
