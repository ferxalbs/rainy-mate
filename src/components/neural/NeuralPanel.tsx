import { Button, Card, Chip, Separator, Switch } from "@heroui/react";
import {
  Network,
  RefreshCw,
  Shield,
  CheckCircle2,
  XCircle,
  Smartphone,
  Bot,
} from "lucide-react";
import { useEffect, useState } from "react";
import { toast } from "@heroui/react";
import {
  bootstrapAtm,
  generatePairingCode,
  setNeuralCredentials,
  setNeuralWorkspaceId,
  loadNeuralCredentials,
  registerNode,
  respondToAirlock,
  getPendingAirlockApprovals,
  setHeadlessMode,
  ApprovalRequest,
  WorkspaceAuth,
  SkillManifest,
  getNeuralCredentialsValues,
  AirlockLevels,
} from "../../services/tauri";
import { AgentList } from "./AgentList";
import { CreateAgentForm } from "./CreateAgentForm";

// Default skills this Desktop Node exposes
const DEFAULT_SKILLS: SkillManifest[] = [
  {
    name: "file_ops",
    version: "1.0.0",
    methods: [
      {
        name: "read_file",
        description: "Read file content",
        airlockLevel: AirlockLevels.Safe,
        parameters: {
          path: {
            type: "string",
            description: "Absolute path to file",
            required: true,
          },
        },
      },
      {
        name: "write_file",
        description: "Write content to file",
        airlockLevel: AirlockLevels.Sensitive,
        parameters: {
          path: {
            type: "string",
            description: "Absolute path to file",
            required: true,
          },
          content: {
            type: "string",
            description: "Content to write",
            required: true,
          },
        },
      },
    ],
  },
  {
    name: "terminal",
    version: "1.0.0",
    methods: [
      {
        name: "exec",
        description: "Execute terminal command",
        airlockLevel: AirlockLevels.Dangerous,
        parameters: {
          command: {
            type: "string",
            description: "Command to execute",
            required: true,
          },
          cwd: {
            type: "string",
            description: "Working directory",
            required: false,
          },
        },
      },
    ],
  },
];
type ConnectionStatus = "idle" | "loading" | "connected" | "error";

export function NeuralPanel() {
  // Connection State
  const [status, setStatus] = useState<ConnectionStatus>("idle");
  const [workspace, setWorkspace] = useState<WorkspaceAuth | null>(null);
  const [error, setError] = useState<string>("");

  // Form State
  const [platformKey, setPlatformKey] = useState("");
  const [userApiKey, setUserApiKey] = useState("");
  const [workspaceName, setWorkspaceName] = useState("");

  // Pairing State
  const [pairingCode, setPairingCode] = useState<string | null>(null);
  const [pairingExpires, setPairingExpires] = useState<number | null>(null);

  // Agent Management
  const [isCreatingAgent, setIsCreatingAgent] = useState(false);

  // Settings
  const [isHeadless, setIsHeadless] = useState(false);

  // Airlock Approvals
  const [pendingApprovals, setPendingApprovals] = useState<ApprovalRequest[]>(
    [],
  );

  // Load existing credentials on mount
  useEffect(() => {
    const init = async () => {
      try {
        const hasCredentials = await loadNeuralCredentials();
        if (hasCredentials) {
          console.log("Credentials loaded from Keychain");
          const creds = await getNeuralCredentialsValues();
          if (creds) {
            const [pk, uk] = creds;
            setPlatformKey(pk);
            setUserApiKey(uk);
            toast.success("Credentials restored");
          }
        }
      } catch (err) {
        console.error("Failed to load credentials:", err);
      }

      // Load pending approvals
      try {
        const approvals = await getPendingAirlockApprovals();
        setPendingApprovals(approvals);
      } catch (err) {
        console.error("Failed to load approvals:", err);
      }
    };
    init();
  }, []);

  // Main connection handler - full connection flow
  const handleConnect = async () => {
    if (!platformKey.trim() || !userApiKey.trim()) {
      toast.danger("Both Platform Key and User API Key are required");
      return;
    }

    setStatus("loading");
    setError("");

    try {
      // Step 1: Bootstrap workspace via ATM (creates or retrieves workspace)
      console.log("Step 1: Bootstrapping workspace...");
      const ws = await bootstrapAtm(
        platformKey,
        userApiKey,
        workspaceName.trim() || "Desktop Workspace",
      );
      console.log("Workspace created/retrieved:", ws);

      // Step 2: Store credentials for future use (also saves to Keychain)
      console.log("Step 2: Storing credentials...");
      await setNeuralCredentials(platformKey, userApiKey);

      // Step 3: Set workspace ID for NeuralService
      console.log("Step 3: Setting workspace ID...");
      await setNeuralWorkspaceId(ws.id);

      // Step 4: Register this Desktop Node with the Cloud Cortex
      // Note: allowedPaths will be provided per-command by the Cloud
      console.log("Step 4: Registering desktop node...");
      const nodeId = await registerNode(DEFAULT_SKILLS, []);
      console.log("Node registered with ID:", nodeId);

      // Success!
      setWorkspace(ws);
      setStatus("connected");
      toast.success(`Connected to Cloud Cortex! Node: ${nodeId}`);
    } catch (err: unknown) {
      console.error("Connection failed:", err);
      const errorMsg = err instanceof Error ? err.message : JSON.stringify(err);
      setError(errorMsg);
      setStatus("error");

      toast.danger("Connection failed. Check your credentials.");
    }
  };

  // Generate pairing code for mobile
  const handleGeneratePairingCode = async () => {
    try {
      const res = await generatePairingCode();
      setPairingCode(res.code);
      setPairingExpires(res.expiresAt);
    } catch (err: unknown) {
      const errorMsg = err instanceof Error ? err.message : JSON.stringify(err);
      setError(errorMsg);
      toast.danger("Failed to generate pairing code");
    }
  };

  // Headless mode toggle
  const handleToggleHeadless = async (enabled: boolean) => {
    try {
      await setHeadlessMode(enabled);
      setIsHeadless(enabled);
      toast.success(`Headless Mode ${enabled ? "Enabled" : "Disabled"}`);
    } catch (err) {
      console.error("Failed to toggle headless mode:", err);
      toast.danger("Failed to update settings");
    }
  };

  // Airlock response
  const handleAirlockRespond = async (requestId: string, approved: boolean) => {
    try {
      await respondToAirlock(requestId, approved);
      setPendingApprovals((prev) => prev.filter((req) => req.id !== requestId));
      toast.success(approved ? "Request Approved" : "Request Denied");
    } catch (err) {
      console.error("Failed to respond to airlock:", err);
      toast.danger("Failed to process response");
    }
  };

  const getStatusColor = () => {
    switch (status) {
      case "connected":
        return "text-green-500";
      case "loading":
        return "text-yellow-500";
      case "error":
        return "text-red-500";
      default:
        return "text-gray-500";
    }
  };

  const getStatusLabel = () => {
    switch (status) {
      case "connected":
        return "Connected";
      case "loading":
        return "Connecting...";
      case "error":
        return "Error";
      default:
        return "Disconnected";
    }
  };

  return (
    <div className="flex flex-col gap-6 p-6 max-w-2xl mx-auto">
      {/* Header */}
      <div className="flex items-center gap-3">
        <Network className="size-8 text-purple-500" />
        <div>
          <h1 className="text-2xl font-bold">Neural Link</h1>
          <p className="text-muted-foreground text-sm">
            Connect your desktop to the Cloud Cortex
          </p>
        </div>
      </div>

      {/* Main Connection Card */}
      <Card className="p-6">
        {status === "connected" && workspace ? (
          // SUCCESS STATE - Like legacy AtmBootstrap
          <div className="space-y-6">
            {/* Success Banner */}
            <div className="bg-green-50 dark:bg-green-900/20 text-green-700 dark:text-green-400 p-4 rounded-lg">
              <h3 className="font-bold flex items-center gap-2">
                <CheckCircle2 className="size-5" />
                Workspace Connected!
              </h3>
              <div className="mt-2 text-sm font-mono whitespace-pre-wrap bg-green-100/50 dark:bg-green-900/30 p-2 rounded">
                {JSON.stringify(workspace, null, 2)}
              </div>
              <p className="mt-3 text-xs opacity-70">
                API Key has been automatically saved to your session.
              </p>
            </div>

            <Separator />

            {/* Agent Management */}
            <div className="space-y-3">
              <div className="flex items-center gap-2">
                <Bot className="size-5 text-purple-500" />
                <h3 className="font-bold text-lg">Cloud Agents</h3>
              </div>

              {isCreatingAgent ? (
                <CreateAgentForm
                  onSuccess={() => setIsCreatingAgent(false)}
                  onCancel={() => setIsCreatingAgent(false)}
                />
              ) : (
                <AgentList onCreateClick={() => setIsCreatingAgent(true)} />
              )}
            </div>

            <Separator />

            {/* Mobile Pairing */}
            <div className="space-y-3">
              <div className="flex items-center gap-2">
                <Smartphone className="size-5 text-blue-500" />
                <h3 className="font-bold text-lg">Connect Mobile</h3>
              </div>
              <p className="text-sm text-muted-foreground">
                Control this workspace from Telegram or WhatsApp.
              </p>

              {pairingCode ? (
                <div className="bg-default-50 dark:bg-default-900/50 p-6 rounded-xl flex flex-col items-center justify-center text-center space-y-4 border border-default-200">
                  <div className="text-4xl font-mono font-bold tracking-[0.2em] text-primary">
                    {pairingCode}
                  </div>
                  <div className="text-sm text-muted-foreground max-w-xs">
                    Send this code to <b>@RainyMateBot</b> on Telegram or
                    WhatsApp to pair your device.
                  </div>
                  <div className="text-xs text-muted-foreground">
                    Expires at{" "}
                    {pairingExpires
                      ? new Date(pairingExpires).toLocaleTimeString()
                      : ""}{" "}
                    (15 mins)
                  </div>
                </div>
              ) : (
                <Button variant="ghost" onPress={handleGeneratePairingCode}>
                  Generate Pairing Code
                </Button>
              )}
            </div>

            <Separator />

            {/* Disconnect Section */}
            <div className="bg-red-50/50 dark:bg-red-900/10 border border-red-200 dark:border-red-900/50 p-4 rounded-lg">
              <div className="flex items-center justify-between gap-4">
                <div>
                  <h4 className="font-semibold text-red-600 dark:text-red-400">
                    Disconnect Workspace
                  </h4>
                  <p className="text-sm text-muted-foreground">
                    Delete workspace from server and reset all credentials
                  </p>
                </div>
                <Button
                  variant="danger"
                  onPress={async () => {
                    if (
                      confirm(
                        "⚠️ This will permanently delete your workspace from the server. Are you sure?",
                      )
                    ) {
                      try {
                        // Get stored credentials from Keychain
                        const {
                          getNeuralCredentialsValues,
                          resetNeuralWorkspace,
                        } = await import("../../services/tauri");
                        const creds = await getNeuralCredentialsValues();

                        if (!creds || !creds[0] || !creds[1]) {
                          toast.danger("No stored credentials found");
                          return;
                        }

                        await resetNeuralWorkspace(creds[0], creds[1]);
                        setPlatformKey("");
                        setUserApiKey("");
                        setWorkspace(null);
                        setStatus("idle");
                        setPairingCode(null);
                        toast.success("Workspace deleted successfully!");
                      } catch (e: any) {
                        console.error("Failed to reset workspace:", e);
                        toast.danger(e?.message || "Failed to reset workspace");
                      }
                    }
                  }}
                >
                  Delete Workspace
                </Button>
              </div>
            </div>
          </div>
        ) : (
          // FORM STATE - For connection
          <div className="space-y-6">
            {/* Status Indicator */}
            <div className="flex items-center gap-3">
              <div
                className={`p-2 rounded-full ${getStatusColor().replace("text-", "bg-")}/10`}
              >
                <Network className={`size-6 ${getStatusColor()}`} />
              </div>
              <div>
                <h3 className="text-lg font-semibold">Neural Link</h3>
                <div className="flex items-center gap-2">
                  <span
                    className={`size-2 rounded-full ${getStatusColor().replace("text-", "bg-")} ${status === "loading" ? "animate-pulse" : ""}`}
                  />
                  <span className="text-sm text-muted-foreground">
                    {getStatusLabel()}
                  </span>
                </div>
              </div>
            </div>

            {/* Connection Form */}
            <div className="flex flex-col gap-4">
              <div className="flex flex-col gap-1">
                <label className="text-sm font-medium">
                  Rainy Platform API Key
                </label>
                <input
                  type="password"
                  placeholder="rk_live_..."
                  className="px-3 py-2 rounded-lg border bg-background/50 text-sm font-mono"
                  value={platformKey}
                  onChange={(e) => setPlatformKey(e.target.value)}
                />
                <p className="text-xs text-muted-foreground">
                  Get your key at platform.rainymate.com
                </p>
              </div>

              <div className="flex flex-col gap-1">
                <label className="text-sm font-medium">
                  User API Key (Creator Key)
                </label>
                <input
                  type="password"
                  placeholder="rny_..."
                  className="px-3 py-2 rounded-lg border bg-background/50 text-sm font-mono"
                  value={userApiKey}
                  onChange={(e) => setUserApiKey(e.target.value)}
                />
                <p className="text-xs text-muted-foreground">
                  Required for Premium Agents (Gemini 3 Pro)
                </p>
              </div>

              <div className="flex flex-col gap-1">
                <label className="text-sm font-medium">Workspace Name</label>
                <input
                  type="text"
                  placeholder="e.g. My Agency"
                  className="px-3 py-2 rounded-lg border bg-background/50 text-sm"
                  value={workspaceName}
                  onChange={(e) => setWorkspaceName(e.target.value)}
                />
              </div>

              {/* Error Display - Like legacy */}
              {status === "error" && error && (
                <div className="bg-red-50 dark:bg-red-900/20 text-red-600 dark:text-red-400 p-3 rounded-lg text-sm">
                  🚨 Error: {error}
                </div>
              )}
            </div>

            {/* Connect Button */}
            <div className="flex justify-end">
              <Button
                variant="primary"
                isDisabled={
                  !platformKey.trim() ||
                  !userApiKey.trim() ||
                  status === "loading"
                }
                onPress={handleConnect}
              >
                {status === "loading" ? (
                  <>
                    <RefreshCw className="size-4 animate-spin mr-2" />
                    Connecting...
                  </>
                ) : (
                  "Connect to Cloud Cortex"
                )}
              </Button>
            </div>
          </div>
        )}
      </Card>

      {/* Settings Card */}
      <Card className="p-6">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-4">
            <div className="p-2 bg-purple-500/10 rounded-lg text-purple-500">
              <Shield className="size-5" />
            </div>
            <div>
              <h3 className="font-semibold">Headless Mode</h3>
              <p className="text-sm text-muted-foreground">
                Auto-approve sensitive commands
              </p>
            </div>
          </div>
          <Switch isSelected={isHeadless} onChange={handleToggleHeadless}>
            <Switch.Control className="bg-default-200 data-[selected=true]:bg-purple-500">
              <Switch.Thumb />
            </Switch.Control>
          </Switch>
        </div>
      </Card>

      <Separator />

      {/* Danger Zone */}
      <Card className="p-6 border-red-200 dark:border-red-900/50 bg-red-50/50 dark:bg-red-900/10">
        <div className="flex items-center justify-between">
          <div>
            <h3 className="font-semibold text-red-600 dark:text-red-400">
              Danger Zone
            </h3>
            <p className="text-sm text-muted-foreground">
              Reset connection and clear stored credentials
            </p>
          </div>
          <Button
            variant="danger"
            isDisabled={!platformKey.trim() || !userApiKey.trim()}
            onPress={async () => {
              if (
                confirm(
                  "⚠️ This will permanently delete your workspace from the server and clear all credentials. Are you sure?",
                )
              ) {
                try {
                  const { resetNeuralWorkspace } =
                    await import("../../services/tauri");
                  await resetNeuralWorkspace(platformKey, userApiKey);
                  setPlatformKey("");
                  setUserApiKey("");
                  setWorkspace(null);
                  setStatus("idle");
                  setPairingCode(null);
                  toast.success(
                    "Workspace deleted. You can now create a new one.",
                  );
                } catch (e: any) {
                  console.error("Failed to reset workspace:", e);
                  toast.danger(e?.message || "Failed to reset workspace");
                }
              }
            }}
          >
            Delete Workspace
          </Button>
        </div>
      </Card>

      {/* Pending Approvals */}
      <div>
        <div className="flex items-center gap-2 mb-4">
          <Shield className="size-5 text-orange-500" />
          <h2 className="text-lg font-semibold">Security Approvals</h2>
          {pendingApprovals.length > 0 && (
            <Chip color="warning" size="sm">
              {pendingApprovals.length}
            </Chip>
          )}
        </div>

        {pendingApprovals.length === 0 ? (
          <Card className="p-6 text-center text-muted-foreground">
            <CheckCircle2 className="size-8 mx-auto mb-2 text-green-500" />
            <p>No pending approvals</p>
            <p className="text-sm mt-1">
              Commands from the Cloud Cortex will appear here for review.
            </p>
          </Card>
        ) : (
          <div className="flex flex-col gap-3">
            {pendingApprovals.map((request) => (
              <Card key={request.id} className="p-4">
                <div className="flex items-start justify-between">
                  <div className="flex-1">
                    <div className="flex items-center gap-2">
                      <Chip
                        color={
                          request.level === AirlockLevels.Dangerous
                            ? "danger"
                            : request.level === AirlockLevels.Sensitive
                              ? "warning"
                              : "success"
                        }
                        size="sm"
                      >
                        {request.level === AirlockLevels.Dangerous
                          ? "Dangerous"
                          : request.level === AirlockLevels.Sensitive
                            ? "Sensitive"
                            : "Safe"}
                      </Chip>
                      <span className="font-medium">
                        {request.command_type}
                      </span>
                    </div>
                    <p className="text-sm text-muted-foreground mt-2 font-mono">
                      {JSON.stringify(request.payload, null, 2).slice(0, 100)}
                      {JSON.stringify(request.payload).length > 100 && "..."}
                    </p>
                    <p className="text-xs text-muted-foreground mt-2">
                      {new Date(request.timestamp).toLocaleString()}
                    </p>
                  </div>

                  <div className="flex gap-2 ml-4">
                    <Button
                      variant="secondary"
                      size="sm"
                      className="bg-green-600 hover:bg-green-700 text-white"
                      onPress={() => handleAirlockRespond(request.id, true)}
                    >
                      <CheckCircle2 className="size-4" />
                    </Button>
                    <Button
                      variant="danger"
                      size="sm"
                      onPress={() => handleAirlockRespond(request.id, false)}
                    >
                      <XCircle className="size-4" />
                    </Button>
                  </div>
                </div>
              </Card>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
