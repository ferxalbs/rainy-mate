import { Button } from "@heroui/react";
import { AlertTriangle, RefreshCw } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import {
  AtmCommandMetricsResponse,
  AtmCommandProgressEvent,
  AtmCommandSummary,
  getAtmCommandDetails,
  getAtmCommandMetrics,
  getAtmCommandProgress,
  listAtmCommands,
} from "../../../services/tauri";
import { NeuralChip } from "../shared/UiElements";

export function NeuralActivity() {
  const [recentCommands, setRecentCommands] = useState<AtmCommandSummary[]>([]);
  const [selectedCommandId, setSelectedCommandId] = useState<string | null>(
    null,
  );
  const [commandProgress, setCommandProgress] = useState<
    AtmCommandProgressEvent[]
  >([]);
  const [commandMetrics, setCommandMetrics] =
    useState<AtmCommandMetricsResponse | null>(null);
  const [isLoadingCommands, setIsLoadingCommands] = useState(false);
  const [isLoadingProgress, setIsLoadingProgress] = useState(false);

  const progressSinceRef = useRef(0);

  const formatMs = (value?: number | null) =>
    typeof value === "number" ? `${value}ms` : "-";

  const refreshRecentCommands = useCallback(async () => {
    setIsLoadingCommands(true);
    try {
      const commands = await listAtmCommands(25);
      setRecentCommands(commands);

      if (commands.length === 0) {
        setSelectedCommandId(null);
        setCommandProgress([]);
        setCommandMetrics(null);
        progressSinceRef.current = 0;
        return;
      }

      if (
        !selectedCommandId ||
        !commands.some((c) => c.id === selectedCommandId)
      ) {
        const active =
          commands.find(
            (c) => c.status === "running" || c.status === "pending",
          ) || commands[0];
        setSelectedCommandId(active.id);
      }
    } catch (err) {
      console.error("Failed to load recent commands:", err);
    } finally {
      setIsLoadingCommands(false);
    }
  }, [selectedCommandId]);

  useEffect(() => {
    refreshRecentCommands();
    const interval = setInterval(refreshRecentCommands, 5000);
    return () => clearInterval(interval);
  }, [refreshRecentCommands]);

  useEffect(() => {
    if (!selectedCommandId) return;
    let cancelled = false;

    const loadInitial = async () => {
      setIsLoadingProgress(true);

      try {
        const [details, metrics] = await Promise.all([
          getAtmCommandDetails(selectedCommandId, 200),
          getAtmCommandMetrics(selectedCommandId),
        ]);
        if (cancelled) return;
        setCommandProgress(details.progress);
        setCommandMetrics(metrics);
        const last = details.progress[details.progress.length - 1];
        progressSinceRef.current = last?.createdAt || 0;
      } catch (err) {
        console.error("Failed to load command details:", err);
      } finally {
        if (!cancelled) setIsLoadingProgress(false);
      }
    };

    const pollProgress = async () => {
      try {
        const delta = await getAtmCommandProgress(
          selectedCommandId,
          progressSinceRef.current,
          200,
        );
        if (cancelled || delta.progress.length === 0) return;

        setCommandProgress((prev) => {
          const seen = new Set(prev.map((p) => p.id));
          const merged = [...prev];
          for (const event of delta.progress) {
            if (!seen.has(event.id)) {
              merged.push(event);
            }
          }
          return merged;
        });
        progressSinceRef.current = delta.nextSince;
      } catch (err) {
        console.error("Failed to poll command progress:", err);
      }
    };

    const pollMetrics = async () => {
      try {
        const metrics = await getAtmCommandMetrics(selectedCommandId);
        if (cancelled) return;
        setCommandMetrics(metrics);
      } catch (err) {
        console.error("Failed to poll command metrics:", err);
      }
    };

    loadInitial();
    const progressInterval = setInterval(pollProgress, 2000);
    const metricsInterval = setInterval(pollMetrics, 5000);
    return () => {
      cancelled = true;
      clearInterval(progressInterval);
      clearInterval(metricsInterval);
    };
  }, [selectedCommandId]);

  return (
    <div className="space-y-8 animate-appear">
      <div className="flex items-center justify-between border-b border-border/10 pb-6">
        <div>
          <h3 className="text-2xl font-bold text-foreground tracking-tight">
            Activity Stream
          </h3>
          <p className="text-muted-foreground text-sm">
            Real-time command execution and telemetry.
          </p>
        </div>
        <Button
          size="sm"
          onPress={refreshRecentCommands}
          isDisabled={isLoadingCommands}
          className="bg-transparent text-muted-foreground hover:text-foreground hover:bg-white/5"
        >
          <RefreshCw
            className={`size-4 mr-2 ${isLoadingCommands ? "animate-spin" : ""}`}
          />
          Refresh
        </Button>
      </div>

      {recentCommands.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-20 rounded-2xl border border-dashed border-border/20 text-muted-foreground/50">
          <span className="text-sm font-medium">No commands executed yet</span>
        </div>
      ) : (
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <div className="space-y-4">
            <h4 className="text-xs font-bold uppercase tracking-widest text-muted-foreground">
              Command History
            </h4>
            <div className="rounded-xl border border-border/20 bg-card/20 backdrop-blur-md overflow-hidden">
              <div className="max-h-[600px] overflow-y-auto p-2 space-y-1 custom-scrollbar">
                {recentCommands.map((cmd) => (
                  <button
                    key={cmd.id}
                    type="button"
                    onClick={() => setSelectedCommandId(cmd.id)}
                    className={`w-full text-left p-3 rounded-lg border transition-all ${
                      selectedCommandId === cmd.id
                        ? "border-primary/40 bg-primary/10"
                        : "border-transparent hover:bg-white/5 hover:border-white/5"
                    }`}
                  >
                    <div className="flex items-center justify-between gap-2">
                      <span className="text-sm font-medium text-foreground truncate">
                        {cmd.intent}
                      </span>
                      <NeuralChip
                        variant="flat"
                        color={
                          cmd.status === "running"
                            ? "info"
                            : cmd.status === "completed"
                              ? "success"
                              : "default"
                        }
                      >
                        {cmd.status}
                      </NeuralChip>
                    </div>
                    <div className="text-[10px] text-muted-foreground mt-1 font-mono">
                      {cmd.id}
                    </div>
                  </button>
                ))}
              </div>
            </div>
          </div>

          <div className="space-y-4">
            <h4 className="text-xs font-bold uppercase tracking-widest text-muted-foreground">
              Execution Details
            </h4>
            <div className="rounded-xl border border-border/20 bg-card/20 backdrop-blur-md p-4 min-h-[400px]">
              {selectedCommandId ? (
                <div className="space-y-6">
                  {commandMetrics && (
                    <div className="p-4 rounded-xl bg-card/30 border border-white/5 space-y-3">
                      <div className="flex items-center justify-between gap-2">
                        <span className="text-xs uppercase tracking-wider text-muted-foreground">
                          Metrics
                        </span>
                        <NeuralChip variant="flat" className="bg-white/10">
                          {commandMetrics.status}
                        </NeuralChip>
                      </div>

                      {(commandMetrics.progress.droppedEventsTotal > 0 ||
                        commandMetrics.progress.suppressedEventsTotal > 0) && (
                        <div className="flex items-center gap-2 text-xs text-orange-400 bg-orange-500/10 p-2 rounded-lg border border-orange-500/20">
                          <AlertTriangle className="size-3.5" />
                          <span>Backpressure detected</span>
                        </div>
                      )}

                      <div className="grid grid-cols-3 gap-4 text-xs font-mono text-muted-foreground">
                        <div>
                          <div className="text-[10px] opacity-50 uppercase">
                            Queue
                          </div>
                          {formatMs(commandMetrics.timings.queueDelayMs)}
                        </div>
                        <div>
                          <div className="text-[10px] opacity-50 uppercase">
                            Run
                          </div>
                          {formatMs(commandMetrics.timings.runDurationMs)}
                        </div>
                        <div>
                          <div className="text-[10px] opacity-50 uppercase">
                            Total
                          </div>
                          {formatMs(commandMetrics.timings.totalDurationMs)}
                        </div>
                      </div>
                    </div>
                  )}

                  <div className="space-y-2">
                    <h5 className="text-[10px] uppercase tracking-wider text-muted-foreground">
                      Live Logs
                    </h5>
                    <div className="space-y-2 font-mono">
                      {isLoadingProgress && commandProgress.length === 0 ? (
                        <div className="text-sm text-muted-foreground animate-pulse">
                          Loading progress...
                        </div>
                      ) : commandProgress.length === 0 ? (
                        <div className="text-sm text-muted-foreground">
                          No events recorded.
                        </div>
                      ) : (
                        commandProgress.map((event) => (
                          <div
                            key={event.id}
                            className="p-3 rounded-lg bg-black/20 border border-white/5"
                          >
                            <div className="flex items-center justify-between gap-2 mb-1">
                              <span
                                className={`text-[10px] uppercase font-bold ${
                                  event.level === "error"
                                    ? "text-red-400"
                                    : event.level === "warn"
                                      ? "text-orange-400"
                                      : "text-blue-400"
                                }`}
                              >
                                {event.level}
                              </span>
                              <span className="text-[10px] text-muted-foreground opacity-50">
                                {new Date(event.createdAt).toLocaleTimeString()}
                              </span>
                            </div>
                            <p className="text-sm text-foreground/90 break-words">
                              {event.message}
                            </p>
                            {event.data && (
                              <div className="mt-2 p-2 rounded bg-black/30 overflow-x-auto">
                                <pre className="text-[10px] text-muted-foreground">
                                  {JSON.stringify(event.data, null, 2)}
                                </pre>
                              </div>
                            )}
                          </div>
                        ))
                      )}
                    </div>
                  </div>
                </div>
              ) : (
                <div className="h-full flex items-center justify-center text-muted-foreground text-sm">
                  Select a command to view details.
                </div>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
