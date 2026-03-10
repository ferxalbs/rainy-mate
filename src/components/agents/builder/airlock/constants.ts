import type { AirlockLevel } from "../../../../types/airlock";

export const sectionTitleClass =
  "text-[10px] font-bold uppercase tracking-widest text-muted-foreground";

export const inputClass =
  "w-full bg-default-100/80 dark:bg-white/[0.08] border-default-300/70 dark:border-white/15 data-[hover=true]:bg-default-100 dark:data-[hover=true]:bg-white/[0.12] rounded-xl shadow-sm";

export const LEVELS: Array<{
  level: AirlockLevel;
  title: string;
  tone: string;
  modalBehavior: string;
}> = [
  {
    level: 0,
    title: "Safe",
    tone: "text-emerald-500",
    modalBehavior: "Auto-approved (no modal)",
  },
  {
    level: 1,
    title: "Sensitive",
    tone: "text-amber-500",
    modalBehavior: "Approval modal (notification gate)",
  },
  {
    level: 2,
    title: "Dangerous",
    tone: "text-red-500",
    modalBehavior: "Explicit approval modal required",
  },
];
