import { Button } from "@heroui/react";
import {
  Copy,
  ExternalLink,
  Send,
  Shield,
  Smartphone,
  Check,
} from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import {
  ensureDefaultAtmAgent,
  generatePairingCode,
} from "../../../services/tauri";
import { NeuralChip } from "../shared/UiElements";

interface NeuralDashboardProps {
  workspace: {
    id: string;
    name: string;
  };
  nodeReady: boolean;
  nodeStatusLabel: string;
  isHeadless: boolean;
  onToggleHeadless: (enabled: boolean) => void;
}

export function NeuralDashboard({
  workspace,
  nodeReady,
  nodeStatusLabel,
}: Omit<NeuralDashboardProps, "isHeadless" | "onToggleHeadless">) {
  const [pairingCode, setPairingCode] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const [isPreparingSession, setIsPreparingSession] = useState(false);
  const pairingTimeoutRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const copiedTimeoutRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

  // Clear pending timeouts on unmount
  useEffect(() => () => {
    if (pairingTimeoutRef.current) clearTimeout(pairingTimeoutRef.current);
    if (copiedTimeoutRef.current) clearTimeout(copiedTimeoutRef.current);
  }, []);

  const handleGeneratePairingCode = async () => {
    if (isPreparingSession) return;
    setIsPreparingSession(true);
    try {
      if (!nodeReady) {
        toast.error(
          "Desktop node is still syncing with Bridge. Wait a moment and try again.",
          { id: "node-not-ready" },
        );
        return;
      }
      await ensureDefaultAtmAgent();
      const res = await generatePairingCode();
      if (res && res.code) {
        setPairingCode(res.code);
        if (pairingTimeoutRef.current) clearTimeout(pairingTimeoutRef.current);
        pairingTimeoutRef.current = setTimeout(() => setPairingCode(null), 5 * 60 * 1000);
      }
    } catch (err) {
      console.error("Failed to prepare remote access session:", err);
      const message =
        err instanceof Error
          ? err.message
          : typeof err === "string"
            ? err
            : JSON.stringify(err);
      toast.error(
        message?.trim().length
          ? `Failed to prepare remote access session: ${message}`
          : "Failed to prepare remote access session",
        { id: "pairing-prepare-failed" },
      );
    } finally {
      setIsPreparingSession(false);
    }
  };

  const handleCopyCode = async () => {
    if (pairingCode) {
      await navigator.clipboard.writeText(pairingCode);
      setCopied(true);
      toast.success("Pairing code copied to clipboard");
      if (copiedTimeoutRef.current) clearTimeout(copiedTimeoutRef.current);
      copiedTimeoutRef.current = setTimeout(() => setCopied(false), 2000);
    }
  };

  return (
    <div className="space-y-8 animate-appear">
      <div className="flex flex-col gap-1 border-b border-border/10 pb-6">
        <h3 className="text-2xl font-bold text-foreground tracking-tight">
          Dashboard
        </h3>
        <p className="text-muted-foreground text-sm">
          Workspace overview and session controls.
        </p>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-8 items-start">
        {/* Workspace Info */}
        <div className="rounded-3xl border border-border/20 bg-card/20 p-8 space-y-6 h-full min-h-[240px] flex flex-col justify-between">
          <div>
            <div className="flex items-center gap-3 mb-4">
              <h2 className="text-3xl font-light tracking-tight text-foreground">
                {workspace.name}
              </h2>
              <NeuralChip
                variant="flat"
                color="success"
                className="bg-emerald-500/10 text-emerald-500"
              >
                Active
              </NeuralChip>
            </div>
            <div className="flex flex-col gap-2 text-xs font-mono text-muted-foreground">
              <div className="flex items-center gap-2">
                <span className="opacity-50">ID:</span>
                <span className="select-all">{workspace.id}</span>
              </div>
              <div className="flex items-center gap-2">
                <span className="opacity-50">NODE:</span>
                <span>Desktop_v2</span>
              </div>
              <div className="flex items-center gap-2 text-emerald-500/80">
                <Shield className="size-3" />
                <span>Encrypted Channel</span>
              </div>
            </div>
          </div>

          <div className="pt-2">
            <a
              className="inline-flex items-center justify-center h-10 px-4 rounded-xl text-sm font-medium transition-colors bg-white/5 border border-white/10 text-foreground hover:bg-white/10 cursor-pointer"
              href={`https://platform.rainymate.com/workspaces/${workspace.id}`}
              target="_blank"
              rel="noopener noreferrer"
            >
              <ExternalLink className="size-4 mr-2 opacity-50" />
              View in Cloud Cortex
            </a>
          </div>
        </div>

        {/* Remote & Telegram - Redesigned Minimalist Card */}
        <div className="rounded-3xl border border-border/20 bg-card/20 p-6 h-full min-h-[240px] flex flex-col relative overflow-hidden group hover:border-border/40 transition-colors">
          {/* Header Section */}
          <div className="flex items-start justify-between mb-2">
            <div className="flex flex-col gap-1">
              <div className="flex items-center gap-2 text-foreground">
                <Smartphone className="size-5 text-blue-400" />
                <span className="text-lg font-semibold tracking-tight">
                  Remote Access
                </span>
              </div>
              <div className="text-sm text-muted-foreground">
                Connect via{" "}
                <a
                  href="https://t.me/RainyAMTBot"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-blue-400 hover:text-blue-300 hover:underline transition-colors inline-flex items-center gap-1"
                >
                  Telegram Bot
                  <ExternalLink className="size-3" />
                </a>
              </div>
            </div>

            {/* Status Indicator (Only visual when code exists) */}
            {pairingCode && (
              <div className="flex items-center gap-1.5 px-2 py-1 rounded-full bg-emerald-500/10 border border-emerald-500/20">
                <div className="size-1.5 rounded-full bg-emerald-500 animate-pulse" />
                <span className="text-[10px] font-medium text-emerald-500">
                  Active
                </span>
              </div>
            )}
          </div>

          {/* Main Content Area */}
          <div className="flex-1 flex flex-col items-center justify-center w-full">
            {pairingCode ? (
              <div className="w-full h-full flex items-center justify-center animate-in fade-in zoom-in-95 duration-300">
                <button
                  onClick={handleCopyCode}
                  className="group/code relative flex flex-col items-center justify-center gap-2 py-4 px-8 rounded-2xl hover:bg-background/5 transition-all cursor-pointer"
                >
                  <span className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground/30 font-medium group-hover/code:text-muted-foreground/50 transition-colors">
                    Active Session
                  </span>

                  <div className="relative flex items-center justify-center gap-4">
                    <span className="font-mono text-5xl md:text-6xl font-light text-foreground/90 tracking-[0.1em] select-all drop-shadow-sm group-hover/code:text-primary-400 transition-colors">
                      {pairingCode}
                    </span>
                    <div className="absolute -right-8 opacity-0 group-hover/code:opacity-100 transition-opacity translate-x-1 duration-200">
                      {copied ? (
                        <Check className="size-5 text-emerald-500" />
                      ) : (
                        <Copy className="size-5 text-muted-foreground" />
                      )}
                    </div>
                  </div>

                  <span className="text-[9px] text-muted-foreground/20 font-medium group-hover/code:text-muted-foreground/40 transition-colors mt-2">
                    Click Code to Copy
                  </span>
                </button>
              </div>
            ) : (
              <div className="w-full max-w-xs flex flex-col items-center gap-4 text-center">
                <div className="size-12 rounded-full bg-blue-500/10 flex items-center justify-center mb-1">
                  <Send className="size-5 text-blue-400 ml-0.5" />
                </div>
                <Button
                  className="w-full bg-blue-500/90 text-white hover:bg-blue-500 font-medium tracking-wide shadow-lg shadow-blue-500/20 transition-all"
                  onPress={handleGeneratePairingCode}
                  isDisabled={!nodeReady || isPreparingSession}
                >
                  {isPreparingSession ? "Preparing Session..." : "Generate Session Code"}
                </Button>
                <p className="text-xs text-muted-foreground/50">
                  {nodeReady
                    ? "Generate a temporary code to pair your device securely."
                    : nodeStatusLabel}
                </p>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
