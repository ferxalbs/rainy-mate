import { useCallback, useEffect, useMemo, useState } from "react";
import { BrainCircuit, CheckCircle2, FolderKanban, Shield, Sparkles } from "lucide-react";

import * as tauri from "../../services/tauri";
import { Badge } from "../ui/badge";
import { Card } from "../ui/card";
import { Button } from "../ui/button";

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
      <div className="flex h-full items-center justify-center">
        <div className="text-sm text-muted-foreground">Loading workspace launchpad...</div>
      </div>
    );
  }

  if (!launchpad) {
    return (
      <div className="flex h-full items-center justify-center">
        <div className="text-sm text-red-500">{error || "Launchpad unavailable."}</div>
      </div>
    );
  }

  return (
    <div className="relative h-full w-full overflow-y-auto overflow-x-hidden bg-background">
      <div className="mx-auto flex max-w-7xl flex-col gap-8 p-6 lg:p-10">
      <div className="flex flex-col gap-3 md:flex-row md:items-end md:justify-between">
        <div className="space-y-2">
          <div className="flex items-center gap-2">
            <BrainCircuit className="size-5 text-primary" />
            <h1 className="text-2xl font-semibold tracking-tight">Workspace Launchpad</h1>
          </div>
          <p className="max-w-3xl text-sm text-muted-foreground">
            Turn the current workspace into a production-ready MaTE operating surface with trust
            presets, first-party packs, and a guided first run that lands in the main chat runtime.
          </p>
          <div className="flex flex-wrap gap-2">
            <Badge variant="outline">Workspace: {launchpad.workspaceName}</Badge>
            <Badge variant="outline">Path: {workspacePath}</Badge>
            <Badge variant="outline">Launches: {launchpad.launchCount}</Badge>
            <Badge variant="outline">Successful: {launchpad.successfulLaunchCount}</Badge>
          </div>
        </div>
        <Button variant="outline" onClick={() => void refresh()} disabled={isSaving}>
          Refresh
        </Button>
      </div>

      {error && (
        <Card className="border-red-500/30 bg-red-500/5 p-4 text-sm text-red-600 dark:text-red-300">
          {error}
        </Card>
      )}

      <Card className="p-5">
        <div className="mb-4 flex items-center gap-2">
          <Shield className="size-4 text-primary" />
          <h2 className="text-lg font-medium">Trust Preset</h2>
        </div>
        <div className="grid gap-3 md:grid-cols-3">
          {TRUST_PRESETS.map((preset) => {
            const active = launchpad.trustPreset === preset.id;
            return (
              <button
                key={preset.id}
                type="button"
                onClick={() => void handlePresetChange(preset.id)}
                disabled={isSaving}
                className={`rounded-2xl border p-4 text-left transition-colors ${
                  active
                    ? "border-primary/50 bg-primary/10"
                    : "border-border/60 bg-background/40 hover:bg-accent/40"
                }`}
              >
                <div className="mb-2 flex items-center justify-between">
                  <span className="font-medium">{preset.title}</span>
                  {active ? <CheckCircle2 className="size-4 text-primary" /> : null}
                </div>
                <p className="text-sm text-muted-foreground">{preset.summary}</p>
              </button>
            );
          })}
        </div>
      </Card>

      <Card className="p-5">
        <div className="mb-4 flex items-center gap-2">
          <FolderKanban className="size-4 text-primary" />
          <h2 className="text-lg font-medium">MaTE Packs</h2>
        </div>
        <div className="grid gap-3 lg:grid-cols-2">
          {packs.map((pack) => {
            const enabled = enabledPackSet.has(pack.id);
            return (
              <div
                key={pack.id}
                className={`rounded-2xl border p-4 ${
                  enabled ? "border-primary/40 bg-primary/5" : "border-border/60"
                }`}
              >
                <div className="mb-3 flex items-start justify-between gap-3">
                  <div>
                    <h3 className="font-medium">{pack.title}</h3>
                    <p className="mt-1 text-sm text-muted-foreground">{pack.summary}</p>
                  </div>
                  <Button
                    size="sm"
                    variant={enabled ? "default" : "outline"}
                    onClick={() => void handleTogglePack(pack.id)}
                    disabled={isSaving}
                  >
                    {enabled ? "Enabled" : "Enable"}
                  </Button>
                </div>
                <div className="mb-3 flex flex-wrap gap-2">
                  <Badge variant="outline">Best for: {pack.recommendedFor}</Badge>
                  <Badge variant="outline">Default preset: {pack.defaultTrustPreset}</Badge>
                </div>
                <div className="flex flex-wrap gap-2">
                  {pack.expectedOutputs.map((output) => (
                    <Badge key={output} variant="secondary">
                      {output}
                    </Badge>
                  ))}
                </div>
              </div>
            );
          })}
        </div>
      </Card>

      <div className="grid gap-6 xl:grid-cols-[1.2fr,0.8fr]">
        <Card className="p-5">
          <div className="mb-4 flex items-center gap-2">
            <Sparkles className="size-4 text-primary" />
            <h2 className="text-lg font-medium">Guided First Run</h2>
          </div>
          <div className="grid gap-3">
            {scenarios.map((scenario) => (
              <div key={scenario.id} className="rounded-2xl border border-border/60 p-4">
                <div className="mb-2 flex items-start justify-between gap-3">
                  <div>
                    <h3 className="font-medium">{scenario.title}</h3>
                    <p className="mt-1 text-sm text-muted-foreground">{scenario.summary}</p>
                  </div>
                  <Button onClick={() => void onRunScenario(scenario.id)} disabled={isSaving}>
                    Run in Chat
                  </Button>
                </div>
                <div className="mb-2 flex flex-wrap gap-2">
                  {scenario.recommendedPackIds.map((packId) => (
                    <Badge key={packId} variant="outline">
                      {packId}
                    </Badge>
                  ))}
                </div>
                <div className="flex flex-wrap gap-2">
                  {scenario.suggestedOutputs.map((output) => (
                    <Badge key={output} variant="secondary">
                      {output}
                    </Badge>
                  ))}
                </div>
              </div>
            ))}
          </div>
        </Card>

        <Card className="p-5">
          <div className="mb-4 flex items-center gap-2">
            <Shield className="size-4 text-primary" />
            <h2 className="text-lg font-medium">Capability Summary</h2>
          </div>
          <div className="space-y-4 text-sm">
            <div className="flex flex-wrap gap-2">
              <Badge variant="outline">{launchpad.capabilitySummary.label}</Badge>
              <Badge variant="outline">
                Tool policy: {launchpad.capabilitySummary.effectiveToolPolicyMode}
              </Badge>
              <Badge variant="outline">
                Allowed paths: {launchpad.capabilitySummary.allowedPathsCount}
              </Badge>
            </div>
            <div>
              <p className="mb-2 font-medium">Enabled</p>
              <div className="flex flex-wrap gap-2">
                {launchpad.capabilitySummary.enabledCapabilities.map((capability) => (
                  <Badge key={capability} variant="secondary">
                    {capability}
                  </Badge>
                ))}
              </div>
            </div>
            <div>
              <p className="mb-2 font-medium">Cautions</p>
              <div className="flex flex-col gap-2">
                {launchpad.capabilitySummary.cautions.length ? (
                  launchpad.capabilitySummary.cautions.map((caution) => (
                    <div key={caution} className="rounded-xl border border-amber-500/20 bg-amber-500/5 p-3 text-muted-foreground">
                      {caution}
                    </div>
                  ))
                ) : (
                  <div className="rounded-xl border border-emerald-500/20 bg-emerald-500/5 p-3 text-muted-foreground">
                    This workspace has no immediate launchpad cautions.
                  </div>
                )}
              </div>
            </div>
            {launchpad.firstRunCompletedAt ? (
              <div className="rounded-xl border border-emerald-500/20 bg-emerald-500/5 p-3">
                First run completed with <strong>{launchpad.firstRunScenarioId || "starter"}</strong>{" "}
                on {new Date(launchpad.firstRunCompletedAt).toLocaleString()}.
              </div>
            ) : (
              <div className="rounded-xl border border-border/60 p-3 text-muted-foreground">
                No guided first run has been recorded yet.
              </div>
            )}
          </div>
        </Card>
      </div>
    </div>
    </div>
  );
}
