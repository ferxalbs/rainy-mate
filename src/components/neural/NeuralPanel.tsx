import {
  Button,
  Chip,
  Separator,
  Switch,
  Modal,
  TextField,
  Input,
  Label,
  Description,
} from "@heroui/react";
import {
  RefreshCw,
  Shield,
  CheckCircle2,
  XCircle,
  Smartphone,
  Unplug,
  Sparkles,
  ExternalLink,
  AlertTriangle,
} from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import {
  bootstrapAtm,
  generatePairingCode,
  setNeuralCredentials,
  setNeuralWorkspaceId,
  loadNeuralCredentials,
  registerNode,
  setHeadlessMode,
  hasAtmCredentials,
  WorkspaceAuth,
  getNeuralCredentialsValues,
  AirlockLevels,
  listAtmCommands,
  getAtmCommandDetails,
  getAtmCommandProgress,
  getAtmCommandMetrics,
  getAtmWorkspaceCommandMetrics,
  getAtmEndpointMetrics,
  syncAtmMetricsAlerts,
  listAtmMetricsAlerts,
  ackAtmMetricsAlert,
  getAtmMetricsSlo,
  updateAtmMetricsSlo,
  getAtmMetricsAlertRetention,
  updateAtmMetricsAlertRetention,
  cleanupAtmMetricsAlerts,
  getAtmAdminPermissions,
  AtmCommandSummary,
  AtmCommandProgressEvent,
  AtmCommandMetricsResponse,
  AtmWorkspaceCommandMetricsResponse,
  AtmEndpointMetricsResponse,
  AtmMetricsAlert,
  AtmMetricsSloConfig,
  AtmMetricsAlertRetentionConfig,
  AtmAdminPermissions,
} from "../../services/tauri";
import { useAirlock } from "../../hooks/useAirlock";
import { DEFAULT_NEURAL_SKILLS } from "../../constants/defaultNeuralSkills";
import { AgentList } from "./AgentList";
import { CreateAgentForm } from "./CreateAgentForm";
import { AgentRuntimePanel } from "./AgentRuntimePanel";

type NeuralState = "idle" | "restored" | "connected" | "connecting";
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
const DEFAULT_ADMIN_PERMISSIONS: AtmAdminPermissions = {
  canEditSlo: true,
  canAckAlerts: true,
  canEditAlertRetention: true,
  canRunAlertCleanup: true,
};

const NEURAL_WORKSPACE_STORAGE_KEY = "rainy-neural-workspace";

type StoredWorkspace = {
  id: string;
  name: string;
};

const readStoredWorkspace = (): StoredWorkspace | null => {
  if (typeof window === "undefined") return null;
  try {
    const raw = localStorage.getItem(NEURAL_WORKSPACE_STORAGE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as StoredWorkspace;
    if (!parsed?.id || !parsed?.name) return null;
    return parsed;
  } catch (err) {
    console.warn("Failed to parse stored Neural workspace:", err);
    return null;
  }
};

const writeStoredWorkspace = (workspace: WorkspaceAuth) => {
  if (typeof window === "undefined") return;
  try {
    const stored: StoredWorkspace = { id: workspace.id, name: workspace.name };
    localStorage.setItem(NEURAL_WORKSPACE_STORAGE_KEY, JSON.stringify(stored));
  } catch (err) {
    console.warn("Failed to persist Neural workspace:", err);
  }
};

const clearStoredWorkspace = () => {
  if (typeof window === "undefined") return;
  try {
    localStorage.removeItem(NEURAL_WORKSPACE_STORAGE_KEY);
  } catch (err) {
    console.warn("Failed to clear stored Neural workspace:", err);
  }
};

export function NeuralPanel() {
  const [state, setState] = useState<NeuralState>("idle");
  const [workspace, setWorkspace] = useState<WorkspaceAuth | null>(null);

  const [platformKey, setPlatformKey] = useState("");
  const [userApiKey, setUserApiKey] = useState("");
  const [workspaceName, setWorkspaceName] = useState("");

  const [pairingCode, setPairingCode] = useState<string | null>(null);
  const [isCreatingAgent, setIsCreatingAgent] = useState(false);
  const [agentsRefreshToken, setAgentsRefreshToken] = useState(0);
  const [isHeadless, setIsHeadless] = useState(false);
  const { pendingRequests: pendingApprovals, respond: respondAirlock } =
    useAirlock();
  const [hasAtmKey, setHasAtmKey] = useState<boolean | null>(null);
  const [activeView, setActiveView] = useState<"dashboard" | "runtime">(
    "dashboard",
  );
  const [recentCommands, setRecentCommands] = useState<AtmCommandSummary[]>([]);
  const [selectedCommandId, setSelectedCommandId] = useState<string | null>(null);
  const [commandProgress, setCommandProgress] = useState<AtmCommandProgressEvent[]>(
    [],
  );
  const [commandMetrics, setCommandMetrics] =
    useState<AtmCommandMetricsResponse | null>(null);
  const [workspaceCommandMetrics, setWorkspaceCommandMetrics] =
    useState<AtmWorkspaceCommandMetricsResponse | null>(null);
  const [endpointMetrics, setEndpointMetrics] =
    useState<AtmEndpointMetricsResponse | null>(null);
  const [endpointAlerts, setEndpointAlerts] = useState<EndpointAlert[]>([]);
  const [persistedAlerts, setPersistedAlerts] = useState<AtmMetricsAlert[]>([]);
  const [alertHistoryStatus, setAlertHistoryStatus] =
    useState<AlertHistoryStatus>("open");
  const [alertRetention, setAlertRetention] =
    useState<AtmMetricsAlertRetentionConfig>(DEFAULT_ALERT_RETENTION);
  const [adminPermissions, setAdminPermissions] =
    useState<AtmAdminPermissions>(DEFAULT_ADMIN_PERMISSIONS);
  const [sloThresholds, setSloThresholds] =
    useState<AtmMetricsSloConfig>(DEFAULT_SLO_THRESHOLDS);
  const [isLoadingCommands, setIsLoadingCommands] = useState(false);
  const [isLoadingProgress, setIsLoadingProgress] = useState(false);
  const [isLoadingMetrics, setIsLoadingMetrics] = useState(false);
  const [isSyncingAlerts, setIsSyncingAlerts] = useState(false);
  const [isSavingSlo, setIsSavingSlo] = useState(false);
  const [isSavingAlertRetention, setIsSavingAlertRetention] = useState(false);
  const [isCleaningAlerts, setIsCleaningAlerts] = useState(false);
  const progressSinceRef = useRef(0);
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
    return typeof value === "number" && Number.isFinite(value) ? value : fallback;
  };
  const severityClass = (severity: AlertSeverity) => {
    if (severity === "critical") {
      return "border-red-500/30 bg-red-500/10";
    }
    if (severity === "warn") {
      return "border-orange-500/30 bg-orange-500/10";
    }
    return "border-white/5 bg-background/20";
  };
  const classifyEndpointSeverity = (
    errorRate?: number | null,
    p95Ms?: number | null,
  ): AlertSeverity => {
    if (
      (typeof errorRate === "number" &&
        errorRate >= thresholdNumber("endpointErrorRateCritical", 20)) ||
      (typeof p95Ms === "number" &&
        p95Ms >= thresholdNumber("endpointP95CriticalMs", 10000))
    ) {
      return "critical";
    }
    if (
      (typeof errorRate === "number" &&
        errorRate >= thresholdNumber("endpointErrorRateWarn", 5)) ||
      (typeof p95Ms === "number" &&
        p95Ms >= thresholdNumber("endpointP95WarnMs", 4000))
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
      const p95 = endpoint.latency.p95TotalMs ?? endpoint.latency.p95RunMs ?? null;
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

      const sloErrorRateTarget = thresholdNumber("endpointSloErrorRateTarget", 2);
      const sloP95Target = thresholdNumber("endpointSloP95TargetMs", 2500);
      const criticalErrorRate = thresholdNumber("endpointErrorRateCritical", 20);
      const criticalP95 = thresholdNumber("endpointP95CriticalMs", 10000);
      const regressionErrorFactor = thresholdNumber(
        "endpointRegressionErrorRateFactor",
        1.5,
      );
      const regressionErrorDelta = thresholdNumber(
        "endpointRegressionErrorRateDelta",
        2,
      );
      const regressionP95Factor = thresholdNumber("endpointRegressionP95Factor", 1.5);
      const regressionP95Delta = thresholdNumber("endpointRegressionP95DeltaMs", 1000);

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
  const workspaceFailureSeverity: AlertSeverity = (() => {
    if (!workspaceCommandMetrics) return "normal";
    const buckets = workspaceCommandMetrics.failureBuckets;
    if (
      (buckets.timeout || 0) >= thresholdNumber("failureTimeoutCritical", 10) ||
      (buckets.runtime_error || 0) >= thresholdNumber("failureRuntimeCritical", 15) ||
      (buckets.transport_error || 0) >= thresholdNumber("failureTransportCritical", 6)
    ) {
      return "critical";
    }
    if (
      (buckets.timeout || 0) >= thresholdNumber("failureTimeoutWarn", 3) ||
      (buckets.runtime_error || 0) >= thresholdNumber("failureRuntimeWarn", 5) ||
      (buckets.transport_error || 0) >= thresholdNumber("failureTransportWarn", 2)
    ) {
      return "warn";
    }
    return "normal";
  })();

  const refreshPersistedAlerts = useCallback(
    async (statusOverride?: AlertHistoryStatus) => {
      const status = statusOverride ?? alertHistoryStatus;
      const statusParam = status === "all" ? undefined : status;
      try {
        const alerts = await listAtmMetricsAlerts(statusParam, 50);
        setPersistedAlerts(alerts);
      } catch (err) {
        console.error("Failed to load persisted alerts:", err);
      }
    },
    [alertHistoryStatus],
  );

  const loadSloThresholds = useCallback(async () => {
    try {
      const remote = await getAtmMetricsSlo();
      setSloThresholds(remote);
    } catch (err) {
      console.error("Failed to load metrics SLO thresholds:", err);
    }
  }, []);

  const loadAlertRetention = useCallback(async () => {
    try {
      const remote = await getAtmMetricsAlertRetention();
      setAlertRetention(remote);
    } catch (err) {
      console.error("Failed to load alert retention:", err);
    }
  }, []);

  const loadAdminPermissions = useCallback(async () => {
    try {
      const permissions = await getAtmAdminPermissions();
      setAdminPermissions(permissions);
    } catch (err) {
      console.error("Failed to load admin permissions:", err);
    }
  }, []);

  useEffect(() => {
    let cancelled = false;
    const init = async () => {
      let atmKeyPresent: boolean | null = null;
      try {
        atmKeyPresent = await hasAtmCredentials();
        if (!cancelled) setHasAtmKey(atmKeyPresent);
      } catch (err) {
        console.error("Failed to check ATM admin key:", err);
      }

      try {
        const hasCredentials = await loadNeuralCredentials();
        if (cancelled) return;
        if (hasCredentials) {
          const creds = await getNeuralCredentialsValues();
          if (cancelled) return;
          if (creds) {
            const platform = creds[0];
            const userKey = creds[1];
            setPlatformKey(platform);
            setUserApiKey(userKey);

            const storedWorkspace = readStoredWorkspace();
            let effectiveWorkspace: WorkspaceAuth | null = null;

            const shouldBootstrap = !storedWorkspace || !atmKeyPresent;
            if (shouldBootstrap) {
              try {
                const ws = await bootstrapAtm(
                  platform,
                  userKey,
                  storedWorkspace?.name || "Desktop Workspace",
                );
                if (!cancelled) {
                  setHasAtmKey(true);
                }
                writeStoredWorkspace(ws);
                effectiveWorkspace = ws;
              } catch (err) {
                console.error("Failed to restore ATM admin key:", err);
                if (!cancelled) {
                  setHasAtmKey(false);
                }
                if (storedWorkspace) {
                  effectiveWorkspace = {
                    id: storedWorkspace.id,
                    name: storedWorkspace.name,
                    apiKey: "",
                  };
                }
              }
            } else if (storedWorkspace) {
              effectiveWorkspace = {
                id: storedWorkspace.id,
                name: storedWorkspace.name,
                apiKey: "",
              };
            }

            if (effectiveWorkspace) {
              setWorkspace(effectiveWorkspace);
              setState("connecting");
              try {
                await setNeuralWorkspaceId(effectiveWorkspace.id);
                await registerNode(DEFAULT_NEURAL_SKILLS, []);
                if (!cancelled) {
                  setState("connected");
                }
              } catch (err) {
                console.error("Failed to restore Neural session:", err);
                if (!cancelled) {
                  setWorkspace(null);
                  setState("restored");
                }
              }
            } else {
              setState("restored");
            }
          }
        }
      } catch (err) {
        console.error("Failed to load credentials:", err);
      }

    };
    init();
    return () => {
      cancelled = true;
    };
  }, []);

  const refreshRecentCommands = useCallback(async () => {
    if (state !== "connected") return;
    setIsLoadingCommands(true);
    try {
      const [commandsResult, workspaceMetricsResult, endpointMetricsResult] =
        await Promise.allSettled([
          listAtmCommands(25),
          getAtmWorkspaceCommandMetrics(24 * 60 * 60 * 1000, 500),
          getAtmEndpointMetrics(60 * 60 * 1000, 2000),
        ]);

      if (commandsResult.status !== "fulfilled") {
        throw commandsResult.reason;
      }
      const commands = commandsResult.value;
      setRecentCommands(commands);
      if (workspaceMetricsResult.status === "fulfilled") {
        setWorkspaceCommandMetrics(workspaceMetricsResult.value);
      }
      if (endpointMetricsResult.status === "fulfilled") {
        const metrics = endpointMetricsResult.value;
        setEndpointMetrics(metrics);
        const generatedAlerts = buildEndpointAlerts(metrics);
        setEndpointAlerts(generatedAlerts);
        setIsSyncingAlerts(true);
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
        } finally {
          setIsSyncingAlerts(false);
        }
      }
      await refreshPersistedAlerts();
      if (commands.length === 0) {
        setSelectedCommandId(null);
        setCommandProgress([]);
        setCommandMetrics(null);
        progressSinceRef.current = 0;
        return;
      }
      if (!selectedCommandId || !commands.some((c) => c.id === selectedCommandId)) {
        const active =
          commands.find((c) => c.status === "running" || c.status === "pending") ||
          commands[0];
        setSelectedCommandId(active.id);
      }
    } catch (err) {
      console.error("Failed to load recent commands:", err);
    } finally {
      setIsLoadingCommands(false);
    }
  }, [refreshPersistedAlerts, selectedCommandId, state]);

  useEffect(() => {
    if (state !== "connected") return;
    refreshRecentCommands();
    const interval = setInterval(() => {
      refreshRecentCommands();
    }, 5000);
    return () => clearInterval(interval);
  }, [refreshRecentCommands, state]);

  useEffect(() => {
    if (state !== "connected") return;
    loadSloThresholds();
    loadAlertRetention();
    loadAdminPermissions();
  }, [loadAdminPermissions, loadAlertRetention, loadSloThresholds, state]);

  useEffect(() => {
    if (state !== "connected") return;
    refreshPersistedAlerts(alertHistoryStatus);
  }, [alertHistoryStatus, refreshPersistedAlerts, state]);

  useEffect(() => {
    if (state !== "connected" || !selectedCommandId) return;
    let cancelled = false;

    const loadInitial = async () => {
      setIsLoadingProgress(true);
      setIsLoadingMetrics(true);
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
        if (!cancelled) setIsLoadingMetrics(false);
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
  }, [selectedCommandId, state]);

  const handleConnect = async () => {
    if (!platformKey.trim() || !userApiKey.trim()) {
      toast.error("Credentials are required");
      return;
    }

    setState("connecting");

    try {
      const ws = await bootstrapAtm(
        platformKey,
        userApiKey,
        workspaceName.trim() || "Desktop Workspace",
      );
      await setNeuralCredentials(platformKey, userApiKey);
      await setNeuralWorkspaceId(ws.id);
      await registerNode(DEFAULT_NEURAL_SKILLS, []);

      setWorkspace(ws);
      writeStoredWorkspace(ws);
      setHasAtmKey(true);
      setState("connected");
      toast.success(`Neural Link Established! Welcome to ${ws.name}`);
    } catch (err: any) {
      console.error("Connection failed:", err);
      setState("idle");
      toast.error("Connection failed. Please check your credentials.");
    }
  };

  const handleGeneratePairingCode = async () => {
    try {
      const res = await generatePairingCode();
      setPairingCode(res.code);
    } catch (err) {
      toast.error("Failed to generate pairing code");
    }
  };

  const handleToggleHeadless = async (enabled: boolean) => {
    try {
      await setHeadlessMode(enabled);
      setIsHeadless(enabled);
      toast.success(`Headless Mode ${enabled ? "Enabled" : "Disabled"}`);
    } catch (err) {
      toast.error("Failed to update settings");
    }
  };

  const handleAirlockRespond = async (commandId: string, approved: boolean) => {
    try {
      await respondAirlock(commandId, approved);
      toast.success(approved ? "Request Approved" : "Request Denied");
    } catch (err) {
      toast.error("Failed to process response");
    }
  };

  const handleLogout = async () => {
    if (
      confirm(
        "⚠️ This will disconnect you from the Cloud Cortex. Are you sure?",
      )
    ) {
      try {
        const { resetNeuralWorkspace } = await import("../../services/tauri");
        await resetNeuralWorkspace(platformKey, userApiKey);
        setPlatformKey("");
        setUserApiKey("");
        setWorkspace(null);
        setState("idle");
        setPairingCode(null);
        setHasAtmKey(false);
        clearStoredWorkspace();
        toast.success("Succesfully disconnected");
      } catch (e: any) {
        toast.error(e?.message || "Logout failed");
      }
    }
  };

  const handleAckAlert = async (alertId: string) => {
    if (!adminPermissions.canAckAlerts) {
      toast.error("Alert acknowledgement is disabled by workspace policy");
      return;
    }
    try {
      await ackAtmMetricsAlert(alertId, "desktop-admin");
      await refreshPersistedAlerts();
      toast.success("Alert acknowledged");
    } catch (err) {
      console.error("Failed to acknowledge alert:", err);
      toast.error("Failed to acknowledge alert");
    }
  };

  const handleSloInputChange = (
    key: keyof AtmMetricsSloConfig,
    value: string,
  ) => {
    const parsed = Number(value);
    setSloThresholds((prev) => ({
      ...prev,
      [key]: Number.isFinite(parsed) ? parsed : prev[key],
    }));
  };

  const handleSaveSloThresholds = async () => {
    if (!adminPermissions.canEditSlo) {
      toast.error("SLO editing is disabled by workspace policy");
      return;
    }
    setIsSavingSlo(true);
    try {
      const updated = await updateAtmMetricsSlo(sloThresholds);
      setSloThresholds(updated);
      toast.success("SLO thresholds updated");
      await refreshRecentCommands();
    } catch (err) {
      console.error("Failed to save SLO thresholds:", err);
      toast.error("Failed to save SLO thresholds");
    } finally {
      setIsSavingSlo(false);
    }
  };

  const handleSaveAlertRetention = async () => {
    if (!adminPermissions.canEditAlertRetention) {
      toast.error("Alert retention editing is disabled by workspace policy");
      return;
    }
    setIsSavingAlertRetention(true);
    try {
      const updated = await updateAtmMetricsAlertRetention(alertRetention);
      setAlertRetention(updated);
      toast.success("Alert retention updated");
      await refreshPersistedAlerts();
    } catch (err) {
      console.error("Failed to save alert retention:", err);
      toast.error("Failed to save alert retention");
    } finally {
      setIsSavingAlertRetention(false);
    }
  };

  const handleCleanupAlerts = async () => {
    if (!adminPermissions.canRunAlertCleanup) {
      toast.error("Alert cleanup is disabled by workspace policy");
      return;
    }
    setIsCleaningAlerts(true);
    try {
      const result = await cleanupAtmMetricsAlerts();
      toast.success(`Cleanup completed: ${result.deleted} alert(s) deleted`);
      await refreshPersistedAlerts();
    } catch (err) {
      console.error("Failed to cleanup alerts:", err);
      toast.error("Failed to cleanup alerts");
    } finally {
      setIsCleaningAlerts(false);
    }
  };

  return (
    <div className="h-full w-full relative bg-transparent overflow-hidden text-foreground">
      {/* Background Ambience / Base Layer (Matches AgentChat) */}
      <div className="absolute inset-0 bg-gradient-to-b from-primary/5 via-background/50 to-background/80 pointer-events-none z-0" />

      {/* Scrollable Content Area - Absolute Inset - Z-10 */}
      <div className="absolute inset-0 overflow-y-auto w-full h-full scrollbar-none z-10">
        <div className="flex flex-col px-6 max-w-4xl mx-auto min-h-full pt-16 pb-20">
          {/* Header - Flat, No Icons, Centered */}
          <div className="flex items-center justify-between mb-8">
            <div>
              <h1 className="text-2xl font-bold tracking-tight text-foreground">
                Neural Link
              </h1>
              <p className="text-muted-foreground text-sm font-medium">
                Desktop Node Management
              </p>
              {hasAtmKey !== null && (
                <div className="flex items-center gap-2 mt-2">
                  <span className="text-[10px] uppercase tracking-widest text-muted-foreground">
                    ATM Admin Key
                  </span>
                  <Chip
                    size="sm"
                    variant="soft"
                    className={
                      hasAtmKey
                        ? "bg-green-500/10 text-green-500"
                        : "bg-red-500/10 text-red-400"
                    }
                  >
                    {hasAtmKey ? "Stored" : "Missing"}
                  </Chip>
                </div>
              )}
            </div>

            {state === "connected" && (
              <div className="flex items-center gap-3">
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-9 hover:bg-background/40"
                >
                  <RefreshCw className="size-3 mr-2" />
                  Refresh
                </Button>
                <Button
                  variant="danger-soft"
                  size="sm"
                  onPress={handleLogout}
                  className="h-9"
                >
                  <Unplug className="size-3 mr-2" />
                  Disconnect
                </Button>
              </div>
            )}
          </div>

          <Separator className="mb-10 opacity-40" />

          {/* STATE: IDLE (No Credentials) - Floating Inputs */}
          {state === "idle" && (
            <div className="max-w-lg mx-auto w-full flex flex-col gap-8 animate-appear">
              <div className="text-center space-y-2 mb-4">
                <h2 className="text-xl font-semibold">
                  Authentication Required
                </h2>
                <p className="text-muted-foreground text-sm">
                  Provide your credentials to join the Cloud Cortex.
                </p>
              </div>

              <div className="space-y-6">
                <TextField>
                  <Label className="uppercase text-[10px] font-bold tracking-widest text-muted-foreground mb-1.5 ml-1">
                    Platform API Key
                  </Label>
                  <Input
                    type="password"
                    placeholder="rk_live_..."
                    value={platformKey}
                    onChange={(e) => setPlatformKey(e.target.value)}
                    className="bg-background/40 backdrop-blur-sm border-white/5 hover:border-white/10 focus:border-primary/50 transition-colors h-12"
                  />
                  <Description className="text-xs mt-1.5 ml-1 text-muted-foreground/60">
                    Available at platform.rainymate.com
                  </Description>
                </TextField>

                <TextField>
                  <Label className="uppercase text-[10px] font-bold tracking-widest text-muted-foreground mb-1.5 ml-1">
                    Creator API Key
                  </Label>
                  <Input
                    type="password"
                    placeholder="rny_..."
                    value={userApiKey}
                    onChange={(e) => setUserApiKey(e.target.value)}
                    className="bg-background/40 backdrop-blur-sm border-white/5 hover:border-white/10 focus:border-primary/50 transition-colors h-12"
                  />
                </TextField>

                <TextField>
                  <Label className="uppercase text-[10px] font-bold tracking-widest text-muted-foreground mb-1.5 ml-1">
                    Workspace Name (Optional)
                  </Label>
                  <Input
                    placeholder="e.g. My Neural Net"
                    value={workspaceName}
                    onChange={(e) => setWorkspaceName(e.target.value)}
                    className="bg-background/40 backdrop-blur-sm border-white/5 hover:border-white/10 focus:border-primary/50 transition-colors h-12"
                  />
                </TextField>

                <Button
                  variant="primary"
                  className="w-full h-12 font-bold shadow-lg shadow-primary/20 mt-4"
                  onPress={handleConnect}
                >
                  Connect Node
                </Button>
              </div>
            </div>
          )}

          {/* STATE: RESTORED - Minimal Welcome */}
          {state === "restored" && (
            <div className="max-w-md mx-auto w-full flex flex-col items-center justify-center pt-20 animate-appear text-center">
              <div className="mb-6 relative">
                <div className="absolute inset-0 bg-primary/20 blur-3xl rounded-full opacity-50" />
                <Shield className="size-16 text-primary/80 relative z-10" />
              </div>

              <h2 className="text-2xl font-bold mb-2">Welcome Back</h2>
              <p className="text-muted-foreground mb-8">
                Capabilities restored. Ready to reconnect?
              </p>

              <div className="flex flex-col gap-3 w-full">
                <Button
                  variant="primary"
                  size="lg"
                  className="h-12 font-bold shadow-xl shadow-primary/20 w-full"
                  onPress={handleConnect}
                >
                  Quick Connect
                </Button>
                <Button
                  variant="ghost"
                  onPress={() => {
                    setState("idle");
                    setWorkspace(null);
                    clearStoredWorkspace();
                  }}
                  className="font-medium text-muted-foreground hover:text-foreground w-full"
                >
                  Use Different Keys
                </Button>
              </div>
            </div>
          )}

          {/* STATE: CONNECTING - Minimal Spinner */}
          {state === "connecting" && (
            <div className="flex-1 flex flex-col items-center justify-center pt-20 animate-appear">
              <div className="size-12 border-2 border-primary/20 border-t-primary rounded-full animate-spin mb-6" />
              <h2 className="text-lg font-semibold animate-pulse">
                Synchronizing...
              </h2>
            </div>
          )}

          {/* STATE: CONNECTED - Flat Dashboard */}
          {state === "connected" && workspace && (
            <div className="animate-appear space-y-6">
              {/* View Switcher */}
              <div className="flex justify-center">
                <div className="bg-white/5 p-1 rounded-lg flex items-center gap-1 border border-white/5">
                  <Button
                    size="sm"
                    variant={activeView === "dashboard" ? "primary" : "ghost"}
                    onPress={() => setActiveView("dashboard")}
                    className="h-8"
                  >
                    Dashboard
                  </Button>
                  <Button
                    size="sm"
                    variant={activeView === "runtime" ? "primary" : "ghost"}
                    onPress={() => setActiveView("runtime")}
                    className="h-8"
                  >
                    Agent Runtime
                  </Button>
                </div>
              </div>

              {activeView === "runtime" ? (
                <div className="h-[600px] animate-appear">
                  <AgentRuntimePanel workspaceId={workspace.id} />
                </div>
              ) : (
                <div className="space-y-12">
                  {/* Workspace Info & Quick Stats */}
                  <div className="grid grid-cols-1 md:grid-cols-3 gap-8">
                    {/* Info Column */}
                    <div className="md:col-span-2 space-y-6">
                      <div>
                        <div className="flex items-center gap-3 mb-2">
                          <h2 className="text-4xl font-light tracking-tight text-foreground">
                            {workspace.name}
                          </h2>
                          <Chip
                            color="success"
                            size="sm"
                            variant="soft"
                            className="bg-green-500/10 text-green-500"
                          >
                            Active
                          </Chip>
                        </div>
                        <div className="flex items-center gap-4 text-xs font-mono text-muted-foreground">
                          <span>ID: {workspace.id}</span>
                          <span className="text-border/40">|</span>
                          <span>NODE: Desktop_v2</span>
                          <span className="text-border/40">|</span>
                          <span className="flex items-center gap-1 text-green-500/80">
                            <Shield className="size-3" /> Encrypted
                          </span>
                        </div>
                      </div>

                      <div className="flex gap-4">
                        <Button
                          variant="outline"
                          size="sm"
                          className="border-white/10 hover:bg-white/5 bg-transparent"
                        >
                          <ExternalLink className="size-3 mr-2 opacity-50" />
                          View in Cloud
                        </Button>
                      </div>
                    </div>

                    {/* Settings Column */}
                    <div className="flex flex-col gap-4 justify-center md:items-end font-sans">
                      {/* Headless Toggle */}
                      <div className="flex items-center justify-between gap-4 p-5 rounded-2xl bg-default-100/40 hover:bg-default-100/60 transition-all border border-white/5 hover:border-white/10 w-full backdrop-blur-xl group">
                        <div className="flex items-center gap-4">
                          <div className="size-10 rounded-xl bg-purple-500/10 flex items-center justify-center text-purple-400 group-hover:scale-110 transition-transform">
                            <Shield className="size-5" />
                          </div>
                          <div className="flex flex-col gap-0.5">
                            <span className="text-sm font-semibold text-foreground">
                              Headless Mode
                            </span>
                            <span className="text-[10px] text-muted-foreground uppercase tracking-wider font-medium">
                              {isHeadless ? "Active" : "Disabled"}
                            </span>
                          </div>
                        </div>
                        <Switch
                          isSelected={isHeadless}
                          onChange={handleToggleHeadless}
                          size="lg"
                          className="group-data-[selected=true]:bg-purple-500"
                        />
                      </div>

                      {/* Mobile Link */}
                      <div className="flex items-center justify-between gap-4 p-5 rounded-2xl bg-default-100/40 hover:bg-default-100/60 transition-all border border-white/5 hover:border-white/10 w-full backdrop-blur-xl group">
                        <div className="flex items-center gap-4">
                          <div className="size-10 rounded-xl bg-blue-500/10 flex items-center justify-center text-blue-400 group-hover:scale-110 transition-transform">
                            <Smartphone className="size-5" />
                          </div>
                          <div className="flex flex-col gap-0.5">
                            <span className="text-sm font-semibold text-foreground">
                              Mobile Link
                            </span>
                            <span className="text-[10px] text-muted-foreground uppercase tracking-wider font-medium">
                              Remote Access
                            </span>
                          </div>
                        </div>
                        {pairingCode ? (
                          <div className="flex flex-col items-end">
                            <span className="font-mono text-xl font-bold text-blue-400 tracking-widest">
                              {pairingCode}
                            </span>
                            <span className="text-[10px] text-muted-foreground">
                              Expires in 5m
                            </span>
                          </div>
                        ) : (
                          <Button
                            size="sm"
                            variant="ghost"
                            className="font-semibold text-primary"
                            onPress={handleGeneratePairingCode}
                          >
                            Generate
                          </Button>
                        )}
                      </div>
                    </div>
                  </div>

                  <Separator className="opacity-20" />

                  {/* Agents Section - Clean List */}
                  <div className="space-y-6">
                    {/* Agent List Container - Transparent */}
                    <div className="rounded-xl border border-white/5 bg-background/20 backdrop-blur-md overflow-hidden p-1">
                      <AgentList
                        onCreateClick={() => setIsCreatingAgent(true)}
                        refreshToken={agentsRefreshToken}
                      />
                    </div>
                  </div>

                  {/* Command Stream Section */}
                  <div className="space-y-6">
                    <div className="flex items-center justify-between">
                      <h3 className="text-lg font-semibold flex items-center gap-2">
                        <RefreshCw className="size-4 text-blue-500" />
                        Command Stream
                      </h3>
                      <Button
                        size="sm"
                        variant="ghost"
                        onPress={refreshRecentCommands}
                        isDisabled={isLoadingCommands}
                      >
                        {isLoadingCommands ? "Refreshing..." : "Refresh"}
                      </Button>
                    </div>

                    <div className="rounded-xl border border-white/5 bg-background/20 backdrop-blur-md p-3 space-y-3">
                      <div className="flex items-center justify-between">
                        <span className="text-xs uppercase tracking-wider text-muted-foreground">
                          SLO Thresholds
                        </span>
                        <Button
                          size="sm"
                          variant="ghost"
                          onPress={handleSaveSloThresholds}
                          isDisabled={isSavingSlo || !adminPermissions.canEditSlo}
                        >
                          {isSavingSlo ? "Saving..." : "Save Thresholds"}
                        </Button>
                      </div>
                      {!adminPermissions.canEditSlo && (
                        <div className="text-[11px] text-muted-foreground">
                          SLO editing is disabled by workspace policy.
                        </div>
                      )}
                      <div className="grid grid-cols-2 md:grid-cols-4 gap-2">
                        <Input
                          type="number"
                          placeholder="Err Warn %"
                          value={String(sloThresholds.endpointErrorRateWarn)}
                          onChange={(e) =>
                            handleSloInputChange("endpointErrorRateWarn", e.target.value)
                          }
                        />
                        <Input
                          type="number"
                          placeholder="Err Critical %"
                          value={String(sloThresholds.endpointErrorRateCritical)}
                          onChange={(e) =>
                            handleSloInputChange(
                              "endpointErrorRateCritical",
                              e.target.value,
                            )
                          }
                        />
                        <Input
                          type="number"
                          placeholder="P95 Warn ms"
                          value={String(sloThresholds.endpointP95WarnMs)}
                          onChange={(e) =>
                            handleSloInputChange("endpointP95WarnMs", e.target.value)
                          }
                        />
                        <Input
                          type="number"
                          placeholder="P95 Critical ms"
                          value={String(sloThresholds.endpointP95CriticalMs)}
                          onChange={(e) =>
                            handleSloInputChange(
                              "endpointP95CriticalMs",
                              e.target.value,
                            )
                          }
                        />
                        <Input
                          type="number"
                          placeholder="SLO Err %"
                          value={String(sloThresholds.endpointSloErrorRateTarget)}
                          onChange={(e) =>
                            handleSloInputChange(
                              "endpointSloErrorRateTarget",
                              e.target.value,
                            )
                          }
                        />
                        <Input
                          type="number"
                          placeholder="SLO P95 ms"
                          value={String(sloThresholds.endpointSloP95TargetMs)}
                          onChange={(e) =>
                            handleSloInputChange(
                              "endpointSloP95TargetMs",
                              e.target.value,
                            )
                          }
                        />
                        <Input
                          type="number"
                          placeholder="Reg Err Factor"
                          value={String(sloThresholds.endpointRegressionErrorRateFactor)}
                          onChange={(e) =>
                            handleSloInputChange(
                              "endpointRegressionErrorRateFactor",
                              e.target.value,
                            )
                          }
                        />
                        <Input
                          type="number"
                          placeholder="Reg Err Delta"
                          value={String(sloThresholds.endpointRegressionErrorRateDelta)}
                          onChange={(e) =>
                            handleSloInputChange(
                              "endpointRegressionErrorRateDelta",
                              e.target.value,
                            )
                          }
                        />
                        <Input
                          type="number"
                          placeholder="Reg P95 Factor"
                          value={String(sloThresholds.endpointRegressionP95Factor)}
                          onChange={(e) =>
                            handleSloInputChange(
                              "endpointRegressionP95Factor",
                              e.target.value,
                            )
                          }
                        />
                        <Input
                          type="number"
                          placeholder="Reg P95 Delta"
                          value={String(sloThresholds.endpointRegressionP95DeltaMs)}
                          onChange={(e) =>
                            handleSloInputChange(
                              "endpointRegressionP95DeltaMs",
                              e.target.value,
                            )
                          }
                        />
                        <Input
                          type="number"
                          placeholder="Timeout Warn"
                          value={String(sloThresholds.failureTimeoutWarn)}
                          onChange={(e) =>
                            handleSloInputChange("failureTimeoutWarn", e.target.value)
                          }
                        />
                        <Input
                          type="number"
                          placeholder="Timeout Critical"
                          value={String(sloThresholds.failureTimeoutCritical)}
                          onChange={(e) =>
                            handleSloInputChange(
                              "failureTimeoutCritical",
                              e.target.value,
                            )
                          }
                        />
                        <Input
                          type="number"
                          placeholder="Runtime Warn"
                          value={String(sloThresholds.failureRuntimeWarn)}
                          onChange={(e) =>
                            handleSloInputChange("failureRuntimeWarn", e.target.value)
                          }
                        />
                        <Input
                          type="number"
                          placeholder="Runtime Critical"
                          value={String(sloThresholds.failureRuntimeCritical)}
                          onChange={(e) =>
                            handleSloInputChange(
                              "failureRuntimeCritical",
                              e.target.value,
                            )
                          }
                        />
                        <Input
                          type="number"
                          placeholder="Transport Warn"
                          value={String(sloThresholds.failureTransportWarn)}
                          onChange={(e) =>
                            handleSloInputChange("failureTransportWarn", e.target.value)
                          }
                        />
                        <Input
                          type="number"
                          placeholder="Transport Critical"
                          value={String(sloThresholds.failureTransportCritical)}
                          onChange={(e) =>
                            handleSloInputChange(
                              "failureTransportCritical",
                              e.target.value,
                            )
                          }
                        />
                      </div>
                    </div>

                    {workspaceCommandMetrics && (
                      <div
                        className={`rounded-xl border backdrop-blur-md p-3 space-y-2 ${severityClass(workspaceFailureSeverity)}`}
                      >
                        <div className="flex items-center justify-between">
                          <span className="text-xs uppercase tracking-wider text-muted-foreground">
                            Workspace Metrics
                          </span>
                          <span className="text-[11px] text-muted-foreground font-mono">
                            {(workspaceCommandMetrics.windowMs / (60 * 60 * 1000)).toFixed(0)}
                            h window
                          </span>
                        </div>
                        {workspaceFailureSeverity !== "normal" && (
                          <div
                            className={`flex items-center gap-2 text-xs ${
                              workspaceFailureSeverity === "critical"
                                ? "text-red-400"
                                : "text-orange-400"
                            }`}
                          >
                            <AlertTriangle className="size-3.5" />
                            {workspaceFailureSeverity === "critical"
                              ? "Failure spike detected"
                              : "Failure rate elevated"}
                          </div>
                        )}
                        <div className="flex flex-wrap gap-2">
                          <Chip size="sm" variant="soft">
                            timeout: {workspaceCommandMetrics.failureBuckets.timeout || 0}
                          </Chip>
                          <Chip size="sm" variant="soft">
                            airlock:{" "}
                            {workspaceCommandMetrics.failureBuckets.airlock_rejected || 0}
                          </Chip>
                          <Chip size="sm" variant="soft">
                            runtime:{" "}
                            {workspaceCommandMetrics.failureBuckets.runtime_error || 0}
                          </Chip>
                          <Chip size="sm" variant="soft">
                            transport:{" "}
                            {workspaceCommandMetrics.failureBuckets.transport_error || 0}
                          </Chip>
                        </div>
                        <div className="grid grid-cols-3 gap-2 text-[11px] font-mono text-muted-foreground">
                          <div>
                            queue:{" "}
                            {formatMs(workspaceCommandMetrics.averages.queueDelayMs)}
                          </div>
                          <div>
                            run: {formatMs(workspaceCommandMetrics.averages.runDurationMs)}
                          </div>
                          <div>
                            total:{" "}
                            {formatMs(workspaceCommandMetrics.averages.totalDurationMs)}
                          </div>
                        </div>
                      </div>
                    )}

                    {endpointMetrics && endpointMetrics.endpoints.length > 0 && (
                      <div className="grid grid-cols-1 md:grid-cols-3 gap-2">
                        {endpointMetrics.endpoints.map((metric) => {
                          const p95 =
                            metric.latency.p95TotalMs ?? metric.latency.p95RunMs;
                          const avg =
                            metric.latency.avgTotalMs ?? metric.latency.avgRunMs;
                          const severity = classifyEndpointSeverity(
                            metric.errorRate,
                            p95,
                          );
                          return (
                            <div
                              key={metric.key}
                              className={`rounded-xl border backdrop-blur-md p-3 space-y-1.5 ${severityClass(severity)}`}
                            >
                              <div className="flex items-center justify-between gap-2">
                                <div className="text-xs uppercase tracking-wider text-muted-foreground">
                                  {metric.label}
                                </div>
                                {severity !== "normal" && (
                                  <Chip
                                    size="sm"
                                    variant="soft"
                                    className={
                                      severity === "critical"
                                        ? "bg-red-500/20 text-red-300 border-red-500/30"
                                        : "bg-orange-500/20 text-orange-300 border-orange-500/30"
                                    }
                                  >
                                    {severity}
                                  </Chip>
                                )}
                              </div>
                              <div className="grid grid-cols-2 gap-1 text-[11px] font-mono text-muted-foreground">
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
                      <div className="rounded-xl border border-orange-500/30 bg-orange-500/10 backdrop-blur-md p-3 space-y-2">
                        <div className="flex items-center gap-2 text-xs text-orange-300">
                          <AlertTriangle className="size-3.5" />
                          Endpoint SLO/regression alerts ({endpointAlerts.length})
                        </div>
                        <div className="flex flex-wrap gap-2">
                          {endpointAlerts.slice(0, 6).map((alert, index) => (
                            <Chip
                              key={`${alert.alertKey}-${index}`}
                              size="sm"
                              variant="soft"
                              className={
                                alert.severity === "critical"
                                  ? "bg-red-500/20 text-red-300 border-red-500/30"
                                  : "bg-orange-500/20 text-orange-300 border-orange-500/30"
                              }
                            >
                              {alert.label}: {alert.reason}
                            </Chip>
                          ))}
                        </div>
                      </div>
                    )}

                    <div className="rounded-xl border border-white/5 bg-background/20 backdrop-blur-md p-3 space-y-2">
                      <div className="flex items-center justify-between">
                        <span className="text-xs uppercase tracking-wider text-muted-foreground">
                          Alert History
                        </span>
                        <span className="text-[11px] text-muted-foreground font-mono">
                          {isSyncingAlerts
                            ? "syncing..."
                            : `${persistedAlerts.length} ${alertHistoryStatus}`}
                        </span>
                      </div>
                      <div className="grid grid-cols-1 md:grid-cols-3 gap-2 items-end">
                        <Input
                          type="number"
                          placeholder="Retention Days"
                          value={String(alertRetention.days)}
                          onChange={(e) => {
                            const next = Number(e.target.value);
                            if (Number.isFinite(next)) {
                              setAlertRetention({ days: Math.max(1, Math.round(next)) });
                            }
                          }}
                        />
                        <Button
                          size="sm"
                          variant="secondary"
                          onPress={handleSaveAlertRetention}
                          isDisabled={
                            isSavingAlertRetention ||
                            !adminPermissions.canEditAlertRetention
                          }
                        >
                          {isSavingAlertRetention ? "Saving..." : "Save Retention"}
                        </Button>
                        <Button
                          size="sm"
                          variant="outline"
                          onPress={handleCleanupAlerts}
                          isDisabled={
                            isCleaningAlerts || !adminPermissions.canRunAlertCleanup
                          }
                        >
                          {isCleaningAlerts ? "Cleaning..." : "Run Cleanup"}
                        </Button>
                      </div>
                      {(!adminPermissions.canEditAlertRetention ||
                        !adminPermissions.canRunAlertCleanup) && (
                        <div className="text-[11px] text-muted-foreground">
                          Alert retention controls are limited by workspace policy.
                        </div>
                      )}
                      <div className="flex items-center gap-1.5">
                        {(["open", "acked", "resolved", "all"] as const).map(
                          (status) => (
                            <Button
                              key={status}
                              size="sm"
                              variant={
                                alertHistoryStatus === status
                                  ? "primary"
                                  : "ghost"
                              }
                              className="text-[11px] capitalize"
                              onPress={() => setAlertHistoryStatus(status)}
                            >
                              {status}
                            </Button>
                          ),
                        )}
                      </div>
                      {persistedAlerts.length === 0 ? (
                        <div className="text-xs text-muted-foreground">
                          No persisted alerts for this filter.
                        </div>
                      ) : (
                        <div className="space-y-2">
                          {persistedAlerts.slice(0, 8).map((alert) => (
                            <div
                              key={alert.id}
                              className="flex items-center justify-between gap-2 p-2 rounded-lg border border-white/5 bg-white/5"
                            >
                              <div className="min-w-0">
                                <div className="text-xs text-foreground truncate">
                                  {alert.reason}
                                </div>
                                <div className="text-[11px] text-muted-foreground font-mono truncate">
                                  {alert.source}:{alert.key}
                                </div>
                                {alert.ackedBy && (
                                  <div className="text-[10px] text-muted-foreground/80">
                                    acked by {alert.ackedBy}
                                  </div>
                                )}
                              </div>
                              {alert.status === "open" ? (
                                <Button
                                  size="sm"
                                  variant="ghost"
                                  className="text-xs"
                                  onPress={() => handleAckAlert(alert.id)}
                                  isDisabled={!adminPermissions.canAckAlerts}
                                >
                                  Ack
                                </Button>
                              ) : (
                                <Chip
                                  size="sm"
                                  variant="soft"
                                  className="text-[10px] uppercase tracking-wider"
                                >
                                  {alert.status}
                                </Chip>
                              )}
                            </div>
                          ))}
                        </div>
                      )}
                    </div>

                    {commandMetrics &&
                      (commandMetrics.progress.droppedEventsTotal > 0 ||
                        commandMetrics.progress.suppressedEventsTotal > 0) && (
                      <div className="rounded-xl border border-orange-500/30 bg-orange-500/10 backdrop-blur-md p-3">
                        <div className="flex items-center gap-2 text-xs text-orange-300">
                          <AlertTriangle className="size-3.5" />
                          Runtime telemetry backpressure detected for selected command.
                        </div>
                      </div>
                    )}

                    {recentCommands.length === 0 ? (
                      <div className="flex flex-col items-center justify-center py-10 rounded-2xl border border-dashed border-white/10 text-muted-foreground/50">
                        <span className="text-sm font-medium">
                          No commands yet
                        </span>
                      </div>
                    ) : (
                      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
                        <div className="rounded-xl border border-white/5 bg-background/20 backdrop-blur-md p-3 max-h-72 overflow-y-auto space-y-2">
                          {recentCommands.map((cmd) => (
                            <button
                              key={cmd.id}
                              type="button"
                              onClick={() => setSelectedCommandId(cmd.id)}
                              className={`w-full text-left p-3 rounded-lg border transition-all ${
                                selectedCommandId === cmd.id
                                  ? "border-primary/40 bg-primary/10"
                                  : "border-white/5 bg-white/5 hover:border-white/10"
                              }`}
                            >
                              <div className="flex items-center justify-between gap-2">
                                <span className="text-xs font-mono text-foreground/80 truncate">
                                  {cmd.intent}
                                </span>
                                <Chip size="sm" variant="soft">
                                  {cmd.status}
                                </Chip>
                              </div>
                              <div className="text-[11px] text-muted-foreground mt-1 font-mono">
                                {cmd.id}
                              </div>
                            </button>
                          ))}
                        </div>

                        <div className="rounded-xl border border-white/5 bg-background/20 backdrop-blur-md p-3 max-h-72 overflow-y-auto">
                          {selectedCommandId ? (
                            <div className="space-y-2">
                              {commandMetrics && (
                                <div className="p-2 rounded-md bg-white/5 border border-white/5 space-y-2">
                                  <div className="flex items-center justify-between gap-2">
                                    <span className="text-xs uppercase tracking-wider text-muted-foreground">
                                      Command Metrics
                                    </span>
                                    <Chip size="sm" variant="soft">
                                      {commandMetrics.status}
                                    </Chip>
                                  </div>
                                  <div className="grid grid-cols-3 gap-2 text-[11px] font-mono text-muted-foreground">
                                    <div>
                                      queue: {formatMs(commandMetrics.timings.queueDelayMs)}
                                    </div>
                                    <div>
                                      run: {formatMs(commandMetrics.timings.runDurationMs)}
                                    </div>
                                    <div>
                                      total: {formatMs(commandMetrics.timings.totalDurationMs)}
                                    </div>
                                  </div>
                                  <div className="flex flex-wrap gap-2">
                                    <Chip size="sm" variant="soft">
                                      events: {commandMetrics.progress.totalEvents}
                                    </Chip>
                                    <Chip size="sm" variant="soft">
                                      dropped: {commandMetrics.progress.droppedEventsTotal}
                                    </Chip>
                                    <Chip size="sm" variant="soft">
                                      suppressed:{" "}
                                      {commandMetrics.progress.suppressedEventsTotal}
                                    </Chip>
                                  </div>
                                </div>
                              )}
                              {isLoadingMetrics && !commandMetrics && (
                                <div className="text-sm text-muted-foreground">
                                  Loading metrics...
                                </div>
                              )}
                              {isLoadingProgress && commandProgress.length === 0 ? (
                                <div className="text-sm text-muted-foreground">
                                  Loading progress...
                                </div>
                              ) : commandProgress.length === 0 ? (
                                <div className="text-sm text-muted-foreground">
                                  No progress events yet.
                                </div>
                              ) : (
                                commandProgress.map((event) => (
                                  <div
                                    key={event.id}
                                    className="p-2 rounded-md bg-white/5 border border-white/5"
                                  >
                                    <div className="flex items-center justify-between gap-2">
                                      <span className="text-xs uppercase tracking-wider text-muted-foreground">
                                        {event.level}
                                      </span>
                                      <span className="text-[11px] text-muted-foreground font-mono">
                                        {new Date(event.createdAt).toLocaleTimeString()}
                                      </span>
                                    </div>
                                    <p className="text-sm text-foreground/90 mt-1">
                                      {event.message}
                                    </p>
                                    {event.data && (
                                      <pre className="text-[11px] text-muted-foreground mt-1 whitespace-pre-wrap font-mono">
                                        {JSON.stringify(event.data, null, 2)}
                                      </pre>
                                    )}
                                  </div>
                                ))
                              )}
                            </div>
                          ) : (
                            <div className="text-sm text-muted-foreground">
                              Select a command to inspect progress.
                            </div>
                          )}
                        </div>
                      </div>
                    )}
                  </div>

                  {/* Airlock Section - Clean Alerts */}
                  <div className="space-y-6">
                    <h3 className="text-lg font-semibold flex items-center gap-2">
                      <Shield className="size-4 text-orange-500" />
                      Airlock Monitor
                    </h3>

                    {pendingApprovals.length === 0 ? (
                      <div className="flex flex-col items-center justify-center py-10 rounded-2xl border border-dashed border-white/10 text-muted-foreground/50">
                        <CheckCircle2 className="size-8 mb-2 opacity-20" />
                        <span className="text-sm font-medium">
                          Cortex Secure
                        </span>
                      </div>
                    ) : (
                      <div className="grid grid-cols-1 gap-3">
                        {pendingApprovals.map((request) => (
                          <div
                            key={request.commandId}
                            className="flex items-center justify-between p-4 rounded-xl bg-white/5 border border-white/5 hover:border-white/10 transition-all"
                          >
                            <div className="flex items-center gap-4">
                              <Chip
                                size="sm"
                                className={
                                  request.airlockLevel === AirlockLevels.Dangerous
                                    ? "bg-red-500/20 text-red-400 border-red-500/20"
                                    : request.airlockLevel === AirlockLevels.Sensitive
                                      ? "bg-orange-500/20 text-orange-400 border-orange-500/20"
                                      : "bg-green-500/20 text-green-400 border-green-500/20"
                                }
                                variant="soft"
                              >
                                {request.intent}
                              </Chip>
                              <code className="text-xs text-muted-foreground font-mono bg-black/20 px-2 py-1 rounded">
                                {request.payloadSummary.slice(0, 60)}
                                ...
                              </code>
                            </div>

                            <div className="flex gap-2">
                              <Button
                                variant="ghost"
                                size="sm"
                                isIconOnly
                                className="text-green-500 hover:bg-green-500/10"
                                onPress={() =>
                                  handleAirlockRespond(request.commandId, true)
                                }
                              >
                                <CheckCircle2 className="size-4" />
                              </Button>
                              <Button
                                variant="ghost"
                                size="sm"
                                isIconOnly
                                className="text-red-500 hover:bg-red-500/10"
                                onPress={() =>
                                  handleAirlockRespond(request.commandId, false)
                                }
                              >
                                <XCircle className="size-4" />
                              </Button>
                            </div>
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                </div>
              )}
            </div>
          )}

          {/* CREATE AGENT MODAL - Floating & Blurry */}
          <Modal isOpen={isCreatingAgent} onOpenChange={setIsCreatingAgent}>
            <Modal.Backdrop className="backdrop-blur-2xl bg-white/60 dark:bg-background/20">
              <Modal.Container>
                <Modal.Dialog className="bg-background/30 border border-white/10 max-w-2xl w-full rounded-3xl relative z-[100]">
                  <Modal.Header className="px-8 pt-8 pb-4 border-b border-white/5">
                    <div className="flex items-center gap-4">
                      <div className="size-10 rounded-xl bg-primary/10 flex items-center justify-center text-primary">
                        <Sparkles className="size-5" />
                      </div>
                      <div>
                        <Modal.Heading className="text-xl font-bold tracking-tight text-foreground">
                          Deploy Cloud Agent
                        </Modal.Heading>
                        <p className="text-xs text-muted-foreground font-medium uppercase tracking-widest mt-2">
                          New Instance
                        </p>
                      </div>
                    </div>
                  </Modal.Header>
                  <Modal.Body className="p-8 relative z-[101]">
                    <CreateAgentForm
                      onSuccess={() => {
                        setIsCreatingAgent(false);
                        setAgentsRefreshToken((prev) => prev + 1);
                      }}
                      onCancel={() => setIsCreatingAgent(false)}
                    />
                  </Modal.Body>
                </Modal.Dialog>
              </Modal.Container>
            </Modal.Backdrop>
          </Modal>
        </div>
      </div>
    </div>
  );
}
