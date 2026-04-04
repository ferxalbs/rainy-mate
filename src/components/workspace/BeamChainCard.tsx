import { useCallback, useEffect, useState } from "react";
import { Button, Chip } from "@heroui/react";
import { Shield, Zap, Wallet, Globe, RefreshCw, CheckCircle2, AlertCircle } from "lucide-react";
import * as tauri from "../../services/tauri";
import type { BeamWorkspaceConfig, WalletInfo } from "../../types/beam";

interface BeamChainCardProps {
  workspacePath: string;
}

const NETWORKS = [
  {
    id: "mainnet" as const,
    label: "Beam Mainnet",
    chainId: 4337,
    rpcUrl: "https://build.onbeam.com/rpc",
    color: "text-emerald-500",
    chipColor: "success" as const,
  },
  {
    id: "testnet" as const,
    label: "Beam Testnet",
    chainId: 13337,
    rpcUrl: "https://build.onbeam.com/rpc/testnet",
    color: "text-amber-500",
    chipColor: "warning" as const,
  },
];

export function BeamChainCard({ workspacePath }: BeamChainCardProps) {
  const [config, setConfig] = useState<BeamWorkspaceConfig | null>(null);
  const [wallets, setWallets] = useState<WalletInfo[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [isConnecting, setIsConnecting] = useState<string | null>(null);
  const [isCreatingWallet, setIsCreatingWallet] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const [cfg, walletList] = await Promise.allSettled([
        tauri.getBeamWorkspaceConfig(workspacePath),
        tauri.listBeamWallets(),
      ]);
      setConfig(cfg.status === "fulfilled" ? cfg.value : null);
      setWallets(walletList.status === "fulfilled" ? walletList.value : []);
    } finally {
      setIsLoading(false);
    }
  }, [workspacePath]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const handleConnect = useCallback(
    async (network: "mainnet" | "testnet") => {
      setIsConnecting(network);
      setError(null);
      try {
        const cfg = await tauri.connectBeamWorkspace(workspacePath, network);
        setConfig(cfg);
      } catch (e) {
        setError(e instanceof Error ? e.message : "Connection failed");
      } finally {
        setIsConnecting(null);
      }
    },
    [workspacePath],
  );

  const handleCreateWallet = useCallback(async () => {
    setIsCreatingWallet(true);
    setError(null);
    try {
      const wallet = await tauri.createBeamWallet(`Beam Wallet ${wallets.length + 1}`);
      setWallets((prev) => [...prev, wallet]);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Wallet creation failed");
    } finally {
      setIsCreatingWallet(false);
    }
  }, [wallets.length]);

  const activeNetwork = NETWORKS.find(
    (n) => config && n.chainId === config.chain.chainId,
  );

  return (
    <div className="rounded-2xl border border-white/10 bg-background/40 backdrop-blur-md p-5 flex flex-col gap-4">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <div className="flex items-center justify-center size-8 rounded-xl bg-primary/10">
            <Zap className="size-4 text-primary" />
          </div>
          <div>
            <p className="text-sm font-semibold text-foreground">Beam Chain Config</p>
            <p className="text-[11px] text-muted-foreground">
              Official Beam Mainnet / Testnet RPC
            </p>
          </div>
        </div>
        <button
          onClick={() => void refresh()}
          disabled={isLoading}
          className="rounded-lg p-1.5 text-muted-foreground hover:text-foreground hover:bg-white/5 transition-colors"
          aria-label="Refresh"
        >
          <RefreshCw className={`size-3.5 ${isLoading ? "animate-spin" : ""}`} />
        </button>
      </div>

      {/* Connection status */}
      {config ? (
        <div className="flex items-center gap-2 rounded-xl bg-emerald-500/10 px-3 py-2">
          <CheckCircle2 className="size-4 text-emerald-500 shrink-0" />
          <div className="min-w-0">
            <p className="text-xs font-medium text-emerald-500">{config.network}</p>
            <p className="text-[11px] text-muted-foreground font-mono truncate">
              {config.chain.rpcUrl}
            </p>
          </div>
          <Chip size="sm" color={activeNetwork?.chipColor ?? "default"} variant="secondary" className="shrink-0 ml-auto">
            Chain {config.chain.chainId}
          </Chip>
        </div>
      ) : (
        <div className="flex items-center gap-2 rounded-xl bg-foreground/5 px-3 py-2">
          <Globe className="size-4 text-muted-foreground shrink-0" />
          <p className="text-xs text-muted-foreground">Not connected to any Beam network</p>
        </div>
      )}

      {/* Network buttons */}
      <div className="grid grid-cols-2 gap-2">
        {NETWORKS.map((net) => {
          const isActive = config?.chain.chainId === net.chainId;
          const isThisConnecting = isConnecting === net.id;
          return (
            <button
              key={net.id}
              onClick={() => void handleConnect(net.id)}
              disabled={isThisConnecting || isConnecting !== null}
              className={`rounded-xl border px-3 py-2.5 text-left transition-all ${
                isActive
                  ? "border-emerald-500/40 bg-emerald-500/10"
                  : "border-white/10 bg-white/5 hover:border-white/20 hover:bg-white/10"
              } disabled:opacity-50 disabled:cursor-not-allowed`}
            >
              <p className={`text-xs font-medium ${isActive ? "text-emerald-500" : "text-foreground"}`}>
                {isThisConnecting ? "Connecting..." : net.label}
              </p>
              <p className="text-[11px] text-muted-foreground font-mono mt-0.5">
                Chain ID {net.chainId}
              </p>
            </button>
          );
        })}
      </div>

      {/* Secure Signing Bridge */}
      <div className="border-t border-white/5 pt-3">
        <div className="flex items-center justify-between mb-2">
          <div className="flex items-center gap-1.5">
            <Shield className="size-3.5 text-indigo-400" />
            <p className="text-[11px] font-medium text-foreground">Secure Local Signing Bridge</p>
          </div>
          {wallets.length > 0 && (
            <Chip size="sm" variant="secondary" color="accent" className="text-[10px]">
              {wallets.length} wallet{wallets.length !== 1 ? "s" : ""}
            </Chip>
          )}
        </div>
        <p className="text-[11px] text-muted-foreground mb-3 leading-relaxed">
          AES-256-GCM encrypted local wallets. Private keys never leave this device.
          All signatures require Airlock L2 approval.
        </p>

        {wallets.length > 0 && (
          <div className="flex flex-col gap-1 mb-3">
            {wallets.slice(0, 3).map((w) => (
              <div
                key={w.address}
                className="flex items-center gap-2 rounded-lg bg-white/5 px-2.5 py-1.5"
              >
                <Wallet className="size-3 text-muted-foreground shrink-0" />
                <span className="text-[11px] font-mono text-foreground truncate">
                  {w.address.slice(0, 8)}…{w.address.slice(-6)}
                </span>
                {w.label && (
                  <span className="text-[10px] text-muted-foreground ml-auto shrink-0">
                    {w.label}
                  </span>
                )}
              </div>
            ))}
            {wallets.length > 3 && (
              <p className="text-[10px] text-muted-foreground text-center">
                +{wallets.length - 3} more
              </p>
            )}
          </div>
        )}

        <Button
          size="sm"
          variant="secondary"
          className="w-full text-xs"
          isDisabled={isCreatingWallet}
          onPress={() => void handleCreateWallet()}
        >
          {isCreatingWallet ? (
            <RefreshCw className="size-3.5 mr-1 animate-spin" />
          ) : (
            <Wallet className="size-3.5 mr-1" />
          )}
          {isCreatingWallet ? "Creating..." : "Create Local Wallet"}
        </Button>
      </div>

      {/* Quick info */}
      <div className="grid grid-cols-2 gap-2 text-[10px]">
        <div className="rounded-lg bg-white/5 px-2 py-1.5">
          <p className="text-muted-foreground">Mainnet RPC</p>
          <p className="font-mono text-foreground truncate">build.onbeam.com/rpc</p>
        </div>
        <div className="rounded-lg bg-white/5 px-2 py-1.5">
          <p className="text-muted-foreground">Testnet RPC</p>
          <p className="font-mono text-foreground truncate">build.onbeam.com/rpc/testnet</p>
        </div>
      </div>

      {error && (
        <div className="flex items-start gap-2 rounded-xl bg-red-500/10 px-3 py-2">
          <AlertCircle className="size-3.5 text-red-500 shrink-0 mt-0.5" />
          <p className="text-xs text-red-500">{error}</p>
        </div>
      )}
    </div>
  );
}
