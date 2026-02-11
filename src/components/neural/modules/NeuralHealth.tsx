import { Button } from "@heroui/react";
import { AlertTriangle, RefreshCw } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import {
  AtmEndpointMetricsResponse,
  AtmMetricsAlert,
  AtmMetricsAlertRetentionConfig,
  AtmMetricsSloConfig,
  ackAtmMetricsAlert,
  cleanupAtmMetricsAlerts,
  getAtmAdminPermissions,
  getAtmEndpointMetrics,
  getAtmMetricsAlertRetention,
  getAtmMetricsSlo,
  listAtmMetricsAlerts,
  syncAtmMetricsAlerts,
  updateAtmMetricsAlertRetention,
  updateAtmMetricsSlo,
} from "../../../services/tauri";
import { NeuralChip } from "../shared/UiElements";

type AlertSeverity = "normal" | "warn" | "critical";
type AlertHistoryStatus = "open" | "acked" | "resolved" | "all";
type EndpointAlert = {
  alertKey: string;
  key: string;
  label: string;
  severity: Exclude<AlertSeverity, "normal">;
  reason: string;
};

const DEFAULT_SLO_THRESHOLDS: AtmMetricsSloConfig = {
  endpointErrorRateWarn: 5,
  endpointErrorRateCritical: 20,
  endpointP95WarnMs: 4000,
  endpointP95CriticalMs: 10000,
  endpointSloErrorRateTarget: 2,
  endpointSloP95TargetMs: 2500,
  endpointRegressionErrorRateFactor: 1.5,
  endpointRegressionErrorRateDelta: 2,
  endpointRegressionP95Factor: 1.5,
  endpointRegressionP95DeltaMs: 1000,
  failureTimeoutWarn: 3,
  failureTimeoutCritical: 10,
  failureRuntimeWarn: 5,
  failureRuntimeCritical: 15,
  failureTransportWarn: 2,
  failureTransportCritical: 6,
};

const DEFAULT_ALERT_RETENTION: AtmMetricsAlertRetentionConfig = {
  days: 14,
};

// Styles for native inputs to match design
const inputClass =
  "w-full bg-card/40 hover:bg-card/60 backdrop-blur-md rounded-xl px-3 py-2 text-xs text-foreground placeholder:text-muted-foreground/50 leading-relaxed border border-border/20 focus:outline-none focus:border-primary/50 focus:ring-1 focus:ring-primary/20 transition-all shadow-sm";
const labelClass =
  "block text-muted-foreground text-[10px] font-bold uppercase tracking-widest mb-1.5 ml-1";

export function NeuralHealth() {
  const [endpointMetrics, setEndpointMetrics] =
    useState<AtmEndpointMetricsResponse | null>(null);
  const [endpointAlerts, setEndpointAlerts] = useState<EndpointAlert[]>([]);
  const [persistedAlerts, setPersistedAlerts] = useState<AtmMetricsAlert[]>([]);
  const [alertHistoryStatus, setAlertHistoryStatus] =
    useState<AlertHistoryStatus>("open");
  const [alertRetention, setAlertRetention] =
    useState<AtmMetricsAlertRetentionConfig>(DEFAULT_ALERT_RETENTION);
  const [sloThresholds, setSloThresholds] = useState<AtmMetricsSloConfig>(
    DEFAULT_SLO_THRESHOLDS,
  );
  const [canEditSlo, setCanEditSlo] = useState(false);
  const [canAckAlerts, setCanAckAlerts] = useState(false);
  const [canEditRetention, setCanEditRetention] = useState(false);
  const [canRunCleanup, setCanRunCleanup] = useState(false);

  const [isLoadingMetrics, setIsLoadingMetrics] = useState(false);
  const [isSavingSlo, setIsSavingSlo] = useState(false);
  const [isSavingRetention, setIsSavingRetention] = useState(false);
  const [isCleaningAlerts, setIsCleaningAlerts] = useState(false);

  const endpointSnapshotRef = useRef<
    Record<string, { errorRate: number | null; p95: number | null }>
  >({});

  const formatMs = (value?: number | null) =>
    typeof value === "number" ? `${value}ms` : "-";
  const formatPct = (value?: number | null) =>
    typeof value === "number" ? `${value.toFixed(1)}%` : "-";
  const formatRate = (value?: number | null) =>
    typeof value === "number" ? `${value.toFixed(2)}/s` : "-";

  const thresholdNumber = (
    key: keyof AtmMetricsSloConfig,
    fallback: number,
  ): number => {
    const value = sloThresholds[key];
    return typeof value === "number" && Number.isFinite(value)
      ? value
      : fallback;
  };

  const classifyEndpointSeverity = (
    errorRate?: number | null,
    p95Ms?: number | null,
  ): AlertSeverity => {
    const errorCritical = thresholdNumber("endpointErrorRateCritical", 20);
    const p95Critical = thresholdNumber("endpointP95CriticalMs", 10000);
    const errorWarn = thresholdNumber("endpointErrorRateWarn", 5);
    const p95Warn = thresholdNumber("endpointP95WarnMs", 4000);

    if (
      (typeof errorRate === "number" && errorRate >= errorCritical) ||
      (typeof p95Ms === "number" && p95Ms >= p95Critical)
    ) {
      return "critical";
    }
    if (
      (typeof errorRate === "number" && errorRate >= errorWarn) ||
      (typeof p95Ms === "number" && p95Ms >= p95Warn)
    ) {
      return "warn";
    }
    return "normal";
  };

  const buildEndpointAlerts = (
    metrics: AtmEndpointMetricsResponse,
  ): EndpointAlert[] => {
    const alerts: EndpointAlert[] = [];
    for (const endpoint of metrics.endpoints) {
      const p95 =
        endpoint.latency.p95TotalMs ?? endpoint.latency.p95RunMs ?? null;
      const errorRate = endpoint.errorRate ?? null;

      const pushAlert = (
        alertKey: string,
        severity: Exclude<AlertSeverity, "normal">,
        reason: string,
      ) => {
        alerts.push({
          alertKey,
          key: endpoint.key,
          label: endpoint.label,
          severity,
          reason,
        });
      };

      const sloErrorRateTarget = thresholdNumber(
        "endpointSloErrorRateTarget",
        2,
      );
      const sloP95Target = thresholdNumber("endpointSloP95TargetMs", 2500);
      const criticalErrorRate = thresholdNumber(
        "endpointErrorRateCritical",
        20,
      );
      const criticalP95 = thresholdNumber("endpointP95CriticalMs", 10000);
      const regressionErrorFactor = thresholdNumber(
        "endpointRegressionErrorRateFactor",
        1.5,
      );
      const regressionErrorDelta = thresholdNumber(
        "endpointRegressionErrorRateDelta",
        2,
      );
      const regressionP95Factor = thresholdNumber(
        "endpointRegressionP95Factor",
        1.5,
      );
      const regressionP95Delta = thresholdNumber(
        "endpointRegressionP95DeltaMs",
        1000,
      );

      if (typeof errorRate === "number" && errorRate > sloErrorRateTarget) {
        pushAlert(
          `${endpoint.key}:slo_error_rate`,
          errorRate >= criticalErrorRate ? "critical" : "warn",
          `error rate ${errorRate.toFixed(1)}% > SLO ${sloErrorRateTarget}%`,
        );
      }
      if (typeof p95 === "number" && p95 > sloP95Target) {
        pushAlert(
          `${endpoint.key}:slo_p95`,
          p95 >= criticalP95 ? "critical" : "warn",
          `p95 ${Math.round(p95)}ms > SLO ${sloP95Target}ms`,
        );
      }

      const previous = endpointSnapshotRef.current[endpoint.key];
      if (previous) {
        if (
          typeof errorRate === "number" &&
          typeof previous.errorRate === "number" &&
          errorRate >= previous.errorRate * regressionErrorFactor &&
          errorRate - previous.errorRate >= regressionErrorDelta
        ) {
          pushAlert(
            `${endpoint.key}:regression_error_rate`,
            "warn",
            `error rate regressed from ${previous.errorRate.toFixed(1)}%`,
          );
        }
        if (
          typeof p95 === "number" &&
          typeof previous.p95 === "number" &&
          p95 >= previous.p95 * regressionP95Factor &&
          p95 - previous.p95 >= regressionP95Delta
        ) {
          pushAlert(
            `${endpoint.key}:regression_p95`,
            "warn",
            `p95 regressed from ${Math.round(previous.p95)}ms`,
          );
        }
      }

      endpointSnapshotRef.current[endpoint.key] = { errorRate, p95 };
    }
    return alerts;
  };

  const refreshMetrics = useCallback(async () => {
    setIsLoadingMetrics(true);
    try {
      const metrics = await getAtmEndpointMetrics(60 * 60 * 1000, 2000);
      setEndpointMetrics(metrics);
      const generatedAlerts = buildEndpointAlerts(metrics);
      setEndpointAlerts(generatedAlerts);

      try {
        await syncAtmMetricsAlerts(
          generatedAlerts.map((alert) => ({
            source: "endpoint_metrics",
            key: alert.alertKey,
            severity: alert.severity,
            reason: alert.reason,
            metadata: {
              endpointKey: alert.key,
              endpointLabel: alert.label,
            },
          })),
        );
      } catch (err) {
        console.error("Failed to sync metrics alerts:", err);
      }
    } catch (err) {
      console.error("Failed to load metrics:", err);
    } finally {
      setIsLoadingMetrics(false);
    }
  }, []);

  const refreshPersistedAlerts = useCallback(async () => {
    try {
      const statusParam =
        alertHistoryStatus === "all" ? undefined : alertHistoryStatus;
      const alerts = await listAtmMetricsAlerts(statusParam, 50);
      setPersistedAlerts(alerts);
    } catch (err) {
      console.error("Failed to load persisted alerts:", err);
    }
  }, [alertHistoryStatus]);

  useEffect(() => {
    const init = async () => {
      try {
        const [perms, slo, retention] = await Promise.all([
          getAtmAdminPermissions(),
          getAtmMetricsSlo(),
          getAtmMetricsAlertRetention(),
        ]);
        setCanEditSlo(perms.canEditSlo);
        setCanAckAlerts(perms.canAckAlerts);
        setCanEditRetention(perms.canEditAlertRetention);
        setCanRunCleanup(perms.canRunAlertCleanup);
        setSloThresholds(slo);
        setAlertRetention(retention);
      } catch (err) {
        console.warn("Failed to load initial health config:", err);
      }
    };
    init();
  }, []);

  useEffect(() => {
    refreshMetrics();
    const interval = setInterval(refreshMetrics, 5000);
    return () => clearInterval(interval);
  }, [refreshMetrics]);

  useEffect(() => {
    refreshPersistedAlerts();
  }, [refreshPersistedAlerts]);

  const handleSloChange = (key: keyof AtmMetricsSloConfig, value: string) => {
    const parsed = Number(value);
    setSloThresholds((prev) => ({
      ...prev,
      [key]: Number.isFinite(parsed) ? parsed : prev[key],
    }));
  };

  const handleSaveSlo = async () => {
    if (!canEditSlo) return toast.error("Permission denied");
    setIsSavingSlo(true);
    try {
      const updated = await updateAtmMetricsSlo(sloThresholds);
      setSloThresholds(updated);
      toast.success("SLO configuration saved");
    } catch (err) {
      toast.error("Failed to save SLO configuration");
    } finally {
      setIsSavingSlo(false);
    }
  };

  const handleSaveRetention = async () => {
    if (!canEditRetention) return toast.error("Permission denied");
    setIsSavingRetention(true);
    try {
      const updated = await updateAtmMetricsAlertRetention(alertRetention);
      setAlertRetention(updated);
      toast.success("Retention policy saved");
    } catch (err) {
      toast.error("Failed to save retention policy");
    } finally {
      setIsSavingRetention(false);
    }
  };

  const handleCleanup = async () => {
    if (!canRunCleanup) return toast.error("Permission denied");
    setIsCleaningAlerts(true);
    try {
      const res = await cleanupAtmMetricsAlerts();
      toast.success(`Cleanup complete. Deleted ${res.deleted} alerts.`);
      refreshPersistedAlerts();
    } catch (err) {
      toast.error("Failed to cleanup alerts");
    } finally {
      setIsCleaningAlerts(false);
    }
  };

  const handleAck = async (id: string) => {
    if (!canAckAlerts) return toast.error("Permission denied");
    try {
      await ackAtmMetricsAlert(id, "desktop-admin");
      refreshPersistedAlerts();
      toast.success("Alert acknowledged");
    } catch (err) {
      toast.error("Failed to acknowledge alert");
    }
  };

  return (
    <div className="space-y-8 animate-appear">
      <div className="flex items-center justify-between border-b border-border/10 pb-6">
        <div>
          <h3 className="text-2xl font-bold text-foreground tracking-tight">
            System Health
          </h3>
          <p className="text-muted-foreground text-sm">
            Service Level Objectives and endpoint metrics.
          </p>
        </div>
        <Button
          size="sm"
          onPress={refreshMetrics}
          isDisabled={isLoadingMetrics}
          className="bg-transparent border border-foreground/20 text-foreground hover:bg-foreground/5"
        >
          <RefreshCw
            className={`size-4 mr-2 ${isLoadingMetrics ? "animate-spin" : ""}`}
          />
          Refresh
        </Button>
      </div>

      {endpointMetrics && endpointMetrics.endpoints.length > 0 && (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {endpointMetrics.endpoints.map((metric) => {
            const p95 = metric.latency.p95TotalMs ?? metric.latency.p95RunMs;
            const avg = metric.latency.avgTotalMs ?? metric.latency.avgRunMs;
            const severity = classifyEndpointSeverity(metric.errorRate, p95);
            return (
              <div
                key={metric.key}
                className={`rounded-xl border backdrop-blur-md p-4 space-y-3 ${
                  severity === "critical"
                    ? "border-red-500/30 bg-red-500/10"
                    : severity === "warn"
                      ? "border-orange-500/30 bg-orange-500/10"
                      : "border-border/20 bg-card/20"
                }`}
              >
                <div className="flex items-center justify-between gap-2">
                  <div className="text-xs uppercase tracking-wider text-foreground font-bold">
                    {metric.label}
                  </div>
                  {severity !== "normal" && (
                    <NeuralChip
                      variant="flat"
                      color={severity === "critical" ? "danger" : "warning"}
                    >
                      {severity}
                    </NeuralChip>
                  )}
                </div>
                <div className="grid grid-cols-2 gap-2 text-xs font-mono text-muted-foreground">
                  <div>req: {metric.requests}</div>
                  <div>rate: {formatRate(metric.ratePerSecond)}</div>
                  <div>ok: {formatPct(metric.successRate)}</div>
                  <div>err: {formatPct(metric.errorRate)}</div>
                  <div>p95: {formatMs(p95)}</div>
                  <div>avg: {formatMs(avg)}</div>
                </div>
              </div>
            );
          })}
        </div>
      )}

      {endpointAlerts.length > 0 && (
        <div className="p-4 rounded-xl border border-orange-500/30 bg-orange-500/10 backdrop-blur-md space-y-3">
          <div className="flex items-center gap-2 text-sm text-orange-300 font-medium">
            <AlertTriangle className="size-4" />
            Active SLO Breaches
          </div>
          <div className="flex flex-wrap gap-2">
            {endpointAlerts.map((alert, i) => (
              <NeuralChip
                key={`${alert.alertKey}-${i}`}
                variant="flat"
                color={alert.severity === "critical" ? "danger" : "warning"}
              >
                {alert.label}: {alert.reason}
              </NeuralChip>
            ))}
          </div>
        </div>
      )}

      <div className="rounded-xl border border-border/20 bg-card/10 p-6 space-y-6">
        <div className="flex items-center justify-between">
          <h4 className="text-sm font-bold uppercase tracking-widest text-muted-foreground">
            Alert History
          </h4>
          <div className="flex items-center gap-2">
            {(["open", "acked", "resolved", "all"] as const).map((status) => (
              <Button
                key={status}
                size="sm"
                className={`capitalize text-xs font-medium ${
                  alertHistoryStatus === status
                    ? "bg-primary text-primary-foreground"
                    : "bg-transparent text-muted-foreground hover:text-foreground"
                }`}
                onPress={() => setAlertHistoryStatus(status)}
              >
                {status}
              </Button>
            ))}
          </div>
        </div>

        {persistedAlerts.length === 0 ? (
          <div className="text-sm text-muted-foreground text-center py-8 border border-dashed border-border/20 rounded-xl">
            No alerts found.
          </div>
        ) : (
          <div className="space-y-2">
            {persistedAlerts.map((alert) => (
              <div
                key={alert.id}
                className="flex items-center justify-between gap-4 p-3 rounded-lg border border-white/5 bg-background/20"
              >
                <div className="min-w-0">
                  <div className="text-sm font-medium text-foreground">
                    {alert.reason}
                  </div>
                  <div className="text-xs text-muted-foreground font-mono mt-0.5">
                    {alert.source}:{alert.key} •{" "}
                    {new Date(alert.firstSeenAt).toLocaleString()}
                  </div>
                  {alert.ackedBy && (
                    <div className="text-[10px] text-emerald-500/80 mt-1">
                      Acked by {alert.ackedBy}
                    </div>
                  )}
                </div>
                {alert.status === "open" && canAckAlerts && (
                  <Button
                    size="sm"
                    className="text-xs bg-foreground/10 text-foreground hover:bg-foreground/20"
                    onPress={() => handleAck(alert.id)}
                  >
                    Acknowledge
                  </Button>
                )}
                {alert.status !== "open" && (
                  <NeuralChip
                    variant="flat"
                    className="capitalize"
                    color={
                      alert.status === "resolved"
                        ? "success"
                        : alert.status === "acked"
                          ? "info"
                          : "default"
                    }
                  >
                    {alert.status}
                  </NeuralChip>
                )}
              </div>
            ))}
          </div>
        )}

        <div className="flex items-end gap-4 pt-4 border-t border-border/10">
          <div className="w-32">
            <label className={labelClass}>Retention (Days)</label>
            <input
              type="number"
              className={inputClass}
              value={String(alertRetention.days)}
              onChange={(e) =>
                setAlertRetention({
                  days: Math.max(1, parseInt(e.target.value) || 1),
                })
              }
              disabled={!canEditRetention}
            />
          </div>
          <Button
            size="sm"
            onPress={handleSaveRetention}
            isDisabled={!canEditRetention || isSavingRetention}
            className="bg-primary text-primary-foreground hover:bg-primary/90"
          >
            Save Policy
          </Button>
          <div className="flex-1" />
          <Button
            size="sm"
            onPress={handleCleanup}
            isDisabled={!canRunCleanup || isCleaningAlerts}
            className="bg-red-500/10 text-red-500 hover:bg-red-500/20"
          >
            Run Cleanup Task
          </Button>
        </div>
      </div>

      <div className="rounded-xl border border-border/20 bg-card/10 p-6 space-y-6">
        <div className="flex items-center justify-between">
          <h4 className="text-sm font-bold uppercase tracking-widest text-muted-foreground">
            Service Level Objectives
          </h4>
          <Button
            size="sm"
            onPress={handleSaveSlo}
            isDisabled={!canEditSlo || isSavingSlo}
            className="bg-primary text-primary-foreground hover:bg-primary/90"
          >
            Update Thresholds
          </Button>
        </div>

        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          <div>
            <label className={labelClass}>Error Warn %</label>
            <input
              type="number"
              className={inputClass}
              value={String(sloThresholds.endpointErrorRateWarn)}
              onChange={(e) =>
                handleSloChange("endpointErrorRateWarn", e.target.value)
              }
              disabled={!canEditSlo}
            />
          </div>
          <div>
            <label className={labelClass}>Error Critical %</label>
            <input
              type="number"
              className={inputClass}
              value={String(sloThresholds.endpointErrorRateCritical)}
              onChange={(e) =>
                handleSloChange("endpointErrorRateCritical", e.target.value)
              }
              disabled={!canEditSlo}
            />
          </div>
          <div>
            <label className={labelClass}>P95 Warn ms</label>
            <input
              type="number"
              className={inputClass}
              value={String(sloThresholds.endpointP95WarnMs)}
              onChange={(e) =>
                handleSloChange("endpointP95WarnMs", e.target.value)
              }
              disabled={!canEditSlo}
            />
          </div>
          <div>
            <label className={labelClass}>P95 Critical ms</label>
            <input
              type="number"
              className={inputClass}
              value={String(sloThresholds.endpointP95CriticalMs)}
              onChange={(e) =>
                handleSloChange("endpointP95CriticalMs", e.target.value)
              }
              disabled={!canEditSlo}
            />
          </div>
          <div>
            <label className={labelClass}>Target Error %</label>
            <input
              type="number"
              className={inputClass}
              value={String(sloThresholds.endpointSloErrorRateTarget)}
              onChange={(e) =>
                handleSloChange("endpointSloErrorRateTarget", e.target.value)
              }
              disabled={!canEditSlo}
            />
          </div>
          <div>
            <label className={labelClass}>Target P95 ms</label>
            <input
              type="number"
              className={inputClass}
              value={String(sloThresholds.endpointSloP95TargetMs)}
              onChange={(e) =>
                handleSloChange("endpointSloP95TargetMs", e.target.value)
              }
              disabled={!canEditSlo}
            />
          </div>
        </div>
      </div>
    </div>
  );
}
