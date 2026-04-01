import { useCallback, useEffect, useMemo, useState } from "react";
import { BrainCircuit, CheckCircle2, FolderKanban, Shield, Sparkles } from "lucide-react";
import { Button } from "@heroui/react";

import * as tauri from "../../services/tauri";

interface WorkspaceLaunchpadProps {
  workspacePath: string;
  onRunScenario: (scenarioId: string) => Promise<void> | void;
}

const TRUST_PRESETS: Array<{
  id: "conservative" | "balanced" | "elevated";
  title: string;
  summary: string;
}> = [
  {
    id: "conservative",
    title: "Conservative",
    summary: "Read-heavy default for sensitive workspaces and documentation-heavy flows.",
  },
  {
    id: "balanced",
    title: "Balanced",
    summary: "Recommended for active developer workspaces with guided file creation and bounded autonomy.",
  },
  {
    id: "elevated",
    title: "Elevated",
    summary: "Broader operational latitude for trusted workspaces that need stronger autonomy.",
  },
];

export function WorkspaceLaunchpad({
  workspacePath,
  onRunScenario,
}: WorkspaceLaunchpadProps) {
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [launchpad, setLaunchpad] = useState<tauri.WorkspaceLaunchpadSummary | null>(null);
  const [packs, setPacks] = useState<tauri.MatePackDefinition[]>([]);
  const [scenarios, setScenarios] = useState<tauri.FirstRunScenarioDefinition[]>([]);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const [nextLaunchpad, nextPacks, nextScenarios] = await Promise.all([
        tauri.getWorkspaceLaunchpad(workspacePath),
        tauri.listMatePackDefinitions(),
        tauri.listFirstRunScenarios(),
      ]);
      setLaunchpad(nextLaunchpad);
      setPacks(nextPacks);
      setScenarios(nextScenarios);
    } catch (nextError) {
      setError(nextError instanceof Error ? nextError.message : "Failed to load launchpad");
    } finally {
      setIsLoading(false);
    }
  }, [workspacePath]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const enabledPackSet = useMemo(
    () => new Set(launchpad?.enabledPackIds ?? []),
    [launchpad?.enabledPackIds],
  );

  const handlePresetChange = useCallback(
    async (trustPreset: "conservative" | "balanced" | "elevated") => {
      if (!launchpad) return;
      setIsSaving(true);
      setError(null);
      try {
        const next = await tauri.updateWorkspaceLaunchConfig(
          workspacePath,
          trustPreset,
          launchpad.enabledPackIds,
        );
        setLaunchpad(next);
      } catch (nextError) {
        setError(nextError instanceof Error ? nextError.message : "Failed to save trust preset");
      } finally {
        setIsSaving(false);
      }
    },
    [launchpad, workspacePath],
  );

  const handleTogglePack = useCallback(
    async (packId: string) => {
      if (!launchpad) return;
      const nextEnabled = enabledPackSet.has(packId)
        ? launchpad.enabledPackIds.filter((id) => id !== packId)
        : [...launchpad.enabledPackIds, packId];
      setIsSaving(true);
      setError(null);
      try {
        const next = await tauri.updateWorkspaceLaunchConfig(
          workspacePath,
          launchpad.trustPreset,
          nextEnabled,
        );
        setLaunchpad(next);
      } catch (nextError) {
        setError(nextError instanceof Error ? nextError.message : "Failed to update packs");
      } finally {
        setIsSaving(false);
      }
    },
    [enabledPackSet, launchpad, workspacePath],
  );

  if (isLoading) {
    return (
      <div className="flex h-full w-full items-center justify-center">
        <div className="animate-pulse text-sm font-medium text-muted-foreground tracking-wide">
          Loading workspace launchpad...
        </div>
      </div>
    );
  }

  if (!launchpad) {
    return (
      <div className="flex h-full w-full items-center justify-center p-6">
        <div className="max-w-md w-full rounded-2xl border border-red-500/20 bg-red-500/5 p-6 text-center">
          <p className="text-base font-medium text-red-500">{error || "Launchpad unavailable."}</p>
        </div>
      </div>
    );
  }

  return (
    <div className="relative h-full w-full overflow-y-auto overflow-x-hidden">
      <div className="flex flex-col gap-8">
        
        {/* Header Section */}
        <div className="flex flex-col gap-6 md:flex-row md:items-start md:justify-between">
          <div className="space-y-3">
            <div className="flex items-center gap-3">
              <BrainCircuit className="size-6 text-primary" />
              <h1 className="text-2xl font-semibold tracking-tight text-foreground">
                Workspace Launchpad
              </h1>
            </div>
            <p className="max-w-2xl text-[14px] leading-relaxed text-muted-foreground">
              Turn the current workspace into a production-ready MaTE operating surface with trust
              presets, first-party packs, and a guided first run.
            </p>
            <div className="flex flex-wrap items-center gap-2 pt-2">
              <div className="rounded-xl border border-white/10 bg-white/5 px-2.5 py-1 text-[12px]">
                 <span className="text-muted-foreground mr-1.5">Workspace:</span> <span className="font-medium text-foreground">{launchpad.workspaceName}</span>
              </div>
              <div className="rounded-xl border border-white/10 bg-white/5 px-2.5 py-1 text-[12px]">
                 <span className="text-muted-foreground mr-1.5">Path:</span> <span className="font-mono font-medium text-foreground">{workspacePath}</span>
              </div>
              <div className="rounded-xl bg-primary/10 px-2.5 py-1 text-[12px] font-medium text-primary">
                Launches: {launchpad.launchCount}
              </div>
              <div className="rounded-xl bg-green-500/10 px-2.5 py-1 text-[12px] font-medium text-green-500">
                Successful: {launchpad.successfulLaunchCount}
              </div>
            </div>
          </div>
        </div>


        {error && (
          <div className="rounded-2xl border border-red-500/20 bg-red-500/5 p-4">
            <p className="text-sm font-medium text-red-500">{error}</p>
          </div>
        )}

        {/* Trust Preset */}
        <div className="flex flex-col gap-4">
          <div className="flex items-center gap-2 px-1">
            <Shield className="size-4 text-primary" />
            <h2 className="text-[15px] font-semibold tracking-tight text-foreground">Trust Preset</h2>
          </div>
          <div className="grid gap-4 md:grid-cols-3">
            {TRUST_PRESETS.map((preset) => {
              const active = launchpad.trustPreset === preset.id;
              return (
                <button
                  key={preset.id}
                  type="button"
                  onClick={() => void handlePresetChange(preset.id)}
                  disabled={isSaving}
                  className={`group relative overflow-hidden rounded-2xl border p-5 text-left transition-all duration-200 outline-none ${
                    active
                      ? "border-primary/50 bg-primary/5"
                      : "border-white/5 bg-white/5 hover:border-white/10 hover:bg-white/10"
                  }`}
                >
                  <div className="mb-2 flex items-center justify-between">
                    <span className={`text-[14px] font-semibold tracking-tight ${active ? "text-primary" : "text-foreground"}`}>
                      {preset.title}
                    </span>
                    {active ? (
                      <CheckCircle2 className="size-4.5 text-primary" />
                    ) : (
                      <div className="size-4.5 rounded-full border border-white/20 group-hover:border-white/30 transition-colors" />
                    )}
                  </div>
                  <p className={`text-[13px] leading-relaxed ${active ? "text-primary/70" : "text-muted-foreground group-hover:text-foreground/80"}`}>
                    {preset.summary}
                  </p>
                </button>
              );
            })}
          </div>
        </div>

        {/* MaTE Packs */}
        <div className="flex flex-col gap-4">
          <div className="flex items-center gap-2 px-1">
            <FolderKanban className="size-4 text-primary" />
            <h2 className="text-[15px] font-semibold tracking-tight text-foreground">MaTE Packs</h2>
          </div>
          <div className="grid gap-4 lg:grid-cols-2">
            {packs.map((pack) => {
              const enabled = enabledPackSet.has(pack.id);
              return (
                <div
                  key={pack.id}
                  className={`flex flex-col gap-4 rounded-2xl border p-5 transition-colors ${
                    enabled ? "border-primary/20 bg-primary/5" : "border-white/5 bg-white/5"
                  }`}
                >
                  <div className="flex items-start justify-between gap-4">
                    <div className="space-y-1 flex-1">
                      <h3 className="text-[14px] font-semibold tracking-tight text-foreground">{pack.title}</h3>
                      <p className="text-[13px] leading-relaxed text-muted-foreground">{pack.summary}</p>
                    </div>
                    <Button
                      size="sm"
                      variant={enabled ? "secondary" : "ghost"}
                      onPress={() => void handleTogglePack(pack.id)}
                      isDisabled={isSaving}
                      className={`font-medium shrink-0 h-8 rounded-xl ${
                        !enabled ? "text-muted-foreground hover:bg-white/10 hover:text-foreground" : "text-primary bg-primary/10"
                      }`}
                    >
                      {enabled ? "Enabled" : "Enable"}
                    </Button>
                  </div>
                  
                  <div className="flex flex-col gap-3 pt-2 border-t border-white/5">
                    <div className="flex flex-wrap items-center gap-4">
                      <div className="flex items-center">
                        <span className="text-[10px] font-semibold text-muted-foreground/50 uppercase tracking-widest mr-2">Packs</span>
                        <div className="rounded-lg bg-white/5 border border-white/5 px-2 py-0.5 text-[11px] font-medium text-foreground/80">
                          {pack.defaultTrustPreset}
                        </div>
                      </div>
                    </div>
                    
                    <div className="flex flex-wrap items-center gap-2">
                       <span className="text-[10px] font-semibold text-muted-foreground/50 uppercase tracking-widest mr-1">Outputs</span>
                      {pack.expectedOutputs.map((output) => (
                         <div key={output} className="rounded-lg bg-white/5 border border-white/5 px-2 py-0.5 text-[11px] font-medium text-foreground/80">
                          {output}
                        </div>
                      ))}
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        </div>

        {/* Bottom Split Section */}
        <div className="grid gap-6 xl:grid-cols-[1.2fr,0.8fr] pb-8">
          <div className="flex flex-col gap-4">
            <div className="flex items-center gap-2 px-1">
              <Sparkles className="size-4 text-primary" />
              <h2 className="text-[15px] font-semibold tracking-tight text-foreground">Guided First Run</h2>
            </div>
            <div className="flex flex-col gap-4">
              {scenarios.map((scenario) => (
                <div key={scenario.id} className="rounded-2xl border border-white/5 bg-white/5 p-5 flex flex-col gap-4">
                  <div className="flex items-start justify-between gap-4">
                    <div className="space-y-1 flex-1">
                      <h3 className="text-[14px] font-semibold tracking-tight text-foreground">{scenario.title}</h3>
                      <p className="text-[13px] leading-relaxed text-muted-foreground">{scenario.summary}</p>
                    </div>
                    <Button
                      variant="secondary"
                      size="sm"
                      onPress={() => void onRunScenario(scenario.id)}
                      isDisabled={isSaving}
                      className="font-medium shrink-0 h-8 rounded-xl bg-primary/10 text-primary"
                    >
                      Run in Chat
                    </Button>
                  </div>
                  
                  <div className="flex flex-col gap-3 pt-3 border-t border-white/5">
                    {scenario.recommendedPackIds.length > 0 && (
                      <div className="flex flex-wrap items-center gap-2">
                        <span className="text-[10px] font-semibold text-muted-foreground/50 uppercase tracking-widest mr-1">Packs</span>
                        {scenario.recommendedPackIds.map((packId) => (
                          <div key={packId} className="rounded-lg bg-white/5 border border-white/5 px-2 py-0.5 text-[11px] font-medium text-foreground/80">
                            {packId}
                          </div>
                        ))}
                      </div>
                    )}
                    
                    {scenario.suggestedOutputs.length > 0 && (
                      <div className="flex flex-wrap items-center gap-2">
                         <span className="text-[10px] font-semibold text-muted-foreground/50 uppercase tracking-widest mr-1">Outputs</span>
                        {scenario.suggestedOutputs.map((output) => (
                          <div key={output} className="rounded-lg bg-white/5 border border-white/5 px-2 py-0.5 text-[11px] font-medium text-foreground/80">
                            {output}
                          </div>
                        ))}
                      </div>
                     )}
                  </div>
                </div>
              ))}
            </div>
          </div>

          <div className="flex flex-col gap-4 h-full">
            <div className="flex items-center gap-2 px-1">
              <Shield className="size-4 text-primary" />
              <h2 className="text-[15px] font-semibold tracking-tight text-foreground">Capability Summary</h2>
            </div>
            <div className="rounded-2xl border border-white/5 bg-white/5 p-5 flex flex-col gap-5 h-full">
              
              <div className="flex flex-wrap gap-2">
                <div className="rounded-lg bg-primary/10 px-2 py-1 text-[11px] font-semibold uppercase tracking-wider text-primary">
                  {launchpad.capabilitySummary.label}
                </div>
                <div className="rounded-lg bg-white/5 border border-white/5 px-2.5 py-1 text-[12px] text-foreground/80">
                  <span className="text-muted-foreground mr-1.5">Policy:</span> <span className="font-medium">{launchpad.capabilitySummary.effectiveToolPolicyMode}</span>
                </div>
                <div className="rounded-lg bg-white/5 border border-white/5 px-2.5 py-1 text-[12px] text-foreground/80">
                  <span className="text-muted-foreground mr-1.5">Paths:</span> <span className="font-medium">{launchpad.capabilitySummary.allowedPathsCount}</span>
                </div>
              </div>
              
              <div className="space-y-3 pt-3 border-t border-white/5">
                <p className="text-[10px] font-semibold tracking-widest text-muted-foreground/50 uppercase">Capabilities</p>
                <div className="flex flex-wrap gap-1.5">
                  {launchpad.capabilitySummary.enabledCapabilities.map((capability) => (
                    <div key={capability} className="rounded-lg bg-green-500/10 px-2.5 py-1 text-[11px] font-medium text-green-500">
                      {capability}
                    </div>
                  ))}
                </div>
              </div>
              
              <div className="space-y-3 pt-3 border-t border-white/5">
                <p className="text-[10px] font-semibold tracking-widest text-muted-foreground/50 uppercase">Cautions</p>
                <div className="flex flex-col gap-2">
                  {launchpad.capabilitySummary.cautions.length ? (
                    launchpad.capabilitySummary.cautions.map((caution) => (
                      <div key={caution} className="rounded-xl border border-yellow-500/20 bg-yellow-500/10 p-3 text-[12.5px] text-foreground/90 leading-relaxed">
                        {caution}
                      </div>
                    ))
                  ) : (
                    <div className="rounded-xl border border-green-500/20 bg-green-500/10 p-3 text-[12.5px] text-green-500 leading-relaxed flex items-center gap-2">
                      <CheckCircle2 className="size-4 shrink-0" />
                      No immediate launchpad cautions.
                    </div>
                  )}
                </div>
              </div>
              
              <div className="mt-auto pt-6 flex flex-col justify-end">
                {launchpad.firstRunCompletedAt ? (
                  <div className="rounded-xl border border-primary/20 bg-primary/10 p-3 text-[12.5px] leading-relaxed text-foreground">
                    First run completed with <span className="font-medium text-primary">{launchpad.firstRunScenarioId || "starter"}</span>{" "}
                    on <span className="text-muted-foreground">{new Date(launchpad.firstRunCompletedAt).toLocaleString()}</span>.
                  </div>
                ) : (
                  <div className="rounded-xl border border-dashed border-white/10 bg-transparent p-3 text-[12.5px] text-muted-foreground text-center">
                    No guided first run recorded yet.
                  </div>
                )}
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
