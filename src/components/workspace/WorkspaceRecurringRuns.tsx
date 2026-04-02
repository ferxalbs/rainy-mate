import { useCallback, useEffect, useMemo, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { Clock3, Pencil, RefreshCw, ShieldAlert, Trash2, X } from "lucide-react";

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

function formatTimestamp(value?: number | null): string {
  if (!value) return "Not yet";
  return new Date(value * 1000).toLocaleString();
}

function statusTone(status?: string | null): string {
  switch (status) {
    case "completed":
      return "text-green-500";
    case "running":
      return "text-blue-500";
    case "failed":
      return "text-red-500";
    default:
      return "text-muted-foreground";
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

  return (
    <div className="flex h-full w-full flex-col overflow-y-auto p-6">
      <div className="mx-auto flex w-full max-w-6xl flex-col gap-6">
        <div className="flex flex-col gap-3 md:flex-row md:items-end md:justify-between">
          <div className="space-y-2">
            <div className="flex items-center gap-3">
              <Clock3 className="size-5 text-primary" />
              <h1 className="text-2xl font-semibold tracking-tight text-foreground">
                Recurring Runs
              </h1>
            </div>
            <p className="max-w-3xl text-sm leading-relaxed text-muted-foreground">
              Schedule first-party MaTE playbooks against this workspace without
              turning the runtime into an unbounded daemon. Each run re-enters
              the normal governed contract flow.
            </p>
          </div>
          <button
            className="inline-flex h-10 items-center gap-2 rounded-2xl border border-white/10 bg-background px-4 text-sm font-medium text-foreground transition hover:bg-white/5 disabled:cursor-not-allowed disabled:opacity-60"
            onClick={() => void refresh()}
            disabled={isLoading || isSaving}
          >
            <RefreshCw className="size-4" />
            Refresh
          </button>
        </div>

        {error && (
          <div className="rounded-2xl border border-red-500/20 bg-red-500/5 p-4 text-sm text-red-500">
            {error}
          </div>
        )}

        <div className="grid gap-5 rounded-3xl border border-white/10 bg-background/40 p-5">
          <label className="flex flex-col gap-2">
            <span className="text-xs font-semibold uppercase tracking-[0.16em] text-muted-foreground">
              Scenario
            </span>
            <select
              className="h-11 rounded-2xl border border-border/60 bg-background px-3 text-sm text-foreground outline-none"
              value={scenarioId}
              onChange={(event) => setScenarioId(event.target.value)}
              disabled={isSaving || isLoading}
            >
              {scenarios.map((scenario) => (
                <option key={scenario.id} value={scenario.id}>
                  {scenario.title}
                </option>
              ))}
            </select>
          </label>

          <ScheduleBuilder
            value={scheduleDraft}
            onChange={setScheduleDraft}
            disabled={isSaving || isLoading}
          />

          <div className="flex items-end justify-end">
            <button
              className="inline-flex h-11 items-center justify-center rounded-2xl bg-primary px-5 text-sm font-semibold text-primary-foreground transition hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-60"
              onClick={() => void handleCreate()}
              disabled={isSaving}
            >
              {isSaving ? "Saving..." : "Create Run"}
            </button>
          </div>
        </div>

        {editingJobId ? (
          <div className="grid gap-5 rounded-3xl border border-primary/20 bg-primary/5 p-5">
            <div className="flex items-center justify-between gap-3">
              <div>
                <p className="text-sm font-semibold uppercase tracking-[0.16em] text-muted-foreground">
                  Edit Recurring Run
                </p>
                <p className="text-sm text-muted-foreground">
                  Update the stored schedule and task payload for this workspace run.
                </p>
              </div>
              <button
                className="rounded-full p-2 text-muted-foreground transition hover:bg-white/5 hover:text-foreground"
                onClick={handleCancelEdit}
                disabled={isSaving}
              >
                <X className="size-4" />
              </button>
            </div>

            {editingMode === "playbook" ? (
              <label className="flex flex-col gap-2">
                <span className="text-xs font-semibold uppercase tracking-[0.16em] text-muted-foreground">
                  Scenario
                </span>
                <select
                  className="h-11 rounded-2xl border border-border/60 bg-background px-3 text-sm text-foreground outline-none"
                  value={editingScenarioId}
                  onChange={(event) => setEditingScenarioId(event.target.value)}
                  disabled={isSaving || isLoading}
                >
                  {scenarios.map((scenario) => (
                    <option key={scenario.id} value={scenario.id}>
                      {scenario.title}
                    </option>
                  ))}
                </select>
              </label>
            ) : (
              <div className="grid gap-4">
                <label className="flex flex-col gap-2">
                  <span className="text-xs font-semibold uppercase tracking-[0.16em] text-muted-foreground">
                    Task title
                  </span>
                  <input
                    className="h-11 rounded-2xl border border-border/60 bg-background px-3 text-sm text-foreground outline-none"
                    value={editingTitle}
                    onChange={(event) => setEditingTitle(event.target.value)}
                    disabled={isSaving}
                  />
                </label>

                <label className="flex flex-col gap-2">
                  <span className="text-xs font-semibold uppercase tracking-[0.16em] text-muted-foreground">
                    Prompt
                  </span>
                  <textarea
                    className="min-h-[120px] rounded-2xl border border-border/60 bg-background px-3 py-3 text-sm text-foreground outline-none"
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

            <div className="flex items-end justify-end gap-3">
              <button
                className="inline-flex h-11 items-center justify-center rounded-2xl border border-white/10 bg-background px-5 text-sm font-semibold text-foreground transition hover:bg-white/5 disabled:cursor-not-allowed disabled:opacity-60"
                onClick={handleCancelEdit}
                disabled={isSaving}
              >
                Cancel
              </button>
              <button
                className="inline-flex h-11 items-center justify-center rounded-2xl bg-primary px-5 text-sm font-semibold text-primary-foreground transition hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-60"
                onClick={() => void handleUpdate()}
                disabled={
                  isSaving ||
                  (editingMode === "prompt" &&
                    (!editingTitle.trim() || !editingPrompt.trim()))
                }
              >
                {isSaving ? "Saving..." : "Save Changes"}
              </button>
            </div>
          </div>
        ) : null}

        <div className="rounded-3xl border border-white/10 bg-background/35">
          <div className="border-b border-white/10 px-5 py-4">
            <h2 className="text-sm font-semibold uppercase tracking-[0.16em] text-muted-foreground">
              Scheduled Playbooks
            </h2>
          </div>

          {isLoading ? (
            <div className="px-5 py-8 text-sm text-muted-foreground">
              Loading recurring runs...
            </div>
          ) : jobs.length === 0 ? (
            <div className="px-5 py-8 text-sm text-muted-foreground">
              No recurring playbooks configured for this workspace yet.
            </div>
          ) : (
            <div className="divide-y divide-white/10">
              {jobs.map((job) => {
                const scenario = scenarioMap.get(job.scenarioId);
                return (
                  <div
                    key={job.id}
                    className="grid gap-4 px-5 py-5 md:grid-cols-[1.3fr_1fr_1fr_auto]"
                  >
                    <div className="space-y-2">
                      <div className="flex items-center gap-2">
                        <p className="text-base font-semibold text-foreground">
                          {job.title || scenario?.title || job.scenarioId}
                        </p>
                        <span className="rounded-full border border-white/10 px-2 py-0.5 text-[10px] uppercase tracking-[0.16em] text-muted-foreground">
                          {job.jobKind === "prompt" ? "Prompt" : "Playbook"}
                        </span>
                        <span
                          className={`text-xs font-medium uppercase tracking-[0.16em] ${statusTone(job.lastStatus)}`}
                        >
                          {job.lastStatus ?? "scheduled"}
                        </span>
                      </div>
                      <p className="text-sm text-muted-foreground">
                        {job.jobKind === "prompt"
                          ? job.promptText || "Recurring chat task"
                          : scenario?.summary ?? "First-party recurring playbook"}
                      </p>
                      <div className="flex flex-wrap gap-2 text-xs text-muted-foreground">
                        <span className="rounded-full border border-white/10 px-2.5 py-1">
                          {describeSchedule(inferScheduleDraft(job.schedule))}
                        </span>
                        <span className="rounded-full border border-white/10 px-2.5 py-1 font-mono">
                          {job.schedule}
                        </span>
                        <span className="rounded-full border border-white/10 px-2.5 py-1">
                          {job.enabledPackIds.length} packs
                        </span>
                        <span className="rounded-full border border-white/10 px-2.5 py-1">
                          {job.lastArtifactCount} artifacts
                        </span>
                      </div>
                    </div>

                    <div className="space-y-1 text-sm">
                      <p className="font-medium text-foreground">Next Run</p>
                      <p className="text-muted-foreground">
                        {formatTimestamp(job.nextRunAt)}
                      </p>
                      <p className="font-medium text-foreground">Last Run</p>
                      <p className="text-muted-foreground">
                        {formatTimestamp(job.lastRunAt)}
                      </p>
                    </div>

                    <div className="space-y-1 text-sm">
                      <p className="font-medium text-foreground">Approval Posture</p>
                      <p className="text-muted-foreground">
                        {job.lastRequiresExplicitApproval
                          ? "Explicit approval required"
                          : "No explicit approval needed"}
                      </p>
                      {job.lastBlockedByApproval ? (
                        <div className="flex items-center gap-2 text-amber-500">
                          <ShieldAlert className="size-4" />
                          <span>Blocked by Airlock</span>
                        </div>
                      ) : null}
                      {job.lastError ? (
                        <p className="text-red-500">{job.lastError}</p>
                      ) : null}
                      {job.lastChatId ? (
                        <p className="text-muted-foreground">
                          Last chat: {job.lastChatId}
                        </p>
                      ) : null}
                    </div>

                    <div className="flex items-start justify-end">
                      <div className="flex items-center gap-2">
                        <button
                          className="inline-flex items-center gap-2 rounded-2xl border border-white/10 bg-background px-4 py-2 text-sm font-medium text-foreground transition hover:bg-white/5 disabled:cursor-not-allowed disabled:opacity-60"
                          onClick={() => handleStartEdit(job)}
                          disabled={isSaving}
                        >
                          <Pencil className="size-4" />
                          Edit
                        </button>
                        <button
                          className="inline-flex items-center gap-2 rounded-2xl border border-red-500/20 bg-red-500/10 px-4 py-2 text-sm font-medium text-red-500 transition hover:bg-red-500/15 disabled:cursor-not-allowed disabled:opacity-60"
                          onClick={() => void handleDelete(job.id)}
                          disabled={isSaving}
                        >
                          <Trash2 className="size-4" />
                          Delete
                        </button>
                      </div>
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
