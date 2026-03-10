import { Input, TextArea } from "@heroui/react";
import { Bot } from "lucide-react";
import type { AgentSoul } from "../../../types/agent-spec";

interface SoulEditorProps {
  soul: AgentSoul;
  onChange: (soul: AgentSoul) => void;
}

const Field = ({
  label,
  children,
  className = "",
}: {
  label: string;
  children: React.ReactNode;
  className?: string;
}) => (
  <div className={`group ${className}`}>
    <label className="block text-muted-foreground group-hover:text-primary text-[10px] font-bold uppercase tracking-widest mb-2 transition-colors duration-300">
      {label}
    </label>
    {children}
  </div>
);

const controlClass =
  "w-full bg-default-100/80 dark:bg-white/[0.08] border-default-300/70 dark:border-white/15 data-[hover=true]:bg-default-100 dark:data-[hover=true]:bg-white/[0.12] shadow-sm";

export function SoulEditor({ soul, onChange }: SoulEditorProps) {
  const handleChange = (field: keyof AgentSoul, value: string) => {
    onChange({
      ...soul,
      [field]: value,
    });
  };

  return (
    <div className="space-y-8 animate-appear">
      <div className="relative overflow-hidden rounded-2xl border border-border/20 bg-card/40 backdrop-blur-xl p-5">
        <div className="absolute -top-20 right-[-60px] w-[280px] h-[280px] rounded-full bg-primary/10 blur-[85px] pointer-events-none" />
        <div className="absolute -bottom-24 left-[-80px] w-[260px] h-[260px] rounded-full bg-foreground/[0.04] blur-[90px] pointer-events-none" />
        <div className="relative z-10 flex flex-col gap-1">
          <h3 className="text-2xl font-bold text-foreground tracking-tight flex items-center gap-2">
            <Bot className="size-5 text-primary" />
            Identity
          </h3>
          <p className="text-muted-foreground text-sm">
            Define the core persona and purpose.
          </p>
        </div>
      </div>

      <div className="rounded-2xl border border-border/20 bg-card/35 backdrop-blur-md p-6 space-y-6">
        <p className="text-muted-foreground text-sm">
          Build an intentional identity for how this agent responds and reasons.
        </p>

        <div className="grid grid-cols-1 md:grid-cols-4 gap-6">
          <div className="md:col-span-3">
            <Field label="Name">
              <Input
                placeholder="e.g. Neo"
                value={soul.name}
                onChange={(e) => handleChange("name", e.target.value)}
                className={controlClass}
              />
            </Field>
          </div>
          <div>
            <Field label="Version">
              <Input
                placeholder="1.0.0"
                value={soul.version}
                onChange={(e) => handleChange("version", e.target.value)}
                className={`${controlClass} font-mono`}
              />
            </Field>
          </div>
        </div>

        <Field label="Description">
          <TextArea
            placeholder="What is this agent's primary directive?"
            value={soul.description}
            onChange={(e) => handleChange("description", e.target.value)}
            className={controlClass}
            rows={3}
          />
        </Field>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          <Field label="Personality">
            <TextArea
              placeholder="e.g. Stoic, precise, efficient"
              value={soul.personality}
              onChange={(e) => handleChange("personality", e.target.value)}
              className={controlClass}
              rows={3}
            />
          </Field>
          <Field label="Tone">
            <TextArea
              placeholder="e.g. Formal, academic, witty"
              value={soul.tone}
              onChange={(e) => handleChange("tone", e.target.value)}
              className={controlClass}
              rows={3}
            />
          </Field>
        </div>

        <div className="pt-6 border-t border-border/10">
          <Field label="Soul Content (System Prompt)" className="w-full">
            <div className="w-full h-[500px] bg-default-100/60 dark:bg-white/[0.06] rounded-xl border border-default-300/60 dark:border-white/15 group-hover:border-primary/50 transition-all shadow-sm focus-within:ring-1 focus-within:ring-primary flex flex-col relative overflow-hidden backdrop-blur-md">
              <div className="h-8 shrink-0 border-b border-border/10 bg-black/20 flex items-center px-3 gap-2">
                <div className="flex gap-1.5">
                  <div className="size-2.5 rounded-full bg-red-500/20" />
                  <div className="size-2.5 rounded-full bg-amber-500/20" />
                  <div className="size-2.5 rounded-full bg-green-500/20" />
                </div>
                <span className="text-[10px] uppercase tracking-widest text-muted-foreground/60 ml-2 font-bold select-none">
                  SYSTEM_PROMPT.md
                </span>
              </div>

              <TextArea
                aria-label="Soul Content"
                placeholder="# Define your agent's core directives, behavior, and limitations here..."
                value={soul.soul_content}
                onChange={(e) => handleChange("soul_content", e.target.value)}
                className="flex-1 w-full bg-transparent border-0 shadow-none font-mono text-xs"
                rows={20}
                spellCheck={false}
              />
            </div>
          </Field>
        </div>
      </div>
    </div>
  );
}
