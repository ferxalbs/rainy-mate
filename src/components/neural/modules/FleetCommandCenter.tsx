import { useEffect, useState } from "react";
import { AlertTriangle, RefreshCcw, ShieldCheck, ShieldOff } from "lucide-react";
import { Button, Card } from "@heroui/react";
import { toast } from "sonner";
import {
  getAtmFleetStatus,
  pushAtmFleetPolicy,
  triggerAtmFleetKillSwitch,
  type AtmFleetCurrentPolicy,
  type AtmFleetNodeStatus,
} from "../../../services/tauri";

interface FleetCommandCenterProps {
  platformKey: string;
  userApiKey: string;
}

export function FleetCommandCenter({
  platformKey,
  userApiKey,
}: FleetCommandCenterProps) {
  const [nodes, setNodes] = useState<AtmFleetNodeStatus[]>([]);
  const [currentPolicy, setCurrentPolicy] = useState<AtmFleetCurrentPolicy | null>(
    null,
  );
  const [lastDispatch, setLastDispatch] = useState<string>("");
  const [loading, setLoading] = useState(false);
  const [allowlist, setAllowlist] = useState("read_file,list_files,search_files");

  const load = async () => {
    setLoading(true);
    try {
      const res = await getAtmFleetStatus();
      setNodes(res.nodes || []);
      setCurrentPolicy(res.currentAirlockPolicy ?? null);
    } catch (error) {
      console.error(error);
      toast.error("Failed to load fleet status");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    load();
  }, []);

  const handlePushPolicy = async () => {
    const allow = allowlist
      .split(",")
      .map((item) => item.trim())
      .filter(Boolean);

    try {
      const res: any = await pushAtmFleetPolicy({
        toolAccessPolicy: {
          enabled: true,
          mode: "allowlist",
          allow,
          deny: [],
        },
        platformKey,
        userApiKey,
      });
      const ack = res?.dispatch?.ack?.within5s;
      if (ack) {
        setLastDispatch(
          `Policy push ack: running/completed=${ack.runningOrCompleted}, completed=${ack.completed}, failed=${ack.failed}, pending=${ack.pending}`,
        );
      }
      toast.success("Fleet policy pushed");
      await load();
    } catch (error) {
      console.error(error);
      toast.error("Failed to push fleet policy");
    }
  };

  const handleKillSwitch = async () => {
    const confirmKill = confirm(
      "Trigger fleet kill switch? This will stop new agent runs on connected nodes.",
    );
    if (!confirmKill) return;

    try {
      const res: any = await triggerAtmFleetKillSwitch({ platformKey, userApiKey });
      const ack = res?.dispatch?.ack?.within5s;
      if (ack) {
        setLastDispatch(
          `Kill switch ack: running/completed=${ack.runningOrCompleted}, completed=${ack.completed}, failed=${ack.failed}, pending=${ack.pending}`,
        );
      }
      toast.success("Fleet kill switch dispatched");
      await load();
    } catch (error) {
      console.error(error);
      toast.error("Failed to dispatch kill switch");
    }
  };

  return (
    <div className="space-y-6 animate-appear">
      <div className="flex items-center justify-between border-b border-border/10 pb-6">
        <div>
          <h3 className="text-2xl font-bold text-foreground tracking-tight">
            Fleet Command Center
          </h3>
          <p className="text-muted-foreground text-sm">
            Monitor node health, push policy, and trigger emergency controls.
          </p>
        </div>
        <Button
          onPress={load}
          isDisabled={loading}
          className="bg-primary/10 text-primary"
        >
          <RefreshCcw className="size-4 mr-2" />
          Refresh
        </Button>
      </div>

      <Card className="p-4 border border-border/20 bg-card/30">
        <div className="flex flex-col gap-3 md:flex-row md:items-end">
          <input
            value={allowlist}
            onChange={(event) => setAllowlist(event.target.value)}
            placeholder="read_file,list_files,search_files"
            className="w-full md:flex-1 h-10 rounded-lg border border-border bg-background/40 px-3 text-sm"
          />
          <Button onPress={handlePushPolicy} className="bg-emerald-600 text-white">
            <ShieldCheck className="size-4 mr-2" />
            Push Policy
          </Button>
          <Button onPress={handleKillSwitch} className="bg-red-600 text-white">
            <ShieldOff className="size-4 mr-2" />
            Kill Switch
          </Button>
        </div>
        <div className="mt-3 text-xs text-muted-foreground space-y-1">
          {currentPolicy ? (
            <p>
              Current policy: mode={currentPolicy.mode}, enabled=
              {String(currentPolicy.enabled)}, version={currentPolicy.version}
            </p>
          ) : (
            <p>Current policy: unavailable</p>
          )}
          {lastDispatch ? <p>{lastDispatch}</p> : null}
        </div>
      </Card>

      <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
        {nodes.map((node) => (
          <Card key={node.id} className="p-4 border border-border/20 bg-card/30">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm font-semibold text-foreground">{node.hostname}</p>
                <p className="text-xs text-muted-foreground">{node.platform}</p>
              </div>
              <span className="text-xs font-mono text-muted-foreground">
                {node.status}
              </span>
            </div>
            <div className="mt-3 space-y-1 text-xs text-muted-foreground">
              <p>Health: {node.health?.score ?? 0}/100</p>
              <p>Pending approvals: {node.pendingApprovals ?? 0}</p>
              <p>Last seen: {Math.floor((node.lastSeenMsAgo || 0) / 1000)}s ago</p>
            </div>
            {(node.health?.score ?? 0) < 60 && (
              <div className="mt-3 flex items-center gap-2 text-amber-500 text-xs">
                <AlertTriangle className="size-3" />
                Node health degraded
              </div>
            )}
          </Card>
        ))}
      </div>
    </div>
  );
}
