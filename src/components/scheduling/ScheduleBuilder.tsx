import {
  DEFAULT_SCHEDULE_DRAFT,
  WEEKDAY_OPTIONS,
  buildCronExpression,
  describeSchedule,
  normalizeScheduleDraft,
} from "../../lib/schedule-builder";
import type { ScheduleDraft } from "../../lib/schedule-builder";

interface ScheduleBuilderProps {
  value: ScheduleDraft;
  onChange: (next: ScheduleDraft) => void;
  disabled?: boolean;
}

function updateDraft(
  current: ScheduleDraft,
  patch: Partial<ScheduleDraft>,
): ScheduleDraft {
  const next = normalizeScheduleDraft({
    ...current,
    ...patch,
  });

  if (next.preset !== "custom") {
    next.cronExpression = buildCronExpression(next);
  }

  return next;
}

export function ScheduleBuilder({
  value,
  onChange,
  disabled = false,
}: ScheduleBuilderProps) {
  const draft = normalizeScheduleDraft(value ?? DEFAULT_SCHEDULE_DRAFT);

  return (
    <div className="grid gap-4">
      <div className="grid gap-4 md:grid-cols-3">
        <label className="flex flex-col gap-2">
          <span className="text-xs font-semibold uppercase tracking-[0.16em] text-muted-foreground">
            Frequency
          </span>
          <select
            className="h-11 rounded-2xl border border-border/60 bg-background px-3 text-sm text-foreground outline-none"
            value={draft.preset}
            onChange={(event) =>
              onChange(updateDraft(draft, { preset: event.target.value as ScheduleDraft["preset"] }))
            }
            disabled={disabled}
          >
            <option value="daily">Daily</option>
            <option value="weekdays">Weekdays</option>
            <option value="weekly">Weekly</option>
            <option value="monthly">Monthly</option>
            <option value="custom">Custom cron</option>
          </select>
        </label>

        {draft.preset !== "custom" && (
          <>
            <label className="flex flex-col gap-2">
              <span className="text-xs font-semibold uppercase tracking-[0.16em] text-muted-foreground">
                Hour
              </span>
              <select
                className="h-11 rounded-2xl border border-border/60 bg-background px-3 text-sm text-foreground outline-none"
                value={draft.hour}
                onChange={(event) =>
                  onChange(updateDraft(draft, { hour: Number(event.target.value) }))
                }
                disabled={disabled}
              >
                {Array.from({ length: 24 }, (_, hour) => (
                  <option key={hour} value={hour}>
                    {hour.toString().padStart(2, "0")}
                  </option>
                ))}
              </select>
            </label>

            <label className="flex flex-col gap-2">
              <span className="text-xs font-semibold uppercase tracking-[0.16em] text-muted-foreground">
                Minute
              </span>
              <select
                className="h-11 rounded-2xl border border-border/60 bg-background px-3 text-sm text-foreground outline-none"
                value={draft.minute}
                onChange={(event) =>
                  onChange(updateDraft(draft, { minute: Number(event.target.value) }))
                }
                disabled={disabled}
              >
                {Array.from({ length: 12 }, (_, index) => index * 5).map((minute) => (
                  <option key={minute} value={minute}>
                    {minute.toString().padStart(2, "0")}
                  </option>
                ))}
              </select>
            </label>
          </>
        )}
      </div>

      {draft.preset === "weekly" && (
        <label className="flex flex-col gap-2">
          <span className="text-xs font-semibold uppercase tracking-[0.16em] text-muted-foreground">
            Day of week
          </span>
          <select
            className="h-11 rounded-2xl border border-border/60 bg-background px-3 text-sm text-foreground outline-none"
            value={draft.dayOfWeek}
            onChange={(event) =>
              onChange(updateDraft(draft, { dayOfWeek: Number(event.target.value) }))
            }
            disabled={disabled}
          >
            {WEEKDAY_OPTIONS.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </label>
      )}

      {draft.preset === "monthly" && (
        <label className="flex flex-col gap-2">
          <span className="text-xs font-semibold uppercase tracking-[0.16em] text-muted-foreground">
            Day of month
          </span>
          <select
            className="h-11 rounded-2xl border border-border/60 bg-background px-3 text-sm text-foreground outline-none"
            value={draft.dayOfMonth}
            onChange={(event) =>
              onChange(updateDraft(draft, { dayOfMonth: Number(event.target.value) }))
            }
            disabled={disabled}
          >
            {Array.from({ length: 28 }, (_, index) => index + 1).map((day) => (
              <option key={day} value={day}>
                {day}
              </option>
            ))}
          </select>
        </label>
      )}

      {draft.preset === "custom" && (
        <label className="flex flex-col gap-2">
          <span className="text-xs font-semibold uppercase tracking-[0.16em] text-muted-foreground">
            Cron expression
          </span>
          <input
            className="h-11 rounded-2xl border border-border/60 bg-background px-3 text-sm text-foreground outline-none"
            value={draft.cronExpression}
            onChange={(event) =>
              onChange(
                normalizeScheduleDraft({
                  ...draft,
                  cronExpression: event.target.value,
                }),
              )
            }
            disabled={disabled}
            placeholder="0 0 9 * * * *"
          />
          <p className="text-xs text-muted-foreground">
            Format: second minute hour day month weekday year
          </p>
        </label>
      )}

      <div className="rounded-2xl border border-white/10 bg-background/35 px-4 py-3">
        <p className="text-xs font-semibold uppercase tracking-[0.16em] text-muted-foreground">
          Schedule Preview
        </p>
        <p className="mt-1 text-sm text-foreground">{describeSchedule(draft)}</p>
        <p className="mt-1 font-mono text-xs text-muted-foreground">
          {draft.preset === "custom" ? draft.cronExpression : buildCronExpression(draft)}
        </p>
      </div>
    </div>
  );
}
