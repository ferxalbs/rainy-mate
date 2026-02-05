// Rainy Cowork - useTauriTask Hook
// React hook for task management with Tauri backend

import { useCallback, useState } from "react";
import * as tauri from "../services/tauri";
import type { Task, TaskEvent, ProviderType } from "../services/tauri";

interface UseTaskResult {
  tasks: Task[];
  isLoading: boolean;
  error: string | null;
  createTask: (
    description: string,
    provider: ProviderType,
    model: string,
    workspacePath?: string,
  ) => Promise<Task>;
  executeTask: (taskId: string) => Promise<void>;
  pauseTask: (taskId: string) => Promise<void>;
  resumeTask: (taskId: string) => Promise<void>;
  cancelTask: (taskId: string) => Promise<void>;
  refreshTasks: () => Promise<void>;
}

export function useTauriTask(): UseTaskResult {
  const [tasks, setTasks] = useState<Task[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refreshTasks = useCallback(async () => {
    try {
      const taskList = await tauri.listTasks();
      setTasks(taskList);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, []);

  const createTask = useCallback(
    async (
      description: string,
      provider: ProviderType,
      model: string,
      workspacePath?: string,
    ): Promise<Task> => {
      setIsLoading(true);
      setError(null);
      try {
        const task = await tauri.createTask(
          description,
          provider,
          model,
          workspacePath,
        );
        setTasks((prev) => [task, ...prev]);
        return task;
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        setError(message);
        throw err;
      } finally {
        setIsLoading(false);
      }
    },
    [],
  );

  const executeTask = useCallback(async (taskId: string) => {
    setError(null);
    try {
      await tauri.executeTask(taskId, (event: TaskEvent) => {
        // Update task based on event
        setTasks((prev) =>
          prev.map((t) => {
            if (t.id !== taskId) return t;

            switch (event.event) {
              case "started":
                return { ...t, status: "running" as const };
              case "progress":
                return {
                  ...t,
                  progress: event.data.progress,
                  status: "running" as const,
                };
              case "completed":
                return {
                  ...t,
                  status: "completed" as const,
                  progress: 100,
                };
              case "failed":
                return {
                  ...t,
                  status: "failed" as const,
                  error: event.data.error,
                };
              default:
                return t;
            }
          }),
        );
      });
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      throw err;
    }
  }, []);

  const pauseTask = useCallback(async (taskId: string) => {
    try {
      await tauri.pauseTask(taskId);
      setTasks((prev) =>
        prev.map((t) =>
          t.id === taskId ? { ...t, status: "paused" as const } : t,
        ),
      );
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      throw err;
    }
  }, []);

  const resumeTask = useCallback(async (taskId: string) => {
    try {
      await tauri.resumeTask(taskId);
      setTasks((prev) =>
        prev.map((t) =>
          t.id === taskId ? { ...t, status: "running" as const } : t,
        ),
      );
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      throw err;
    }
  }, []);

  const cancelTask = useCallback(async (taskId: string) => {
    try {
      await tauri.cancelTask(taskId);
      setTasks((prev) =>
        prev.map((t) =>
          t.id === taskId ? { ...t, status: "cancelled" as const } : t,
        ),
      );
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      throw err;
    }
  }, []);

  return {
    tasks,
    isLoading,
    error,
    createTask,
    executeTask,
    pauseTask,
    resumeTask,
    cancelTask,
    refreshTasks,
  };
}
