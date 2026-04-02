import { useEffect, useMemo, useState } from "react";
import { CalendarClock, Sparkles, X } from "lucide-react";

import * as tauri from "../../services/tauri";
import { ScheduleBuilder } from "./ScheduleBuilder";
import {
  DEFAULT_SCHEDULE_DRAFT,
  normalizeScheduleDraft,
} from "../../lib/schedule-builder";
import type { ScheduleDraft } from "../../lib/schedule-builder";

interface ScheduleTaskDialogProps {
  open: boolean;
  workspacePath: string;
  defaultPrompt?: string;
  onClose: () => void;
  onScheduled?: () => void | Promise<void>;
}

type ScheduleMode = "playbook" | "chat_prompt";

function buildPromptTaskTitle(seed: string): string {
  const compact = seed
    .replace(/\s+/g, " ")
    .trim();

  if (!compact) {
    return "Recurring chat task";
  }

  return compact.slice(0, 48);
}

export function ScheduleTaskDialog({
  open,
  workspacePath,
  defaultPrompt,
  onClose,
  onScheduled,
}: ScheduleTaskDialogProps) {
  const [mode, setMode] = useState<ScheduleMode>("playbook");
  const [scenarios, setScenarios] = useState<tauri.FirstRunScenarioDefinition[]>([]);
  const [scenarioId, setScenarioId] = useState("release_readiness");
  const [title, setTitle] = useState("Recurring workspace task");
  const [prompt, setPrompt] = useState(defaultPrompt ?? "");
  const [scheduleDraft, setScheduleDraft] = useState<ScheduleDraft>(
    DEFAULT_SCHEDULE_DRAFT,
  );
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    setPrompt(defaultPrompt ?? "");
    if ((defaultPrompt ?? "").trim()) {
      setMode("chat_prompt");
      setTitle(buildPromptTaskTitle(defaultPrompt ?? ""));
    } else {
      setMode("playbook");
      setTitle("Recurring workspace task");
    }
    setScheduleDraft(DEFAULT_SCHEDULE_DRAFT);
    setError(null);
    let cancelled = false;
    void tauri
      .listFirstRunScenarios()
      .then((nextScenarios) => {
        if (cancelled) return;
        setScenarios(nextScenarios);
        setScenarioId((current) =>
          nextScenarios.some((entry) => entry.id === current)
            ? current
            : (nextScenarios[0]?.id ?? "release_readiness"),
        );
      })
      .catch((nextError) => {
        if (cancelled) return;
        setError(
          nextError instanceof Error ? nextError.message : "Failed to load scenarios",
        );
      });

    return () => {
      cancelled = true;
    };
  }, [defaultPrompt, open]);

  const cronExpression = useMemo(
    () => normalizeScheduleDraft(scheduleDraft).cronExpression,
    [scheduleDraft],
  );

  if (!open) {
    return null;
  }

  const canSchedulePrompt = prompt.trim().length > 0 && title.trim().length > 0;

  const handleSchedule = async () => {
    setIsLoading(true);
    setError(null);
    try {
      if (mode === "playbook") {
        await tauri.createWorkspaceScheduledRun(workspacePath, scenarioId, cronExpression);
      } else {
        await tauri.createWorkspacePromptScheduledRun(
          workspacePath,
          title.trim(),
          prompt.trim(),
          cronExpression,
        );
      }
      await onScheduled?.();
      onClose();
    } catch (nextError) {
      setError(
        nextError instanceof Error
          ? nextError.message
          : "Failed to schedule recurring task",
      );
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div className="fixed inset-0 z-[120] flex items-center justify-center bg-black/45 px-4 backdrop-blur-md">
      <div className="w-full max-w-2xl rounded-[28px] border border-white/10 bg-background/95 shadow-[0_30px_120px_rgba(0,0,0,0.35)]">
        <div className="flex items-start justify-between gap-4 border-b border-white/10 px-6 py-5">
          <div className="space-y-2">
            <div className="flex items-center gap-3">
              <CalendarClock className="size-5 text-primary" />
              <h2 className="text-xl font-semibold tracking-tight text-foreground">
                Schedule Recurring Work
              </h2>
            </div>
            <p className="text-sm text-muted-foreground">
              Create a recurring playbook or schedule the current chat objective against this workspace.
            </p>
          </div>
          <button
            type="button"
            className="rounded-full p-2 text-muted-foreground transition hover:bg-white/5 hover:text-foreground"
            onClick={onClose}
          >
            <X className="size-4" />
          </button>
        </div>

        <div className="space-y-5 px-6 py-6">
          <div className="flex flex-wrap gap-2">
            <button
              type="button"
              className={`rounded-full px-4 py-2 text-sm font-medium transition ${
                mode === "playbook"
                  ? "bg-primary text-primary-foreground"
                  : "border border-white/10 bg-background text-muted-foreground hover:text-foreground"
              }`}
              onClick={() => setMode("playbook")}
            >
              Playbook
            </button>
            <button
              type="button"
              className={`rounded-full px-4 py-2 text-sm font-medium transition ${
                mode === "chat_prompt"
                  ? "bg-primary text-primary-foreground"
                  : "border border-white/10 bg-background text-muted-foreground hover:text-foreground"
              }`}
              onClick={() => setMode("chat_prompt")}
            >
              Chat task
            </button>
          </div>

          {mode === "playbook" ? (
            <label className="flex flex-col gap-2">
              <span className="text-xs font-semibold uppercase tracking-[0.16em] text-muted-foreground">
                Scenario
              </span>
              <select
                className="h-11 rounded-2xl border border-border/60 bg-background px-3 text-sm text-foreground outline-none"
                value={scenarioId}
                onChange={(event) => setScenarioId(event.target.value)}
                disabled={isLoading}
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
                  value={title}
                  onChange={(event) => setTitle(event.target.value)}
                  disabled={isLoading}
                  placeholder="Weekly release prep"
                />
              </label>

              <label className="flex flex-col gap-2">
                <span className="text-xs font-semibold uppercase tracking-[0.16em] text-muted-foreground">
                  Prompt
                </span>
                <textarea
                  className="min-h-[120px] rounded-2xl border border-border/60 bg-background px-3 py-3 text-sm text-foreground outline-none"
                  value={prompt}
                  onChange={(event) => setPrompt(event.target.value)}
                  disabled={isLoading}
                  placeholder="Describe the recurring task you want MaTE to run in this workspace."
                />
              </label>
            </div>
          )}

          <ScheduleBuilder
            value={scheduleDraft}
            onChange={setScheduleDraft}
            disabled={isLoading}
          />

          {error && (
            <div className="rounded-2xl border border-red-500/20 bg-red-500/5 px-4 py-3 text-sm text-red-500">
              {error}
            </div>
          )}
        </div>

        <div className="flex items-center justify-between gap-3 border-t border-white/10 px-6 py-5">
          <div className="flex items-center gap-2 text-xs text-muted-foreground">
            <Sparkles className="size-4" />
            <span>
              Dangerous actions still require explicit Airlock approval when the run executes.
            </span>
          </div>
          <div className="flex items-center gap-3">
            <button
              type="button"
              className="rounded-2xl border border-white/10 px-4 py-2 text-sm font-medium text-muted-foreground transition hover:bg-white/5 hover:text-foreground"
              onClick={onClose}
            >
              Cancel
            </button>
            <button
              type="button"
              className="rounded-2xl bg-primary px-4 py-2 text-sm font-semibold text-primary-foreground transition hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-60"
              onClick={() => void handleSchedule()}
              disabled={isLoading || (mode === "chat_prompt" && !canSchedulePrompt)}
            >
              {isLoading ? "Scheduling..." : "Schedule"}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
