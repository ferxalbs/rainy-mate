import { useCallback, useEffect, useMemo, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { Clock3, Pencil, RefreshCw, ShieldAlert, Trash2, X, Settings2, CalendarDays } from "lucide-react";
import { Button, Select, ListBox } from "@heroui/react";

import * as tauri from "../../services/tauri";
import { ScheduleBuilder } from "../scheduling/ScheduleBuilder";
import {
  DEFAULT_SCHEDULE_DRAFT,
  describeSchedule,
  inferScheduleDraft,
  normalizeScheduleDraft,
} from "../../lib/schedule-builder";
import type { ScheduleDraft } from "../../lib/schedule-builder";

interface WorkspaceRecurringRunsProps {
  workspacePath: string;
}

const selectionToValue = (selection: unknown): string | null => {
  if (typeof selection === "string") return selection;
  if (selection instanceof Set) {
    const first = selection.values().next().value;
    return typeof first === "string" ? first : null;
  }
  return null;
};

function formatTimestamp(value?: number | null): string {
  if (!value) return "Not yet";
  return new Date(value * 1000).toLocaleString();
}

function statusTone(status?: string | null): string {
  switch (status) {
    case "completed":
      return "text-green-500 bg-green-500/10";
    case "running":
      return "text-blue-500 bg-blue-500/10";
    case "failed":
      return "text-red-500 bg-red-500/10";
    default:
      return "text-muted-foreground bg-white/5";
  }
}

export function WorkspaceRecurringRuns({
  workspacePath,
}: WorkspaceRecurringRunsProps) {
  const [jobs, setJobs] = useState<tauri.WorkspaceScheduledRun[]>([]);
  const [scenarios, setScenarios] = useState<tauri.FirstRunScenarioDefinition[]>([]);
  const [scenarioId, setScenarioId] = useState("release_readiness");
  const [scheduleDraft, setScheduleDraft] = useState<ScheduleDraft>(
    DEFAULT_SCHEDULE_DRAFT,
  );
  const [editingJobId, setEditingJobId] = useState<string | null>(null);
  const [editingMode, setEditingMode] = useState<"playbook" | "prompt">("playbook");
  const [editingScenarioId, setEditingScenarioId] = useState("release_readiness");
  const [editingTitle, setEditingTitle] = useState("");
  const [editingPrompt, setEditingPrompt] = useState("");
  const [editingScheduleDraft, setEditingScheduleDraft] = useState<ScheduleDraft>(
    DEFAULT_SCHEDULE_DRAFT,
  );
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const [nextJobs, nextScenarios] = await Promise.all([
        tauri.listWorkspaceScheduledRuns(workspacePath),
        tauri.listFirstRunScenarios(),
      ]);
      setJobs(nextJobs);
      setScenarios(nextScenarios);
      if (!nextScenarios.some((entry) => entry.id === scenarioId)) {
        setScenarioId(nextScenarios[0]?.id ?? "release_readiness");
      }
    } catch (nextError) {
      setError(
        nextError instanceof Error
          ? nextError.message
          : "Failed to load recurring runs",
      );
    } finally {
      setIsLoading(false);
    }
  }, [scenarioId, workspacePath]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | null = null;

    void listen<{ workspacePath?: string }>(
      "workspace://scheduled-runs-updated",
      (event) => {
        if (cancelled) return;
        if (
          event.payload?.workspacePath &&
          event.payload.workspacePath !== workspacePath
        ) {
          return;
        }
        void refresh();
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
  }, [refresh, workspacePath]);

  const scenarioMap = useMemo(
    () => new Map(scenarios.map((entry) => [entry.id, entry])),
    [scenarios],
  );

  const handleCreate = useCallback(async () => {
    setIsSaving(true);
    setError(null);
    try {
      await tauri.createWorkspaceScheduledRun(
        workspacePath,
        scenarioId,
        normalizeScheduleDraft(scheduleDraft).cronExpression,
      );
      setScheduleDraft(DEFAULT_SCHEDULE_DRAFT);
      await refresh();
    } catch (nextError) {
      setError(
        nextError instanceof Error
          ? nextError.message
          : "Failed to create recurring run",
      );
    } finally {
      setIsSaving(false);
    }
  }, [refresh, scenarioId, scheduleDraft, workspacePath]);

  const handleDelete = useCallback(
    async (scheduledRunId: string) => {
      setIsSaving(true);
      setError(null);
      try {
        await tauri.deleteWorkspaceScheduledRun(workspacePath, scheduledRunId);
        await refresh();
      } catch (nextError) {
        setError(
          nextError instanceof Error
            ? nextError.message
            : "Failed to delete recurring run",
        );
      } finally {
        setIsSaving(false);
      }
    },
    [refresh, workspacePath],
  );

  const handleStartEdit = useCallback((job: tauri.WorkspaceScheduledRun) => {
    setEditingJobId(job.id);
    setEditingMode(job.jobKind === "prompt" ? "prompt" : "playbook");
    setEditingScenarioId(job.scenarioId || "release_readiness");
    setEditingTitle(job.title);
    setEditingPrompt(job.promptText ?? "");
    setEditingScheduleDraft(inferScheduleDraft(job.schedule));
    setError(null);
  }, []);

  const handleCancelEdit = useCallback(() => {
    setEditingJobId(null);
    setEditingMode("playbook");
    setEditingScenarioId("release_readiness");
    setEditingTitle("");
    setEditingPrompt("");
    setEditingScheduleDraft(DEFAULT_SCHEDULE_DRAFT);
    setError(null);
  }, []);

  const handleUpdate = useCallback(async () => {
    if (!editingJobId) return;
    setIsSaving(true);
    setError(null);
    try {
      await tauri.updateWorkspaceScheduledRun(
        workspacePath,
        editingJobId,
        normalizeScheduleDraft(editingScheduleDraft).cronExpression,
        editingMode === "prompt"
          ? {
              title: editingTitle.trim(),
              prompt: editingPrompt.trim(),
            }
          : {
              scenarioId: editingScenarioId,
            },
      );
      handleCancelEdit();
      await refresh();
    } catch (nextError) {
      setError(
        nextError instanceof Error
          ? nextError.message
          : "Failed to update recurring run",
      );
    } finally {
      setIsSaving(false);
    }
  }, [
    editingJobId,
    editingMode,
    editingPrompt,
    editingScenarioId,
    editingScheduleDraft,
    editingTitle,
    handleCancelEdit,
    refresh,
    workspacePath,
  ]);

  if (isLoading && jobs.length === 0) {
    return (
      <div className="flex h-full w-full items-center justify-center">
        <div className="animate-pulse text-sm font-medium text-muted-foreground tracking-wide">
          Loading recurring runs...
        </div>
      </div>
    );
  }

  return (
    <div className="relative h-full w-full overflow-y-auto overflow-x-hidden p-4 md:p-6 lg:p-8">
      <div className="flex flex-col gap-6 max-w-6xl mx-auto pb-12">
        {/* Header Section */}
        <div className="flex flex-col gap-4 md:flex-row md:items-start md:justify-between">
          <div className="space-y-2">
            <div className="flex items-center gap-2.5">
              <Clock3 className="size-5 text-primary" />
              <h1 className="text-xl font-semibold tracking-tight text-foreground">
                Recurring Runs
              </h1>
            </div>
            <p className="max-w-2xl text-[13px] leading-relaxed text-muted-foreground">
              Schedule first-party MaTE playbooks against this workspace.
            </p>
          </div>
          <Button
            variant="secondary"
            className="font-medium shrink-0 h-9 rounded-xl bg-white/5 hover:bg-white/10 text-foreground border border-white/5"
            onPress={() => void refresh()}
            isDisabled={isLoading || isSaving}
          >
            <RefreshCw className="size-4 mr-2" />
            Refresh
          </Button>
        </div>

        {error && (
          <div className="rounded-2xl border border-red-500/20 bg-red-500/5 p-4">
            <p className="text-sm font-medium text-red-500">{error}</p>
          </div>
        )}

        <div className="flex flex-col gap-4">
          <div className="flex items-center gap-2 px-1">
            <Settings2 className="size-4 text-primary" />
            <h2 className="text-[14px] font-semibold tracking-tight text-foreground">New Run Configuration</h2>
          </div>
          
          <div className="rounded-2xl border border-white/5 bg-white/5 p-4 flex flex-col gap-4">
            <label className="flex flex-col gap-1.5">
              <span className="text-[10px] font-semibold tracking-widest text-muted-foreground/50 uppercase">
                Scenario
              </span>
              <Select
                className="w-full"
                selectedKey={scenarioId}
                isDisabled={isSaving || isLoading}
                onSelectionChange={(selection) => {
                  const value = selectionToValue(selection);
                  if (value) setScenarioId(value);
                }}
              >
                <Select.Trigger className="h-9 rounded-xl border border-white/10 bg-white/5 px-2.5 text-[12px] text-foreground shadow-sm">
                  <Select.Value />
                  <Select.Indicator />
                </Select.Trigger>
                <Select.Popover className="bg-background/95 dark:bg-background/35 border border-default-200/70 dark:border-white/15 backdrop-blur-xl max-h-[300px] overflow-y-auto">
                  <ListBox className="bg-transparent">
                    {scenarios.map((scenario) => (
                      <ListBox.Item key={scenario.id} id={scenario.id} textValue={scenario.title}>
                        {scenario.title}
                      </ListBox.Item>
                    ))}
                  </ListBox>
                </Select.Popover>
              </Select>
            </label>

            <ScheduleBuilder
              value={scheduleDraft}
              onChange={setScheduleDraft}
              disabled={isSaving || isLoading}
            />

            <div className="flex items-end justify-end mt-2">
              <Button
                variant="primary"
                className="h-10 px-6 rounded-xl font-medium text-[13px]"
                onPress={() => void handleCreate()}
                isDisabled={isSaving}
              >
                {isSaving ? "Saving..." : "Create Run"}
              </Button>
            </div>
          </div>
        </div>

        {editingJobId ? (
            <div className="flex flex-col gap-4">
            <div className="flex items-center gap-2 px-1">
              <Pencil className="size-4 text-primary" />
              <h2 className="text-[14px] font-semibold tracking-tight text-foreground">Edit Configuration</h2>
            </div>
          
            <div className="rounded-2xl border border-primary/20 bg-primary/5 p-4 flex flex-col gap-4">
              <div className="flex items-center justify-between gap-3 mb-1">
                <p className="text-[12px] text-primary/80">
                  Update the stored schedule and task payload for this workspace run.
                </p>
                <Button
                  isIconOnly
                  variant="ghost"
                  size="sm"
                  className="text-muted-foreground hover:text-foreground border-none h-8 w-8 min-w-8"
                  onPress={handleCancelEdit}
                  isDisabled={isSaving}
                >
                  <X className="size-3.5" />
                </Button>
              </div>

              {editingMode === "playbook" ? (
                <label className="flex flex-col gap-1.5">
                  <span className="text-[10px] font-semibold tracking-widest text-muted-foreground/50 uppercase">
                    Scenario
                  </span>
                  <Select
                    className="w-full"
                    selectedKey={editingScenarioId}
                    isDisabled={isSaving || isLoading}
                    onSelectionChange={(selection) => {
                      const value = selectionToValue(selection);
                      if (value) setEditingScenarioId(value);
                    }}
                  >
                    <Select.Trigger className="h-9 rounded-xl border border-white/10 bg-white/5 px-2.5 text-[12px] text-foreground shadow-sm">
                      <Select.Value />
                      <Select.Indicator />
                    </Select.Trigger>
                    <Select.Popover className="bg-background/95 dark:bg-background/35 border border-default-200/70 dark:border-white/15 backdrop-blur-xl max-h-[300px] overflow-y-auto">
                      <ListBox className="bg-transparent">
                        {scenarios.map((scenario) => (
                          <ListBox.Item key={scenario.id} id={scenario.id} textValue={scenario.title}>
                            {scenario.title}
                          </ListBox.Item>
                        ))}
                      </ListBox>
                    </Select.Popover>
                  </Select>
                </label>
              ) : (
                <div className="grid gap-4 md:grid-cols-2">
                  <label className="flex flex-col gap-1.5 md:col-span-2">
                    <span className="text-[10px] font-semibold tracking-widest text-muted-foreground/50 uppercase">
                      Task Title
                    </span>
                    <input
                      className="h-9 rounded-xl border border-white/10 bg-white/5 px-2.5 text-[12px] text-foreground outline-none focus:border-primary/50 transition-colors"
                      value={editingTitle}
                      onChange={(event) => setEditingTitle(event.target.value)}
                      disabled={isSaving}
                    />
                  </label>

                  <label className="flex flex-col gap-1.5 md:col-span-2">
                    <span className="text-[10px] font-semibold tracking-widest text-muted-foreground/50 uppercase">
                      Prompt
                    </span>
                    <textarea
                      className="min-h-[80px] rounded-xl border border-white/10 bg-white/5 px-2.5 py-2 text-[12px] text-foreground outline-none focus:border-primary/50 transition-colors resize-y"
                      value={editingPrompt}
                      onChange={(event) => setEditingPrompt(event.target.value)}
                      disabled={isSaving}
                    />
                  </label>
                </div>
              )}

              <ScheduleBuilder
                value={editingScheduleDraft}
                onChange={setEditingScheduleDraft}
                disabled={isSaving || isLoading}
              />

              <div className="flex items-end justify-end gap-2.5 mt-1">
                <Button
                  variant="outline"
                  className="h-9 px-4 rounded-xl border-white/10 text-[12px] font-medium"
                  onPress={handleCancelEdit}
                  isDisabled={isSaving}
                >
                  Cancel
                </Button>
                <Button
                  variant="primary"
                  className="h-9 px-4 rounded-xl text-[12px] font-medium"
                  onPress={() => void handleUpdate()}
                  isDisabled={
                    isSaving ||
                    (editingMode === "prompt" &&
                      (!editingTitle.trim() || !editingPrompt.trim()))
                  }
                >
                  {isSaving ? "Saving..." : "Save Changes"}
                </Button>
              </div>
            </div>
          </div>
        ) : null}

        <div className="flex flex-col gap-4 pt-2">
          <div className="flex items-center gap-2 px-1">
            <CalendarDays className="size-4 text-primary" />
            <h2 className="text-[14px] font-semibold tracking-tight text-foreground">Scheduled Playbooks</h2>
          </div>

          {jobs.length === 0 ? (
            <div className="rounded-2xl border border-dashed border-white/10 bg-transparent p-5 text-center">
              <p className="text-[12px] text-muted-foreground">No recurring playbooks configured for this workspace yet.</p>
            </div>
          ) : (
             <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
              {jobs.map((job) => {
                const scenario = scenarioMap.get(job.scenarioId);
                return (
                  <div
                    key={job.id}
                    className="flex flex-col gap-4 rounded-2xl border border-white/5 bg-white/5 p-4"
                  >
                    <div className="space-y-2">
                      <div className="flex items-center flex-wrap gap-2">
                        <p className="text-[15px] font-semibold text-foreground tracking-tight">
                          {job.title || scenario?.title || job.scenarioId}
                        </p>
                        <span className="rounded-lg bg-white/5 border border-white/5 px-2 py-0.5 text-[10px] uppercase tracking-wider text-muted-foreground">
                          {job.jobKind === "prompt" ? "Prompt" : "Playbook"}
                        </span>
                        <span
                          className={`rounded-lg px-2 py-0.5 text-[10px] uppercase tracking-wider font-medium ${statusTone(job.lastStatus)}`}
                        >
                          {job.lastStatus ?? "scheduled"}
                        </span>
                      </div>
                      
                      <p className="text-[12px] leading-relaxed text-muted-foreground line-clamp-2">
                        {job.jobKind === "prompt"
                          ? job.promptText || "Recurring chat task"
                          : scenario?.summary ?? "First-party recurring playbook"}
                      </p>
                      
                      <div className="flex flex-wrap items-center gap-1.5 pt-1">
                        <div className="rounded-lg border border-white/5 bg-white/5 px-2 py-0.5 text-[10px] font-medium text-foreground/80">
                          {describeSchedule(inferScheduleDraft(job.schedule))}
                        </div>
                        <div className="rounded-lg border border-white/5 bg-white/5 px-2 py-0.5 text-[10px] font-mono text-muted-foreground">
                          {job.schedule}
                        </div>
                        <div className="rounded-lg bg-primary/10 px-2 py-0.5 text-[10px] font-medium text-primary">
                          {job.enabledPackIds.length} packs
                        </div>
                        <div className="rounded-lg border border-white/5 bg-white/5 px-2 py-0.5 text-[10px] font-medium text-foreground/80">
                          {job.lastArtifactCount || 0} artifacts
                        </div>
                      </div>
                    </div>

                    <div className="grid grid-cols-2 gap-4 mt-1 border-t border-white/5 pt-3">
                      <div className="space-y-1">
                        <p className="text-[9px] font-semibold tracking-widest text-muted-foreground/50 uppercase">Timing</p>
                        <div className="text-[11px]">
                          <span className="text-muted-foreground">Next:</span> <span className="text-foreground/90 font-medium ml-1">{formatTimestamp(job.nextRunAt)}</span>
                        </div>
                        <div className="text-[11px]">
                          <span className="text-muted-foreground">Last:</span> <span className="text-foreground/90 font-medium ml-1">{formatTimestamp(job.lastRunAt)}</span>
                        </div>
                      </div>

                      <div className="space-y-1">
                        <p className="text-[9px] font-semibold tracking-widest text-muted-foreground/50 uppercase">Posture</p>
                        <p className="text-[11px] text-foreground/90 font-medium line-clamp-1">
                          {job.lastRequiresExplicitApproval
                            ? "Explicit approval required"
                            : "No explicit approval needed"}
                        </p>
                        
                        {job.lastBlockedByApproval ? (
                          <div className="flex items-center gap-1.5 text-amber-500 mt-0.5">
                            <ShieldAlert className="size-3" />
                            <span className="text-[11px]">Blocked by Airlock</span>
                          </div>
                        ) : null}
                        
                        {job.lastError ? (
                          <p className="text-[11px] text-red-500 line-clamp-1 mt-0.5" title={job.lastError}>
                            {job.lastError}
                          </p>
                        ) : null}
                        
                        {job.lastChatId ? (
                          <p className="text-[10px] text-muted-foreground truncate max-w-full mt-0.5">
                            Chat ID: {job.lastChatId}
                          </p>
                        ) : null}
                      </div>
                    </div>

                    <div className="flex items-center justify-end gap-2 pt-2 mt-auto">
                        <Button
                          variant="secondary"
                          size="sm"
                          className="h-8 rounded-xl bg-white/5 hover:bg-white/10 text-foreground font-medium"
                          onPress={() => handleStartEdit(job)}
                          isDisabled={isSaving}
                        >
                          <Pencil className="size-3.5 mr-1" />
                          Edit
                        </Button>
                        <Button
                          variant="danger"
                          size="sm"
                          className="h-8 rounded-xl font-medium"
                          onPress={() => void handleDelete(job.id)}
                          isDisabled={isSaving}
                        >
                          <Trash2 className="size-3.5 mr-1" />
                          Delete
                        </Button>
                      </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

