import { AgentSoul } from "../../../types/agent-spec";

interface SoulEditorProps {
  soul: AgentSoul;
  onChange: (soul: AgentSoul) => void;
}

// Moved OUTSIDE to prevent remounting
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

export function SoulEditor({ soul, onChange }: SoulEditorProps) {
  const handleChange = (field: keyof AgentSoul, value: string) => {
    onChange({
      ...soul,
      [field]: value,
    });
  };

  return (
    <div className="space-y-8 animate-appear">
      {/* Header Section */}
      <div className="flex flex-col gap-1 border-b border-border/10 pb-6">
        <h3 className="text-2xl font-bold text-foreground tracking-tight">
          Identity
        </h3>
        <p className="text-muted-foreground text-sm">
          Define the core persona and purpose.
        </p>
      </div>

      <div className="space-y-6">
        {/* Core Info */}
        <div className="grid grid-cols-1 md:grid-cols-4 gap-6">
          <div className="md:col-span-3">
            <Field label="Name">
              <input
                type="text"
                placeholder="e.g. Neo"
                value={soul.name}
                onChange={(e) => handleChange("name", e.target.value)}
                className="w-full bg-transparent border-b border-border/40 text-lg font-bold text-foreground placeholder:text-muted-foreground/30 px-0 py-2 focus:outline-none focus:border-primary transition-colors"
              />
            </Field>
          </div>
          <div>
            <Field label="Version">
              <input
                type="text"
                placeholder="1.0.0"
                value={soul.version}
                onChange={(e) => handleChange("version", e.target.value)}
                className="w-full bg-transparent border-b border-border/40 text-sm font-mono text-primary placeholder:text-muted-foreground/30 px-0 py-2 focus:outline-none focus:border-border/60 transition-colors"
              />
            </Field>
          </div>
        </div>

        <Field label="Description">
          <textarea
            placeholder="What is this agent's primary directive?"
            value={soul.description}
            onChange={(e) => handleChange("description", e.target.value)}
            className="w-full bg-card/40 hover:bg-card/60 backdrop-blur-md rounded-xl px-4 py-3 text-sm text-foreground placeholder:text-muted-foreground/50 leading-relaxed border border-border/20 focus:outline-none focus:border-primary/50 focus:ring-1 focus:ring-primary/20 transition-all resize-none shadow-sm"
            rows={3}
          />
        </Field>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          <Field label="Personality">
            <textarea
              placeholder="e.g. Stoic, precise, efficient"
              value={soul.personality}
              onChange={(e) => handleChange("personality", e.target.value)}
              className="w-full bg-card/40 hover:bg-card/60 backdrop-blur-md rounded-xl px-4 py-3 text-sm text-foreground placeholder:text-muted-foreground/50 leading-relaxed border border-border/20 focus:outline-none focus:border-primary/50 focus:ring-1 focus:ring-primary/20 transition-all resize-none shadow-sm"
              rows={3}
            />
          </Field>
          <Field label="Tone">
            <textarea
              placeholder="e.g. Formal, academic, witty"
              value={soul.tone}
              onChange={(e) => handleChange("tone", e.target.value)}
              className="w-full bg-card/40 hover:bg-card/60 backdrop-blur-md rounded-xl px-4 py-3 text-sm text-foreground placeholder:text-muted-foreground/50 leading-relaxed border border-border/20 focus:outline-none focus:border-primary/50 focus:ring-1 focus:ring-primary/20 transition-all resize-none shadow-sm"
              rows={3}
            />
          </Field>
        </div>

        <div className="pt-6 border-t border-border/10">
          <Field label="Soul Content (System Prompt)" className="w-full">
            <div className="w-full h-[500px] bg-card/40 hover:bg-card/60 rounded-xl border border-border/20 group-hover:border-primary/50 transition-all shadow-sm focus-within:ring-1 focus-within:ring-primary flex flex-col relative overflow-hidden backdrop-blur-md">
              {/* Editor Header / Toolbar simulation */}
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

              <textarea
                aria-label="Soul Content"
                placeholder="# Define your agent's core directives, behavior, and limitations here..."
                value={soul.soul_content}
                onChange={(e) => handleChange("soul_content", e.target.value)}
                className="flex-1 w-full bg-transparent resize-none p-4 font-mono text-xs leading-relaxed text-primary/90 placeholder:text-muted-foreground/30 focus:outline-none selection:bg-primary/30"
                spellCheck={false}
              />
            </div>
          </Field>
        </div>
      </div>
    </div>
  );
}
