import {
  DEFAULT_SCHEDULE_DRAFT,
  WEEKDAY_OPTIONS,
  buildCronExpression,
  describeSchedule,
  normalizeScheduleDraft,
} from "../../lib/schedule-builder";
import type { ScheduleDraft } from "../../lib/schedule-builder";

import { Select, ListBox } from "@heroui/react";

interface ScheduleBuilderProps {
  value: ScheduleDraft;
  onChange: (next: ScheduleDraft) => void;
  disabled?: boolean;
}

const selectionToValue = (selection: unknown): string | null => {
  if (typeof selection === "string") return selection;
  if (selection instanceof Set) {
    const first = selection.values().next().value;
    return typeof first === "string" ? first : null;
  }
  return null;
};

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
          <Select
            className="w-full"
            selectedKey={draft.preset}
            isDisabled={disabled}
            onSelectionChange={(selection) => {
              const value = selectionToValue(selection);
              if (value) onChange(updateDraft(draft, { preset: value as ScheduleDraft["preset"] }));
            }}
          >
            <Select.Trigger className="h-9 rounded-xl border border-white/10 bg-white/5 text-[12px] text-foreground shadow-sm px-2.5">
              <Select.Value />
              <Select.Indicator />
            </Select.Trigger>
            <Select.Popover className="bg-background/95 dark:bg-background/35 border border-default-200/70 dark:border-white/15 backdrop-blur-xl">
              <ListBox className="bg-transparent">
                <ListBox.Item id="daily" textValue="Daily">Daily</ListBox.Item>
                <ListBox.Item id="weekdays" textValue="Weekdays">Weekdays</ListBox.Item>
                <ListBox.Item id="weekly" textValue="Weekly">Weekly</ListBox.Item>
                <ListBox.Item id="monthly" textValue="Monthly">Monthly</ListBox.Item>
                <ListBox.Item id="custom" textValue="Custom cron">Custom cron</ListBox.Item>
              </ListBox>
            </Select.Popover>
          </Select>
        </label>

        {draft.preset !== "custom" && (
          <>
            <label className="flex flex-col gap-2">
              <span className="text-xs font-semibold uppercase tracking-[0.16em] text-muted-foreground">
                Hour
              </span>
              <Select
                className="w-full"
                selectedKey={String(draft.hour)}
                isDisabled={disabled}
                onSelectionChange={(selection) => {
                  const value = selectionToValue(selection);
                  if (value) onChange(updateDraft(draft, { hour: Number(value) }));
                }}
              >
                <Select.Trigger className="h-9 rounded-xl border border-white/10 bg-white/5 text-[12px] text-foreground shadow-sm px-2.5">
                  <Select.Value />
                  <Select.Indicator />
                </Select.Trigger>
                <Select.Popover className="bg-background/95 dark:bg-background/35 border border-default-200/70 dark:border-white/15 backdrop-blur-xl max-h-[300px] overflow-y-auto">
                  <ListBox className="bg-transparent">
                    {Array.from({ length: 24 }, (_, hour) => (
                      <ListBox.Item key={String(hour)} id={String(hour)} textValue={hour.toString().padStart(2, "0")}>
                        {hour.toString().padStart(2, "0")}
                      </ListBox.Item>
                    ))}
                  </ListBox>
                </Select.Popover>
              </Select>
            </label>

            <label className="flex flex-col gap-2">
              <span className="text-xs font-semibold uppercase tracking-[0.16em] text-muted-foreground">
                Minute
              </span>
              <Select
                className="w-full"
                selectedKey={String(draft.minute)}
                isDisabled={disabled}
                onSelectionChange={(selection) => {
                  const value = selectionToValue(selection);
                  if (value) onChange(updateDraft(draft, { minute: Number(value) }));
                }}
              >
                <Select.Trigger className="h-9 rounded-xl border border-white/10 bg-white/5 text-[12px] text-foreground shadow-sm px-2.5">
                  <Select.Value />
                  <Select.Indicator />
                </Select.Trigger>
                <Select.Popover className="bg-background/95 dark:bg-background/35 border border-default-200/70 dark:border-white/15 backdrop-blur-xl max-h-[300px] overflow-y-auto">
                  <ListBox className="bg-transparent">
                    {Array.from({ length: 12 }, (_, index) => index * 5).map((minute) => (
                      <ListBox.Item key={String(minute)} id={String(minute)} textValue={minute.toString().padStart(2, "0")}>
                        {minute.toString().padStart(2, "0")}
                      </ListBox.Item>
                    ))}
                  </ListBox>
                </Select.Popover>
              </Select>
            </label>
          </>
        )}
      </div>

      {draft.preset === "weekly" && (
        <label className="flex flex-col gap-2">
          <span className="text-xs font-semibold uppercase tracking-[0.16em] text-muted-foreground">
            Day of week
          </span>
          <Select
            className="w-full"
            selectedKey={String(draft.dayOfWeek)}
            isDisabled={disabled}
            onSelectionChange={(selection) => {
              const value = selectionToValue(selection);
              if (value) onChange(updateDraft(draft, { dayOfWeek: Number(value) }));
            }}
          >
            <Select.Trigger className="h-9 rounded-xl border border-white/10 bg-white/5 text-[12px] text-foreground shadow-sm px-2.5">
              <Select.Value />
              <Select.Indicator />
            </Select.Trigger>
            <Select.Popover className="bg-background/95 dark:bg-background/35 border border-default-200/70 dark:border-white/15 backdrop-blur-xl">
              <ListBox className="bg-transparent">
                {WEEKDAY_OPTIONS.map((option) => (
                  <ListBox.Item key={String(option.value)} id={String(option.value)} textValue={option.label}>
                    {option.label}
                  </ListBox.Item>
                ))}
              </ListBox>
            </Select.Popover>
          </Select>
        </label>
      )}

      {draft.preset === "monthly" && (
        <label className="flex flex-col gap-2">
          <span className="text-xs font-semibold uppercase tracking-[0.16em] text-muted-foreground">
            Day of month
          </span>
          <Select
            className="w-full"
            selectedKey={String(draft.dayOfMonth)}
            isDisabled={disabled}
            onSelectionChange={(selection) => {
              const value = selectionToValue(selection);
              if (value) onChange(updateDraft(draft, { dayOfMonth: Number(value) }));
            }}
          >
            <Select.Trigger className="h-11 rounded-2xl border border-white/10 bg-white/5 text-sm text-foreground shadow-sm px-3">
              <Select.Value />
              <Select.Indicator />
            </Select.Trigger>
            <Select.Popover className="bg-background/95 dark:bg-background/35 border border-default-200/70 dark:border-white/15 backdrop-blur-xl max-h-[300px] overflow-y-auto">
              <ListBox className="bg-transparent">
                {Array.from({ length: 28 }, (_, index) => index + 1).map((day) => (
                  <ListBox.Item key={String(day)} id={String(day)} textValue={String(day)}>
                    {day}
                  </ListBox.Item>
                ))}
              </ListBox>
            </Select.Popover>
          </Select>
        </label>
      )}

      {draft.preset === "custom" && (
        <label className="flex flex-col gap-2">
          <span className="text-xs font-semibold uppercase tracking-[0.16em] text-muted-foreground">
            Cron expression
          </span>
          <input
            className="h-9 rounded-xl border border-border/60 bg-background px-2.5 text-[12px] text-foreground outline-none"
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

      <div className="rounded-xl border border-white/10 bg-background/35 px-3 py-2">
        <p className="text-[10px] font-semibold uppercase tracking-[0.16em] text-muted-foreground">
          Schedule Preview
        </p>
        <p className="mt-1 text-[13px] text-foreground">{describeSchedule(draft)}</p>
        <p className="mt-0.5 font-mono text-[11px] text-muted-foreground">
          {draft.preset === "custom" ? draft.cronExpression : buildCronExpression(draft)}
        </p>
      </div>
    </div>
  );
}
