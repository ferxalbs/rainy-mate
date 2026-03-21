import { Button, Input, Slider } from "@heroui/react";
import { Brain, Gauge, Hash, RotateCcw, Sparkles } from "lucide-react";
import type { ChangeEvent, ReactNode } from "react";
import { UnifiedModelSelector } from "../../ai/UnifiedModelSelector";

interface RuntimePanelProps {
  model: string;
  temperature: number;
  maxTokens: number;
  runtimeMode: "single" | "parallel_supervisor" | "hierarchical_supervisor";
  maxSpecialists: number;
  verificationRequired: boolean;
  onChange: (updates: {
    model?: string;
    temperature?: number;
    maxTokens?: number;
    runtimeMode?: "single" | "parallel_supervisor" | "hierarchical_supervisor";
    maxSpecialists?: number;
    verificationRequired?: boolean;
  }) => void;
}

const Field = ({
  label,
  children,
  className = "",
}: {
  label: string;
  children: ReactNode;
  className?: string;
}) => (
  <div className={`group ${className}`}>
    <label className="block text-muted-foreground group-hover:text-primary text-[10px] font-bold uppercase tracking-widest mb-2 transition-colors duration-300">
      {label}
    </label>
    {children}
  </div>
);

function clampTokens(value: number): number {
  if (!Number.isFinite(value)) return 4096;
  return Math.max(256, Math.min(131072, Math.round(value)));
}

export function RuntimePanel({
  model,
  temperature,
  maxTokens,
  runtimeMode,
  maxSpecialists,
  verificationRequired,
  onChange,
}: RuntimePanelProps) {
  const specialistMax = runtimeMode === "parallel_supervisor" ? 2 : 3;
  const tempLabel =
    temperature < 0.3
      ? "Precise"
      : temperature < 0.65
        ? "Balanced"
        : "Creative";

  const applyPreset = (preset: {
    temperature: number;
    maxTokens: number;
  }) => {
    onChange({
      temperature: preset.temperature,
      maxTokens: clampTokens(preset.maxTokens),
    });
  };

  return (
    <div className="space-y-8 animate-appear">
      <div className="relative overflow-hidden rounded-2xl border border-border/20 bg-card/40 backdrop-blur-xl p-5">
        <div className="absolute -top-20 right-[-60px] w-[280px] h-[280px] rounded-full bg-primary/10 blur-[85px] pointer-events-none" />
        <div className="absolute -bottom-24 left-[-80px] w-[260px] h-[260px] rounded-full bg-foreground/[0.04] blur-[90px] pointer-events-none" />

        <div className="relative z-10 flex flex-col gap-1">
          <h3 className="text-2xl font-bold text-foreground tracking-tight flex items-center gap-2">
            <Brain className="size-5 text-primary" />
            Runtime
          </h3>
          <p className="text-muted-foreground text-sm">
            Tune how your agent thinks, writes, and spends tokens.
          </p>
        </div>
      </div>

      <div className="grid grid-cols-1 xl:grid-cols-3 gap-6">
        <section className="xl:col-span-2 rounded-2xl border border-border/20 bg-card/35 backdrop-blur-md p-5">
          <Field label="Runtime Mode" className="mb-6">
            <div className="flex flex-wrap gap-2">
              <Button
                size="sm"
                variant={runtimeMode === "single" ? "primary" : "ghost"}
                className="rounded-full"
                onPress={() => onChange({ runtimeMode: "single" })}
              >
                Single agent
              </Button>
              <Button
                size="sm"
                variant={
                  runtimeMode === "parallel_supervisor" ? "primary" : "ghost"
                }
                className="rounded-full"
                onPress={() => onChange({ runtimeMode: "parallel_supervisor" })}
              >
                Parallel
              </Button>
              <Button
                size="sm"
                variant={
                  runtimeMode === "hierarchical_supervisor"
                    ? "primary"
                    : "ghost"
                }
                className="rounded-full"
                onPress={() =>
                  onChange({ runtimeMode: "hierarchical_supervisor" })
                }
              >
                Hierarchical
              </Button>
            </div>
            <p className="mt-2 text-xs text-muted-foreground/90">
              Parallel mode runs bounded specialist lanes concurrently. Hierarchical mode keeps the principal agent in charge of chained sub-agent work.
            </p>
          </Field>

          <Field label="Model">
            <div className="rounded-xl border border-border/30 bg-background/35 px-3 py-2">
              <UnifiedModelSelector
                selectedModelId={model}
                onSelect={(modelId) => onChange({ model: modelId })}
                filter="chat"
              />
            </div>
            <p className="mt-2 text-xs text-muted-foreground/90">
              Catalog comes from your Rainy API v3 key (workspace-scoped).
            </p>
            <p className="mt-1 text-[11px] text-primary/80 font-mono break-all">
              {model || "No model selected"}
            </p>
          </Field>

          <div className="mt-6 space-y-4">
            <Field label="Quick Presets">
              <div className="flex flex-wrap gap-2">
                <Button
                  size="sm"
                  variant="secondary"
                  className="rounded-full bg-emerald-500/10 text-emerald-400"
                  onPress={() => applyPreset({ temperature: 0.15, maxTokens: 2048 })}
                >
                  <Gauge className="size-3.5 mr-1" />
                  Deterministic
                </Button>
                <Button
                  size="sm"
                  variant="secondary"
                  className="rounded-full bg-blue-500/10 text-blue-400"
                  onPress={() => applyPreset({ temperature: 0.4, maxTokens: 4096 })}
                >
                  <Hash className="size-3.5 mr-1" />
                  Balanced
                </Button>
                <Button
                  size="sm"
                  variant="secondary"
                  className="rounded-full bg-amber-500/10 text-amber-400"
                  onPress={() => applyPreset({ temperature: 0.8, maxTokens: 8192 })}
                >
                  <Sparkles className="size-3.5 mr-1" />
                  Creative
                </Button>
                <Button
                  size="sm"
                  variant="ghost"
                  className="rounded-full"
                  onPress={() => applyPreset({ temperature: 0.4, maxTokens: 4096 })}
                >
                  <RotateCcw className="size-3.5 mr-1" />
                  Reset
                </Button>
              </div>
            </Field>
          </div>
        </section>

        <section className="rounded-2xl border border-border/20 bg-card/35 backdrop-blur-md p-5 space-y-6">
          <Field label="Specialist Count">
            <div className="space-y-3">
              <Slider
                minValue={1}
                maxValue={specialistMax}
                step={1}
                value={maxSpecialists}
                isDisabled={runtimeMode === "single"}
                onChange={(value) =>
                  onChange({
                    maxSpecialists: Array.isArray(value)
                      ? Number(value[0] ?? maxSpecialists)
                      : Number(value),
                  })
                }
                className="max-w-full"
              >
                <Slider.Track className="h-1.5 bg-default-200 dark:bg-white/10 rounded-full">
                  <Slider.Fill className="bg-primary h-full rounded-full" />
                  <Slider.Thumb className="size-4 bg-white border-2 border-primary rounded-full shadow-lg hocus:scale-110 transition-transform cursor-pointer" />
                </Slider.Track>
              </Slider>
              <div className="flex items-center justify-between text-xs">
                <span className="font-mono text-primary">{maxSpecialists}</span>
                <span className="text-muted-foreground">
                  {runtimeMode === "single"
                    ? "Single mode only uses one lane"
                    : runtimeMode === "parallel_supervisor"
                      ? "Parallel lanes (CPU-safe cap)"
                      : "Specialist budget"}
                </span>
              </div>
            </div>
          </Field>

          <Field label="Verifier">
            <div className="flex flex-wrap gap-2">
              <Button
                size="sm"
                variant={verificationRequired ? "primary" : "ghost"}
                isDisabled={runtimeMode === "single"}
                className="rounded-full"
                onPress={() => onChange({ verificationRequired: true })}
              >
                Required
              </Button>
              <Button
                size="sm"
                variant={!verificationRequired ? "primary" : "ghost"}
                isDisabled={runtimeMode === "single"}
                className="rounded-full"
                onPress={() => onChange({ verificationRequired: false })}
              >
                Optional
              </Button>
            </div>
            <p className="mt-2 text-xs text-muted-foreground/90">
              The verifier runs after write-like execution and checks resulting state with read-only tools.
            </p>
          </Field>

          <Field label="Temperature">
            <div className="space-y-3">
              <Slider
                minValue={0}
                maxValue={1}
                step={0.05}
                value={temperature}
                onChange={(value) =>
                  onChange({
                    temperature: Array.isArray(value)
                      ? Number(value[0] ?? temperature)
                      : Number(value),
                  })
                }
                className="max-w-full"
              >
                <Slider.Track className="h-1.5 bg-default-200 dark:bg-white/10 rounded-full">
                  <Slider.Fill className="bg-primary h-full rounded-full" />
                  <Slider.Thumb className="size-4 bg-white border-2 border-primary rounded-full shadow-lg hocus:scale-110 transition-transform cursor-pointer" />
                </Slider.Track>
              </Slider>
              <div className="flex items-center justify-between text-xs">
                <span className="font-mono text-primary">{temperature.toFixed(2)}</span>
                <span className="text-muted-foreground">{tempLabel}</span>
              </div>
            </div>
          </Field>

          <Field label="Max Output Tokens">
            <div className="space-y-3">
              <Slider
                minValue={256}
                maxValue={131072}
                step={256}
                value={maxTokens}
                onChange={(value) =>
                  onChange({
                    maxTokens: clampTokens(
                      Array.isArray(value) ? Number(value[0] ?? maxTokens) : Number(value),
                    ),
                  })
                }
                className="max-w-full"
              >
                <Slider.Track className="h-1.5 bg-default-200 dark:bg-white/10 rounded-full">
                  <Slider.Fill className="bg-primary h-full rounded-full" />
                  <Slider.Thumb className="size-4 bg-white border-2 border-primary rounded-full shadow-lg hocus:scale-110 transition-transform cursor-pointer" />
                </Slider.Track>
              </Slider>

              <Input
                type="number"
                min={256}
                max={131072}
                step={256}
                value={String(maxTokens)}
                onChange={(e: ChangeEvent<HTMLInputElement>) =>
                  onChange({
                    maxTokens: clampTokens(
                      Number.parseInt(e.target.value || String(maxTokens), 10),
                    ),
                  })
                }
                className="w-full bg-background/85 dark:bg-background/20 border border-default-300/70 dark:border-white/15 rounded-xl px-3 py-2 text-sm font-mono shadow-sm focus-within:!border-primary/50 transition-all backdrop-blur-md"
              />

              <div className="text-xs text-muted-foreground font-mono">
                {maxTokens.toLocaleString()} tokens
              </div>
            </div>
          </Field>
        </section>
      </div>
    </div>
  );
}
