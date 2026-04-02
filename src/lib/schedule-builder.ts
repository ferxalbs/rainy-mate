export type SchedulePreset =
  | "daily"
  | "weekdays"
  | "weekly"
  | "monthly"
  | "custom";

export interface ScheduleDraft {
  preset: SchedulePreset;
  hour: number;
  minute: number;
  dayOfWeek: number;
  dayOfMonth: number;
  cronExpression: string;
}

export const WEEKDAY_OPTIONS = [
  { value: 0, label: "Sunday" },
  { value: 1, label: "Monday" },
  { value: 2, label: "Tuesday" },
  { value: 3, label: "Wednesday" },
  { value: 4, label: "Thursday" },
  { value: 5, label: "Friday" },
  { value: 6, label: "Saturday" },
] as const;

const WEEKDAY_LABELS = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];

export const DEFAULT_SCHEDULE_DRAFT: ScheduleDraft = {
  preset: "daily",
  hour: 9,
  minute: 0,
  dayOfWeek: 1,
  dayOfMonth: 1,
  cronExpression: "0 0 9 * * * *",
};

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, Number.isFinite(value) ? value : min));
}

function twoDigits(value: number): string {
  return value.toString().padStart(2, "0");
}

export function buildCronExpression(draft: ScheduleDraft): string {
  const hour = clamp(draft.hour, 0, 23);
  const minute = clamp(draft.minute, 0, 59);
  const dayOfWeek = clamp(draft.dayOfWeek, 0, 6);
  const dayOfMonth = clamp(draft.dayOfMonth, 1, 28);

  switch (draft.preset) {
    case "weekdays":
      return `0 ${minute} ${hour} * * 1-5 *`;
    case "weekly":
      return `0 ${minute} ${hour} * * ${dayOfWeek} *`;
    case "monthly":
      return `0 ${minute} ${hour} ${dayOfMonth} * * *`;
    case "custom":
      return draft.cronExpression.trim();
    case "daily":
    default:
      return `0 ${minute} ${hour} * * * *`;
  }
}

export function inferScheduleDraft(
  cronExpression: string,
): ScheduleDraft {
  const trimmed = cronExpression.trim();
  const parts = trimmed.split(/\s+/);

  if (parts.length === 7) {
    const [second, minuteRaw, hourRaw, dayRaw, monthRaw, weekdayRaw, yearRaw] = parts;
    const minute = Number(minuteRaw);
    const hour = Number(hourRaw);

    if (
      second === "0" &&
      Number.isInteger(minute) &&
      Number.isInteger(hour) &&
      monthRaw === "*" &&
      yearRaw === "*"
    ) {
      if (dayRaw === "*" && weekdayRaw === "*") {
        return normalizeScheduleDraft({
          preset: "daily",
          hour,
          minute,
          cronExpression: trimmed,
        });
      }

      if (dayRaw === "*" && weekdayRaw === "1-5") {
        return normalizeScheduleDraft({
          preset: "weekdays",
          hour,
          minute,
          cronExpression: trimmed,
        });
      }

      if (dayRaw === "*" && /^[0-6]$/.test(weekdayRaw)) {
        return normalizeScheduleDraft({
          preset: "weekly",
          hour,
          minute,
          dayOfWeek: Number(weekdayRaw),
          cronExpression: trimmed,
        });
      }

      if (/^(?:[1-9]|1\d|2\d)$/.test(dayRaw) && weekdayRaw === "*") {
        return normalizeScheduleDraft({
          preset: "monthly",
          hour,
          minute,
          dayOfMonth: Number(dayRaw),
          cronExpression: trimmed,
        });
      }
    }
  }

  return normalizeScheduleDraft({
    preset: "custom",
    cronExpression: trimmed,
  });
}

export function describeSchedule(draft: ScheduleDraft): string {
  const time = `${twoDigits(clamp(draft.hour, 0, 23))}:${twoDigits(
    clamp(draft.minute, 0, 59),
  )}`;

  switch (draft.preset) {
    case "weekdays":
      return `Every weekday at ${time}`;
    case "weekly":
      return `Every ${WEEKDAY_LABELS[clamp(draft.dayOfWeek, 0, 6)]} at ${time}`;
    case "monthly":
      return `Every month on day ${clamp(draft.dayOfMonth, 1, 28)} at ${time}`;
    case "custom":
      return draft.cronExpression.trim()
        ? `Custom cron: ${draft.cronExpression.trim()}`
        : "Enter a cron expression";
    case "daily":
    default:
      return `Every day at ${time}`;
  }
}

export function normalizeScheduleDraft(
  draft?: Partial<ScheduleDraft>,
): ScheduleDraft {
  const next: ScheduleDraft = {
    preset: draft?.preset ?? DEFAULT_SCHEDULE_DRAFT.preset,
    hour: clamp(draft?.hour ?? DEFAULT_SCHEDULE_DRAFT.hour, 0, 23),
    minute: clamp(draft?.minute ?? DEFAULT_SCHEDULE_DRAFT.minute, 0, 59),
    dayOfWeek: clamp(draft?.dayOfWeek ?? DEFAULT_SCHEDULE_DRAFT.dayOfWeek, 0, 6),
    dayOfMonth: clamp(draft?.dayOfMonth ?? DEFAULT_SCHEDULE_DRAFT.dayOfMonth, 1, 28),
    cronExpression:
      draft?.cronExpression?.trim() || DEFAULT_SCHEDULE_DRAFT.cronExpression,
  };

  return {
    ...next,
    cronExpression: next.preset === "custom" ? next.cronExpression : buildCronExpression(next),
  };
}
