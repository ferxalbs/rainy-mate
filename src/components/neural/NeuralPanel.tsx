import { Button } from "@heroui/react";
import { Shield, Sparkles } from "lucide-react";
import { useEffect, useState } from "react";
import { toast } from "sonner";
import {
  bootstrapAtm,
  getNeuralCredentialsValues,
  hasAtmCredentials,
  loadNeuralCredentials,
  registerNode,
  setNeuralCredentials,
  setNeuralWorkspaceId,
  WorkspaceAuth,
} from "../../services/tauri";
import { DEFAULT_NEURAL_SKILLS } from "../../constants/defaultNeuralSkills";
import { NeuralLayout } from "./layout/NeuralLayout";
import { NeuralSidebar } from "./layout/NeuralSidebar";
import { NeuralActivity } from "./modules/NeuralActivity";
import { NeuralAgents } from "./modules/NeuralAgents";
import { NeuralDashboard } from "./modules/NeuralDashboard";
// import { NeuralHealth } from "./modules/NeuralHealth"; // @TODO: Remove
// import { NeuralSettings } from "./modules/NeuralSettings"; // @TODO: Remove

type NeuralState = "idle" | "restored" | "connected" | "connecting";

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
  const [isHeadless, setIsHeadless] = useState(false);
  const [activeTab, setActiveTab] = useState("dashboard");

  // Styles for native inputs in login form
  const loginInputClass =
    "w-full bg-background/40 backdrop-blur-sm border border-white/5 hover:border-white/10 focus:border-primary/50 transition-colors h-12 rounded-xl px-4 text-sm text-foreground placeholder:text-muted-foreground/50 focus:outline-none focus:ring-1 focus:ring-primary/20";
  const labelClass =
    "block uppercase text-[10px] font-bold tracking-widest text-muted-foreground mb-1.5 ml-1";

  useEffect(() => {
    let cancelled = false;
    const init = async () => {
      let atmKeyPresent = false;
      try {
        atmKeyPresent = await hasAtmCredentials();
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
                writeStoredWorkspace(ws);
                effectiveWorkspace = ws;
              } catch (err) {
                console.error("Failed to restore ATM admin key:", err);
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
      setState("connected");
      toast.success(`Neural Link Established! Welcome to ${ws.name}`);
    } catch (err: any) {
      console.error("Connection failed:", err);
      setState("idle");
      toast.error("Connection failed. Please check your credentials.");
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
        clearStoredWorkspace();
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
              <div className="flex justify-center mb-4">
                <div className="size-16 rounded-2xl bg-primary/10 flex items-center justify-center text-primary shadow-lg shadow-primary/20">
                  <Shield className="size-8" />
                </div>
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
                  className="w-full font-bold shadow-lg shadow-primary/20 bg-primary text-primary-foreground hover:bg-primary/90"
                  onPress={handleConnect}
                >
                  Restore Connection
                </Button>
                <Button
                  className="w-full text-muted-foreground hover:text-foreground bg-transparent hover:bg-foreground/5"
                  onPress={() => {
                    setState("idle");
                    clearStoredWorkspace();
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
                  <input
                    type="password"
                    placeholder="rk_live_..."
                    value={platformKey}
                    onChange={(e) => setPlatformKey(e.target.value)}
                    className={loginInputClass}
                  />
                  <p className="text-xs mt-1.5 ml-1 text-muted-foreground/60">
                    Available at platform.rainymate.com
                  </p>
                </div>

                <div>
                  <label className={labelClass}>Creator API Key</label>
                  <input
                    type="password"
                    placeholder="rny_..."
                    value={userApiKey}
                    onChange={(e) => setUserApiKey(e.target.value)}
                    className={loginInputClass}
                  />
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
                  className="w-full font-bold shadow-lg shadow-primary/20 mt-2 bg-primary text-primary-foreground hover:bg-primary/90"
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
              <span className="w-1.5 h-1.5 rounded-full bg-emerald-500 shadow-[0_0_8px_rgba(16,185,129,0.4)]" />
              <span className="text-xs text-muted-foreground font-mono">
                Connected
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
          isHeadless={isHeadless}
          onToggleHeadless={setIsHeadless}
        />
      )}
      {activeTab === "agents" && <NeuralAgents />}
      {activeTab === "activity" && <NeuralActivity />}
      {/* @TODO: Remove in next version - Legacy panels */}
      {/* {activeTab === "health" && <NeuralHealth />}
      {activeTab === "settings" && (
        <NeuralSettings platformKey={platformKey} userApiKey={userApiKey} />
      )} */}
    </NeuralLayout>
  );
}
