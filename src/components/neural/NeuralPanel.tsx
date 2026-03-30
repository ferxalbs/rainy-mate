import { Button } from "@heroui/react";
import { Shield, Sparkles, ExternalLink, Eye, EyeOff } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import {
  bootstrapAtm,
  classifyNeuralError,
  ensureDefaultAtmAgent,
  getAtmFleetStatus,
  getNeuralCredentialsValues,
  hasAtmCredentials,
  loadNeuralCredentials,
  clearNeuralCredentials,
  registerNode,
  resumeNeuralRuntime,
  resetNeuralWorkspace,
  setNeuralCredentials,
  setNeuralWorkspaceId,
} from "../../services/tauri";
import type { WorkspaceAuth } from "../../services/tauri";
import { NeuralLayout } from "./layout/NeuralLayout";
import { NeuralSidebar } from "./layout/NeuralSidebar";
import { NeuralActivity } from "./modules/NeuralActivity";
import { NeuralAgents } from "./modules/NeuralAgents";
import { NeuralDashboard } from "./modules/NeuralDashboard";
import { FleetCommandCenter } from "./modules/FleetCommandCenter";
import { NeuralMcp } from "./modules/NeuralMcp";

type NeuralState = "idle" | "restored" | "connected" | "connecting";

interface NeuralPanelProps {
  onNavigate?: (section: string) => void;
}


export function NeuralPanel({ onNavigate }: NeuralPanelProps) {
  const [state, setState] = useState<NeuralState>("idle");
  const [workspace, setWorkspace] = useState<WorkspaceAuth | null>(null);
  const [platformKey, setPlatformKey] = useState("");
  const [userApiKey, setUserApiKey] = useState("");
  const [workspaceName, setWorkspaceName] = useState("");
  const [activeTab, setActiveTab] = useState("dashboard");
  const [showPlatformKey, setShowPlatformKey] = useState(false);
  const [showUserKey, setShowUserKey] = useState(false);
  const [nodeReady, setNodeReady] = useState(false);
  const [nodeStatusLabel, setNodeStatusLabel] = useState(
    "Waiting for desktop heartbeat...",
  );
  // AbortController for the connect flow so polling stops if component unmounts
  const connectAbortRef = useRef<AbortController | null>(null);

  // Styles for native inputs in login form
  const loginInputClass =
    "w-full bg-background/40 backdrop-blur-sm border border-white/5 hover:border-white/10 focus:border-primary/50 transition-colors h-12 rounded-xl px-4 text-sm text-foreground placeholder:text-muted-foreground/50 focus:outline-none focus:ring-1 focus:ring-primary/20";
  const labelClass =
    "block uppercase text-[10px] font-bold tracking-widest text-muted-foreground mb-1.5 ml-1";

  const ensureWorkspaceDefaultAgent = async (silent = false) => {
    try {
      const result = await ensureDefaultAtmAgent();
      if (!silent && result.status === "created") {
        toast.success("Default cloud agent is ready for remote access");
      }
      return result;
    } catch (error) {
      console.error("Failed to ensure default ATM agent:", error);
      const message =
        error instanceof Error
          ? error.message
          : typeof error === "string"
            ? error
            : JSON.stringify(error);
      if (!silent) {
        toast.error(
          message?.trim().length
            ? `Default cloud agent provisioning failed: ${message}`
            : "Workspace connected, but default cloud agent provisioning failed.",
        );
      }
      return null;
    }
  };

  const waitForOnlineNode = async (workspaceId: string, signal: AbortSignal) => {
    if (signal.aborted) return false;
    setNodeReady(false);
    setNodeStatusLabel("Waiting for desktop heartbeat...");

    const deadline = Date.now() + 20_000;
    while (Date.now() < deadline) {
      if (signal.aborted) return false;
      try {
        const fleet = await getAtmFleetStatus();
        if (signal.aborted) return false;
        const onlineNode = fleet.nodes.find(
          (node) =>
            node.effectiveStatus === "online" || node.effectiveStatus === "busy",
        );
        if (fleet.workspaceId === workspaceId && onlineNode) {
          setNodeReady(true);
          setNodeStatusLabel("Desktop node is online.");
          return true;
        }
      } catch (error) {
        if (signal.aborted) return false;
        console.error("Failed to verify Bridge fleet readiness:", error);
      }

      await new Promise((resolve) => setTimeout(resolve, 1000));
    }

    if (signal.aborted) return false;
    setNodeReady(false);
    setNodeStatusLabel("Desktop node is still syncing with MaTE Bridge.");
    return false;
  };

  useEffect(() => {
    const abortController = new AbortController();
    const { signal } = abortController;
    let cancelled = false;
    const init = async () => {
      try {
        await hasAtmCredentials();
      } catch (err) {
        console.error("Failed to check Bridge admin key:", err);
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

            let effectiveWorkspace: WorkspaceAuth | null = null;

            // Always refresh canonical workspace identity from the Bridge.
            // Workspace name defaults to "Desktop Workspace" — the Bridge returns the real name.
            try {
              const ws = await bootstrapAtm(
                platform,
                userKey,
                "Desktop Workspace",
              );
              effectiveWorkspace = ws;
            } catch (err) {
              console.error("Failed to restore Bridge admin key:", err);
              // Do not auto-fallback to a stale local workspace id, as it can split
              // admin workspace and node workspace contexts.
            }

            if (effectiveWorkspace) {
              setWorkspace(effectiveWorkspace);
              setState("connecting");
              await ensureWorkspaceDefaultAgent(true);
              try {
                await resumeNeuralRuntime();
                await setNeuralWorkspaceId(effectiveWorkspace.id);
                try {
                  await registerNode();
                } catch (registerErr) {
                  // The background poller may be racing to re-register after auth-context sync.
                  await new Promise((resolve) => setTimeout(resolve, 500));
                  await registerNode();
                }
                await waitForOnlineNode(effectiveWorkspace.id, signal);
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
      abortController.abort();
      connectAbortRef.current?.abort();
    };
  }, []);

  const handleConnect = async () => {
    if (!platformKey.trim() || !userApiKey.trim()) {
      toast.error("Credentials are required");
      return;
    }

    const ac = new AbortController();
    connectAbortRef.current?.abort();
    connectAbortRef.current = ac;

    setState("connecting");

    try {
      const ws = await bootstrapAtm(
        platformKey,
        userApiKey,
        workspaceName.trim() || "Desktop Workspace",
      );
      await setNeuralCredentials(platformKey, userApiKey);
      await ensureWorkspaceDefaultAgent(false);
      await resumeNeuralRuntime();
      await setNeuralWorkspaceId(ws.id);
      try {
        await registerNode();
      } catch {
        // Retry once to tolerate startup race with CommandPoller auth-context sync.
        await new Promise((resolve) => setTimeout(resolve, 500));
        await registerNode();
      }
      await waitForOnlineNode(ws.id, ac.signal);
      if (ac.signal.aborted) return;

      setWorkspace(ws);
      setState("connected");
      toast.success(`Neural Link Established! Welcome to ${ws.name}`);
    } catch (err: any) {
      if (ac.signal.aborted) return;
      console.error("Connection failed:", err);
      setState("idle");
      const errorText =
        err instanceof Error
          ? err.message
          : typeof err === "string"
            ? err
            : JSON.stringify(err);
      void classifyNeuralError(errorText).then((msg) => toast.error(msg));
    }
  };

  const handleLogout = async () => {
    if (
      confirm(
        "⚠️ This will disconnect you from the Cloud Cortex. Are you sure?",
      )
    ) {
      try {
        await resetNeuralWorkspace(platformKey, userApiKey);
        // Explicit frontend credential cleanup for defense-in-depth
        await clearNeuralCredentials().catch(() => {});
        setPlatformKey("");
        setUserApiKey("");
        setNodeReady(false);
        setNodeStatusLabel("Waiting for desktop heartbeat...");
        setWorkspace(null);
        setState("idle");
        toast.success("Succesfully disconnected");
      } catch (e: any) {
        toast.error(e?.message || "Logout failed");
      }
    }
  };

  if (state !== "connected") {
    // Login / Restoration Logic
    return (
      <div className="h-full w-full relative bg-transparent overflow-hidden text-foreground">
        <div className="absolute inset-0 bg-gradient-to-b from-primary/5 via-background/50 to-background/80 pointer-events-none z-0" />
        <div className="absolute inset-0 overflow-y-auto w-full h-full scrollbar-none z-10 flex flex-col items-center justify-center p-6">
          <div className="w-full max-w-md space-y-8 animate-appear">
            <div className="text-center space-y-2">
              <div className="flex justify-center mb-6">
                <Shield className="size-12 text-primary animate-pulse-slow" />
              </div>
              <h1 className="text-3xl font-bold tracking-tight text-foreground">
                Neural Link
              </h1>
              <p className="text-muted-foreground text-sm">
                Connect your desktop node to Rainy Cloud Cortex.
              </p>
            </div>

            {state === "connecting" && (
              <div className="flex flex-col items-center justify-center py-12">
                <div className="size-10 border-2 border-primary/20 border-t-primary rounded-full animate-spin mb-4" />
                <span className="text-sm font-medium text-muted-foreground animate-pulse">
                  Establishing secure channel...
                </span>
              </div>
            )}

            {state === "restored" && (
              <div className="space-y-6">
                <div className="p-4 rounded-xl bg-primary/5 border border-primary/10 text-center">
                  <Sparkles className="size-5 text-primary mx-auto mb-2" />
                  <p className="text-sm text-foreground font-medium">
                    Previous session detected
                  </p>
                  <p className="text-xs text-muted-foreground mt-1">
                    Ready to restore connection to your workspace.
                  </p>
                </div>
                <Button
                  size="lg"
                  className="w-full font-bold bg-primary text-primary-foreground hover:bg-primary/90"
                  onPress={handleConnect}
                >
                  Restore Connection
                </Button>
                <Button
                  className="w-full text-muted-foreground hover:text-foreground bg-transparent hover:bg-foreground/5"
                  onPress={() => {
                    setState("idle");
                    setWorkspace(null);
                  }}
                >
                  Sign in with different keys
                </Button>
              </div>
            )}

            {state === "idle" && (
              <div className="space-y-6">
                <div>
                  <label className={labelClass}>Platform API Key</label>
                  <div className="relative">
                    <input
                      type={showPlatformKey ? "text" : "password"}
                      placeholder="rk_live_..."
                      value={platformKey}
                      onChange={(e) => setPlatformKey(e.target.value)}
                      className={`${loginInputClass} pr-10`}
                    />
                    <button
                      type="button"
                      onClick={() => setShowPlatformKey(!showPlatformKey)}
                      className="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground/50 hover:text-foreground transition-colors"
                    >
                      {showPlatformKey ? (
                        <EyeOff className="size-4" />
                      ) : (
                        <Eye className="size-4" />
                      )}
                    </button>
                  </div>
                  <a
                    href="https://app.rainy-mate.com"
                    target="_blank"
                    rel="noreferrer"
                    className="text-xs mt-2 ml-1 text-primary hover:underline flex items-center gap-1"
                  >
                    Get Platform Key at app.rainy-mate.com
                    <ExternalLink className="size-3" />
                  </a>
                </div>

                <div>
                  <label className={labelClass}>Creator API Key</label>
                  <div className="relative">
                    <input
                      type={showUserKey ? "text" : "password"}
                      placeholder="ra_..."
                      value={userApiKey}
                      onChange={(e) => setUserApiKey(e.target.value)}
                      className={`${loginInputClass} pr-10`}
                    />
                    <button
                      type="button"
                      onClick={() => setShowUserKey(!showUserKey)}
                      className="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground/50 hover:text-foreground transition-colors"
                    >
                      {showUserKey ? (
                        <EyeOff className="size-4" />
                      ) : (
                        <Eye className="size-4" />
                      )}
                    </button>
                  </div>
                  <a
                    href="https://app.rainy-mate.com"
                    target="_blank"
                    rel="noreferrer"
                    className="text-xs mt-2 ml-1 text-primary hover:underline flex items-center gap-1"
                  >
                    Get Creator Key at app.rainy-mate.com
                    <ExternalLink className="size-3" />
                  </a>
                </div>

                <div>
                  <label className={labelClass}>
                    Workspace Name (Optional)
                  </label>
                  <input
                    placeholder="e.g. Desktop Node"
                    value={workspaceName}
                    onChange={(e) => setWorkspaceName(e.target.value)}
                    className={loginInputClass}
                  />
                </div>

                <Button
                  size="lg"
                  className="w-full font-bold mt-2 bg-primary text-primary-foreground hover:bg-primary/90"
                  onPress={handleConnect}
                >
                  Connect Node
                </Button>
              </div>
            )}
          </div>
        </div>
      </div>
    );
  }

  // Connected State - 2 Panel Layout
  return (
    <NeuralLayout
      sidebar={
        <NeuralSidebar activeTab={activeTab} onTabChange={setActiveTab} />
      }
      headerContent={
        <div className="flex-1 flex items-center justify-between">
          <div className="flex items-center gap-3">
            <h2 className="text-lg font-bold text-foreground tracking-tight">
              {workspace?.name}
            </h2>
            <div className="flex items-center gap-1.5 px-2 py-0.5 rounded-full bg-foreground/5 border border-foreground/5">
              <span
                className={`w-1.5 h-1.5 rounded-full ${
                  nodeReady ? "bg-emerald-500" : "bg-amber-400"
                }`}
              />
              <span className="text-xs text-muted-foreground font-mono">
                {nodeReady ? "Connected" : "Syncing"}
              </span>
            </div>
          </div>
          <Button
            size="sm"
            onPress={handleLogout}
            className="bg-transparent hover:bg-red-500/10 text-red-400 hover:text-red-500"
          >
            Disconnect
          </Button>
        </div>
      }
    >
      {activeTab === "dashboard" && workspace && (
        <NeuralDashboard
          workspace={workspace}
          nodeReady={nodeReady}
          nodeStatusLabel={nodeStatusLabel}
        />
      )}
      {activeTab === "agents" && <NeuralAgents onNavigate={onNavigate} />}
      {activeTab === "activity" && <NeuralActivity />}
      {activeTab === "fleet" && <FleetCommandCenter />}
      {activeTab === "mcp" && <NeuralMcp />}
    </NeuralLayout>
  );
}
