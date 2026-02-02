import { Button, Card, Chip, Separator, Switch } from "@heroui/react";
import {
  Network,
  Wifi,
  WifiOff,
  Shield,
  CheckCircle2,
  XCircle,
  Clock,
  Copy,
  RefreshCw,
} from "lucide-react";
import { useNeuralService } from "../../hooks/useNeuralService";
import { useEffect } from "react";
import { toast } from "@heroui/react";

export function NeuralPanel() {
  const {
    status,
    nodeId,
    pendingApprovals,
    lastHeartbeat,
    connect,
    respond,
    isPending,
    isHeadless,
    toggleHeadless,
  } = useNeuralService();

  // Auto-connect on mount (can be changed to manual)
  useEffect(() => {
    if (isPending) {
      connect();
    }
  }, []);

  const copyNodeId = () => {
    if (nodeId) {
      navigator.clipboard.writeText(nodeId);
      toast.success("Node ID copied to clipboard");
    }
  };

  const statusConfig = {
    "pending-pairing": {
      icon: <RefreshCw className="size-5 animate-spin text-yellow-500" />,
      label: "Connecting...",
      color: "warning" as const,
    },
    connected: {
      icon: <Wifi className="size-5 text-green-500" />,
      label: "Connected",
      color: "success" as const,
    },
    offline: {
      icon: <WifiOff className="size-5 text-gray-500" />,
      label: "Offline",
      color: "default" as const,
    },
    error: {
      icon: <WifiOff className="size-5 text-red-500" />,
      label: "Error",
      color: "danger" as const,
    },
  };

  const currentStatus = statusConfig[status];

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

      {/* Status Card */}
      <Card className="p-6">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-4">
            {currentStatus.icon}
            <div>
              <h3 className="font-semibold">Connection Status</h3>
              <Chip color={currentStatus.color} size="sm" className="mt-1">
                {currentStatus.label}
              </Chip>
            </div>
          </div>

          {status === "connected" && lastHeartbeat && (
            <div className="text-right text-sm text-muted-foreground">
              <div className="flex items-center gap-1">
                <Clock className="size-3" />
                <span>Last sync: {lastHeartbeat.toLocaleTimeString()}</span>
              </div>
            </div>
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

        {/* Reconnect Button */}
        {(status === "offline" || status === "error") && (
          <Button className="mt-4 w-full" onPress={connect}>
            <RefreshCw className="size-4 mr-2" />
            Reconnect
          </Button>
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
