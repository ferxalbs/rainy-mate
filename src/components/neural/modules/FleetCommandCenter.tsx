import { useEffect, useState } from "react";
import { AlertTriangle, RefreshCcw, ShieldCheck, ShieldOff, Check } from "lucide-react";
import { Button, Card, Modal } from "@heroui/react";
import { toast } from "sonner";
import {
  getAtmFleetStatus,
  pushAtmFleetPolicy,
  retireAtmFleetNode,
  triggerAtmFleetKillSwitch,
  type AtmFleetCurrentPolicy,
  type AtmFleetNodeStatus,
} from "../../../services/tauri";

type PendingAction =
  | { type: "kill" }
  | { type: "retire"; node: AtmFleetNodeStatus }
  | null;

export function FleetCommandCenter() {
  const [nodes, setNodes] = useState<AtmFleetNodeStatus[]>([]);
  const [currentPolicy, setCurrentPolicy] = useState<AtmFleetCurrentPolicy | null>(
    null,
  );
  const [lastDispatch, setLastDispatch] = useState<string>("");
  const [loading, setLoading] = useState(false);
  const [retiringNodeId, setRetiringNodeId] = useState<string | null>(null);
  const [pendingAction, setPendingAction] = useState<PendingAction>(null);
  const [actionSubmitting, setActionSubmitting] = useState(false);
  const [confirmPhrase, setConfirmPhrase] = useState("");
  const [ackChecked, setAckChecked] = useState(false);
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

  const executeKillSwitch = async () => {
    try {
      const res: any = await triggerAtmFleetKillSwitch();
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

  const executeRetireNode = async (node: AtmFleetNodeStatus) => {
    setRetiringNodeId(node.id);
    try {
      await retireAtmFleetNode(node.id, "retired_from_fleet_ui");
      toast.success("Node retired");
      await load();
    } catch (error) {
      console.error(error);
      toast.error("Failed to retire node");
    } finally {
      setRetiringNodeId(null);
    }
  };

  const openKillDialog = () => {
    setPendingAction({ type: "kill" });
    setConfirmPhrase("");
    setAckChecked(false);
  };

  const openRetireDialog = (node: AtmFleetNodeStatus) => {
    setPendingAction({ type: "retire", node });
    setConfirmPhrase("");
    setAckChecked(false);
  };

  const closeActionDialog = () => {
    if (actionSubmitting) return;
    setPendingAction(null);
    setConfirmPhrase("");
    setAckChecked(false);
  };

  const requiredPhrase = pendingAction?.type === "kill" ? "KILL" : "RETIRE";
  const canConfirm =
    !!pendingAction &&
    !actionSubmitting &&
    ackChecked &&
    confirmPhrase.trim().toUpperCase() === requiredPhrase;

  const handleConfirmAction = async () => {
    if (!pendingAction || !canConfirm) return;
    setActionSubmitting(true);
    let succeeded = false;
    try {
      if (pendingAction.type === "kill") {
        await executeKillSwitch();
      } else {
        await executeRetireNode(pendingAction.node);
      }
      succeeded = true;
    } finally {
      setActionSubmitting(false);
      if (succeeded) {
        setPendingAction(null);
        setConfirmPhrase("");
        setAckChecked(false);
      }
    }
  };

  const visibleNodes = nodes.filter((node) => node.effectiveStatus !== "retired");

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
          <Button onPress={openKillDialog} className="bg-red-600 text-white">
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
        {visibleNodes.map((node) => {
          const displayHostname =
            node.hostname === "unknown-host"
              ? `unknown-host-${node.id.slice(-4)}`
              : node.hostname;
          return (
            <Card key={node.id} className="p-4 border border-border/20 bg-card/30">
              <div className="flex items-center justify-between">
                <div>
                  <p className="text-sm font-semibold text-foreground">{displayHostname}</p>
                  <p className="text-xs text-muted-foreground">{node.platform}</p>
                </div>
                <span className="text-xs font-mono text-muted-foreground">
                  {node.effectiveStatus ?? node.status}
                </span>
              </div>
              <div className="mt-3 space-y-1 text-xs text-muted-foreground">
                <p>Health: {node.health?.score ?? 0}/100</p>
                <p>Pending approvals: {node.pendingApprovals ?? 0}</p>
                <p>Last seen: {Math.floor((node.lastSeenMsAgo || 0) / 1000)}s ago</p>
              </div>
              {(node.health?.score ?? 0) < 60 ||
              node.effectiveStatus === "stale" ? (
                <div className="mt-3 flex items-center gap-2 text-amber-500 text-xs">
                  <AlertTriangle className="size-3" />
                  Node health degraded / stale heartbeat
                </div>
              ) : null}
              <div className="mt-3 flex justify-end">
                <Button
                  size="sm"
                  className="bg-red-500/15 text-red-400 hover:bg-red-500/25"
                  isDisabled={
                    retiringNodeId === node.id || node.effectiveStatus === "retired"
                  }
                  onPress={() => openRetireDialog(node)}
                >
                  {retiringNodeId === node.id ? "Retiring..." : "Retire"}
                </Button>
              </div>
            </Card>
          );
        })}
      </div>

      <Modal.Backdrop
        isOpen={pendingAction !== null}
        onOpenChange={(open) => {
          if (!open) closeActionDialog();
        }}
        className="backdrop-blur-md bg-background/80 dark:bg-black/60 z-[9999]"
      >
        <Modal.Container>
          <Modal.Dialog className="max-w-md w-full backdrop-blur-md bg-background/85 dark:bg-background/20 rounded-2xl shadow-2xl border border-border/20 overflow-hidden">
            <Modal.Header className="p-6 pb-4 border-b border-border/10 relative overflow-hidden">
              <div className="absolute inset-0 bg-gradient-to-r from-danger-500/10 to-transparent pointer-events-none" />
              <div className="space-y-1.5 relative z-10">
                <Modal.Heading className="text-xl font-bold text-foreground tracking-tight flex items-center gap-2">
                  <AlertTriangle className="size-5 text-danger-500" />
                  {pendingAction?.type === "kill"
                    ? "Confirm Kill Switch"
                    : "Confirm Node Retire"}
                </Modal.Heading>
                <p className="text-sm text-muted-foreground">
                  {pendingAction?.type === "kill"
                    ? "This stops new agent runs across connected nodes."
                    : "This retires the node from active routing and rejects pending assigned commands."}
                </p>
              </div>
            </Modal.Header>
            <Modal.Body className="px-6 py-5 space-y-5">
              {pendingAction?.type === "retire" ? (
                <div className="p-3.5 rounded-xl border border-border/10 bg-background/30 dark:bg-background/10">
                  <p className="text-[10px] text-muted-foreground mb-1 uppercase tracking-widest font-bold">Target Node</p>
                  <p className="font-mono text-foreground text-sm select-all">{pendingAction.node.id}</p>
                </div>
              ) : null}
              
              <div className="space-y-2">
                <label className="text-[10px] text-muted-foreground uppercase tracking-widest font-bold">
                  Type <span className="text-foreground bg-background/40 dark:bg-background/20 px-1.5 py-0.5 rounded ml-1 border border-border/10">{requiredPhrase}</span> to confirm
                </label>
                <input
                  value={confirmPhrase}
                  onChange={(event) => setConfirmPhrase(event.target.value)}
                  placeholder={`e.g. ${requiredPhrase}`}
                  className="w-full h-12 rounded-xl border border-border/20 bg-background/40 dark:bg-background/10 px-4 text-sm text-foreground placeholder:text-muted-foreground/60 focus:outline-none focus:border-danger-500/50 focus:ring-1 focus:ring-danger-500/50 transition-all font-mono backdrop-blur-sm"
                />
              </div>

              <label className="flex items-start gap-3 text-sm text-muted-foreground mt-2 cursor-pointer group">
                <div className="relative flex items-center justify-center mt-0.5">
                  <input
                    type="checkbox"
                    checked={ackChecked}
                    onChange={(event) => setAckChecked(event.target.checked)}
                    className="appearance-none size-5 rounded border border-border/20 bg-background/40 dark:bg-background/10 checked:bg-danger-500 checked:border-danger-500 transition-all cursor-pointer peer shrink-0 backdrop-blur-sm"
                  />
                  <Check className="absolute size-3.5 text-white opacity-0 peer-checked:opacity-100 pointer-events-none transition-opacity" strokeWidth={3} />
                </div>
                <span className="group-hover:text-foreground leading-tight font-medium transition-colors">I understand this is a sensitive operation that will permanently affect the network state.</span>
              </label>
            </Modal.Body>
            <Modal.Footer className="px-6 py-4 border-t border-border/10 flex justify-end gap-3 bg-background/20 dark:bg-background/5">
              <Button
                onPress={closeActionDialog}
                isDisabled={actionSubmitting}
                className="bg-background/40 dark:bg-background/10 border border-border/10 text-muted-foreground hover:text-foreground hover:bg-background/60 dark:hover:bg-background/20 rounded-xl px-5 font-medium transition-all backdrop-blur-sm"
              >
                Cancel
              </Button>
              <Button
                onPress={handleConfirmAction}
                isDisabled={!canConfirm}
                className={`rounded-xl px-6 font-semibold transition-all shadow-lg ${
                  canConfirm ? "bg-danger-500 text-white shadow-danger-500/20 hover:bg-danger-600" : "bg-danger-500/20 text-danger-500/30"
                }`}
              >
                {actionSubmitting ? "Processing..." : pendingAction?.type === "kill" ? "Execute Kill" : "Confirm Retire"}
              </Button>
            </Modal.Footer>
          </Modal.Dialog>
        </Modal.Container>
      </Modal.Backdrop>
    </div>
  );
}
