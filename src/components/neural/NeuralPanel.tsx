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

type NeuralState = "idle" | "restored" | "connected" | "connecting";

export function NeuralPanel() {
  const [state, setState] = useState<NeuralState>("idle");
  const [workspace, setWorkspace] = useState<WorkspaceAuth | null>(null);

  const [platformKey, setPlatformKey] = useState("");
  const [userApiKey, setUserApiKey] = useState("");
  const [workspaceName, setWorkspaceName] = useState("");

  const [pairingCode, setPairingCode] = useState<string | null>(null);
  const [isCreatingAgent, setIsCreatingAgent] = useState(false);
  const [isHeadless, setIsHeadless] = useState(false);
  const [pendingApprovals, setPendingApprovals] = useState<ApprovalRequest[]>(
    [],
  );

  useEffect(() => {
    const init = async () => {
      try {
        const hasCredentials = await loadNeuralCredentials();
        if (hasCredentials) {
          const creds = await getNeuralCredentialsValues();
          if (creds) {
            setPlatformKey(creds[0]);
            setUserApiKey(creds[1]);
            setState("restored");
          }
        }
      } catch (err) {
        console.error("Failed to load credentials:", err);
      }

      try {
        const approvals = await getPendingAirlockApprovals();
        setPendingApprovals(approvals);
      } catch (err) {
        console.error("Failed to load approvals:", err);
      }
    };
    init();
  }, []);

  const handleConnect = async () => {
    if (!platformKey.trim() || !userApiKey.trim()) {
      toast.danger("Credentials are required");
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
      await registerNode(DEFAULT_SKILLS, []);

      setWorkspace(ws);
      setState("connected");
      toast.success(`Neural Link Established! Welcome to ${ws.name}`);
    } catch (err: any) {
      console.error("Connection failed:", err);
      setState("idle");
      toast.danger("Connection failed. Please check your credentials.");
    }
  };

  const handleGeneratePairingCode = async () => {
    try {
      const res = await generatePairingCode();
      setPairingCode(res.code);
    } catch (err) {
      toast.danger("Failed to generate pairing code");
    }
  };

  const handleToggleHeadless = async (enabled: boolean) => {
    try {
      await setHeadlessMode(enabled);
      setIsHeadless(enabled);
      toast.success(`Headless Mode ${enabled ? "Enabled" : "Disabled"}`);
    } catch (err) {
      toast.danger("Failed to update settings");
    }
  };

  const handleAirlockRespond = async (requestId: string, approved: boolean) => {
    try {
      await respondToAirlock(requestId, approved);
      setPendingApprovals((prev) => prev.filter((req) => req.id !== requestId));
      toast.success(approved ? "Request Approved" : "Request Denied");
    } catch (err) {
      toast.danger("Failed to process response");
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
        toast.success("Succesfully disconnected");
      } catch (e: any) {
        toast.danger(e?.message || "Logout failed");
      }
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
                  onPress={() => setState("idle")}
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
            <div className="animate-appear space-y-12">
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
                  <AgentList onCreateClick={() => setIsCreatingAgent(true)} />
                </div>
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
                    <span className="text-sm font-medium">Cortex Secure</span>
                  </div>
                ) : (
                  <div className="grid grid-cols-1 gap-3">
                    {pendingApprovals.map((request) => (
                      <div
                        key={request.id}
                        className="flex items-center justify-between p-4 rounded-xl bg-white/5 border border-white/5 hover:border-white/10 transition-all"
                      >
                        <div className="flex items-center gap-4">
                          <Chip
                            size="sm"
                            className={
                              request.level === AirlockLevels.Dangerous
                                ? "bg-red-500/20 text-red-400 border-red-500/20"
                                : request.level === AirlockLevels.Sensitive
                                  ? "bg-orange-500/20 text-orange-400 border-orange-500/20"
                                  : "bg-green-500/20 text-green-400 border-green-500/20"
                            }
                            variant="soft"
                          >
                            {request.command_type}
                          </Chip>
                          <code className="text-xs text-muted-foreground font-mono bg-black/20 px-2 py-1 rounded">
                            {JSON.stringify(request.payload).slice(0, 60)}...
                          </code>
                        </div>

                        <div className="flex gap-2">
                          <Button
                            variant="ghost"
                            size="sm"
                            isIconOnly
                            className="text-green-500 hover:bg-green-500/10"
                            onPress={() =>
                              handleAirlockRespond(request.id, true)
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
                              handleAirlockRespond(request.id, false)
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
                      onSuccess={() => setIsCreatingAgent(false)}
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
