import { Button, Card, Chip, Separator, Switch } from "@heroui/react";
import {
  Network,
  RefreshCw,
  Copy,
  Shield,
  CheckCircle2,
  XCircle,
  Clock,
  Key,
} from "lucide-react";
import { useNeuralService } from "../../hooks/useNeuralService";
import { useEffect, useState } from "react";
import { toast } from "@heroui/react";
import {
  setNeuralCredentials,
  loadNeuralCredentials,
} from "../../services/tauri";
import { AgentList } from "./AgentList";
import { CreateAgentForm } from "./CreateAgentForm";

export function NeuralPanel() {
  const {
    status,
    nodeId,
    pendingApprovals,
    lastHeartbeat,
    connect,
    respond,
    isHeadless,
    toggleHeadless,
  } = useNeuralService();

  const [platformKey, setPlatformKey] = useState("");
  const [userApiKey, setUserApiKey] = useState("");
  const [isPairing, setIsPairing] = useState(false);
  const [hasCredentials, setHasCredentials] = useState(false);
  const [isCreatingAgent, setIsCreatingAgent] = useState(false);

  // Check for existing credentials on mount
  useEffect(() => {
    const checkCredentials = async () => {
      try {
        // Try to load from Keychain
        const loaded = await loadNeuralCredentials();
        if (loaded) {
          setHasCredentials(true);
          // Auto-connect if we have credentials
          connect();
        }
      } catch (error) {
        console.error("Failed to load credentials:", error);
      }
    };
    checkCredentials();
  }, [connect]);

  // Auto-connect when credentials are set
  useEffect(() => {
    if (hasCredentials && status === "pending-pairing") {
      connect();
    }
  }, [hasCredentials, status, connect]);

  const copyNodeId = () => {
    if (nodeId) {
      navigator.clipboard.writeText(nodeId);
      toast.success("Node ID copied to clipboard");
    }
  };

  const handlePairing = async () => {
    if (!platformKey.trim() || !userApiKey.trim()) {
      toast.danger("Both Platform Key and User API Key are required");
      return;
    }
    setIsPairing(true);
    try {
      await setNeuralCredentials(platformKey, userApiKey);
      setHasCredentials(true);
      await connect();
      toast.success("Successfully paired with Cloud Cortex");
    } catch (error) {
      console.error("Pairing failed:", error);
      toast.danger("Pairing failed. Check your credentials.");
    } finally {
      setIsPairing(false);
    }
  };

  const getStatusColor = (currentStatus: string) => {
    switch (currentStatus) {
      case "connected":
        return "text-green-500";
      case "pending-pairing":
        return "text-yellow-500";
      case "offline":
        return "text-gray-500";
      case "error":
        return "text-red-500";
      default:
        return "text-gray-500";
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

      {/* Connection Status Card */}
      <Card className="p-6">
        <div className="flex items-center justify-between mb-6">
          <div className="flex items-center gap-3">
            <div
              className={`p-2 rounded-full ${getStatusColor(status).replace("text-", "bg-")}/10`}
            >
              <Network className={`size-6 ${getStatusColor(status)}`} />
            </div>
            <div>
              <h3 className="text-lg font-semibold">Neural Link</h3>
              <div className="flex items-center gap-2">
                <span
                  className={`size-2 rounded-full ${getStatusColor(status).replace("text-", "bg-")} animate-pulse`}
                />
                <span className="text-sm text-muted-foreground capitalize">
                  {status.replace("-", " ")}
                </span>
              </div>
            </div>
          </div>
          {status === "pending-pairing" && !hasCredentials ? (
            <div className="flex flex-col gap-3 mt-4">
              <div className="flex items-center gap-2">
                <Key className="size-4 text-muted-foreground" />
                <span className="text-sm font-medium">Authentication Keys</span>
              </div>
              <input
                type="password"
                placeholder="Platform Key (RAINY_PLATFORM_KEY)"
                className="px-3 py-2 rounded-lg border bg-background text-sm font-mono"
                value={platformKey}
                onChange={(e) => setPlatformKey(e.target.value)}
              />
              <input
                type="password"
                placeholder="User API Key (rny_...)"
                className="px-3 py-2 rounded-lg border bg-background text-sm font-mono"
                value={userApiKey}
                onChange={(e) => setUserApiKey(e.target.value)}
              />
              <Button
                size="sm"
                variant="primary"
                onPress={handlePairing}
                isDisabled={
                  isPairing || !platformKey.trim() || !userApiKey.trim()
                }
                className="w-full"
              >
                {isPairing ? (
                  <RefreshCw className="size-4 animate-spin" />
                ) : (
                  "Connect to Cloud Cortex"
                )}
              </Button>
            </div>
          ) : (
            <Button
              size="sm"
              variant="ghost"
              onPress={connect}
              className="gap-2"
            >
              <RefreshCw className="size-4" />
              {status === "error" ? "Retry" : "Reconnect"}
            </Button>
          )}
        </div>

        {/* Node ID */}
        {nodeId && (
          <div className="mt-4 p-3 bg-muted/50 rounded-lg">
            <div className="flex items-center justify-between">
              <div>
                <span className="text-xs text-muted-foreground uppercase tracking-wider">
                  Node ID
                </span>
                <p className="font-mono text-sm mt-1 truncate max-w-[280px]">
                  {nodeId}
                </p>
              </div>
              <Button variant="ghost" size="sm" onPress={copyNodeId}>
                <Copy className="size-4" />
              </Button>
            </div>
          </div>
        )}

        {status === "connected" && lastHeartbeat && (
          <div className="text-right text-sm text-muted-foreground mt-4">
            <div className="flex items-center gap-1 justify-end">
              <Clock className="size-3" />
              <span>Last sync: {lastHeartbeat.toLocaleTimeString()}</span>
            </div>
          </div>
        )}
      </Card>

      {/* Agent Management Section */}
      {status === "connected" && hasCredentials && (
        <Card className="p-6">
          {isCreatingAgent ? (
            <CreateAgentForm
              onSuccess={() => setIsCreatingAgent(false)}
              onCancel={() => setIsCreatingAgent(false)}
            />
          ) : (
            <AgentList onCreateClick={() => setIsCreatingAgent(true)} />
          )}
        </Card>
      )}

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
          <Switch isSelected={isHeadless} onChange={toggleHeadless}>
            <Switch.Control className="bg-default-200 data-[selected=true]:bg-purple-500">
              <Switch.Thumb />
            </Switch.Control>
          </Switch>
        </div>
      </Card>

      <Separator />

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
                          request.level === "Dangerous"
                            ? "danger"
                            : request.level === "Sensitive"
                              ? "warning"
                              : "success"
                        }
                        size="sm"
                      >
                        {request.level}
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
                      onPress={() => respond(request.id, true)}
                    >
                      <CheckCircle2 className="size-4" />
                    </Button>
                    <Button
                      variant="danger"
                      size="sm"
                      onPress={() => respond(request.id, false)}
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
