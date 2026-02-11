import { Button } from "@heroui/react";
import { ExternalLink, Shield, Smartphone } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";
import { generatePairingCode, setHeadlessMode } from "../../../services/tauri";
import { NeuralChip, NeuralSwitch } from "../shared/UiElements";

interface NeuralDashboardProps {
  workspace: {
    id: string;
    name: string;
  };
  isHeadless: boolean;
  onToggleHeadless: (enabled: boolean) => void;
}

export function NeuralDashboard({
  workspace,
  isHeadless,
  onToggleHeadless,
}: NeuralDashboardProps) {
  const [pairingCode, setPairingCode] = useState<string | null>(null);

  const handleToggleHeadless = async (enabled: boolean) => {
    try {
      await setHeadlessMode(enabled);
      onToggleHeadless(enabled);
      toast.success(`Headless Mode ${enabled ? "Enabled" : "Disabled"}`);
    } catch (err) {
      toast.error("Failed to update settings");
    }
  };

  const handleGeneratePairingCode = async () => {
    try {
      const res = await generatePairingCode();
      if (res && res.code) {
        setPairingCode(res.code);
        setTimeout(() => setPairingCode(null), 5 * 60 * 1000); // Expire in 5m locally
      }
    } catch (err) {
      toast.error("Failed to generate pairing code");
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

      <div className="grid grid-cols-1 md:grid-cols-2 gap-8">
        {/* Workspace Info */}
        <div className="space-y-6">
          <div className="rounded-2xl border border-border/20 bg-card/20 p-6 space-y-4">
            <div>
              <div className="flex items-center gap-3 mb-2">
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
              <div className="flex flex-col gap-1 text-xs font-mono text-muted-foreground">
                <div className="flex items-center gap-2">
                  <span className="opacity-50">ID:</span>
                  <span>{workspace.id}</span>
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
                className="inline-flex items-center justify-center h-8 px-3 rounded-lg text-sm font-medium transition-colors bg-transparent border border-border/20 text-foreground hover:bg-foreground/5 cursor-pointer"
                href={`https://platform.rainymate.com/workspaces/${workspace.id}`}
                target="_blank"
                rel="noopener noreferrer"
              >
                <ExternalLink className="size-3 mr-2 opacity-50" />
                View in Cloud Cortex
              </a>
            </div>
          </div>
        </div>

        {/* Session Controls */}
        <div className="space-y-4">
          {/* Headless Toggle */}
          <div className="flex items-center justify-between gap-4 p-5 rounded-2xl bg-card/20 hover:bg-card/30 transition-all border border-border/20 group">
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
            <NeuralSwitch
              checked={isHeadless}
              onChange={handleToggleHeadless}
            />
          </div>

          {/* Mobile Link */}
          <div className="flex items-center justify-between gap-4 p-5 rounded-2xl bg-card/20 hover:bg-card/30 transition-all border border-border/20 group">
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
                className="bg-blue-500/10 text-blue-400 hover:bg-blue-500/20 font-semibold"
                onPress={handleGeneratePairingCode}
              >
                Generate
              </Button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
