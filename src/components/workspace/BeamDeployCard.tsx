import { useCallback, useEffect, useMemo, useState } from "react";
import { Button, Modal } from "@heroui/react";
import {
  AlertCircle,
  CheckCircle2,
  FileCode2,
  Rocket,
  ScrollText,
  Zap,
} from "lucide-react";

import * as tauri from "../../services/tauri";

interface BeamDeployCardProps {
  workspacePath: string;
  onDeploymentRecorded?: () => Promise<void> | void;
}

export function BeamDeployCard({
  workspacePath,
  onDeploymentRecorded,
}: BeamDeployCardProps) {
  const [templates, setTemplates] = useState<tauri.BeamTemplateSummary[]>([]);
  const [templateDetail, setTemplateDetail] = useState<tauri.BeamTemplateDetail | null>(null);
  const [wallets, setWallets] = useState<tauri.WalletInfo[]>([]);
  const [selectedTemplateId, setSelectedTemplateId] = useState<string>("");
  const [selectedWallet, setSelectedWallet] = useState<string>("");
  const [network, setNetwork] = useState<"mainnet" | "testnet">("testnet");
  const [deployPlan, setDeployPlan] = useState<tauri.BeamDeploymentPlan | null>(null);
  const [deploymentResult, setDeploymentResult] =
    useState<tauri.BeamDeploymentResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isPreparing, setIsPreparing] = useState(false);
  const [isDeploying, setIsDeploying] = useState(false);
  const [isScaffolding, setIsScaffolding] = useState(false);
  const [reviewOpen, setReviewOpen] = useState(false);

  const refresh = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const [nextTemplates, nextWallets, beamConfig] = await Promise.all([
        tauri.listBeamTemplates(),
        tauri.listBeamWallets(),
        tauri.getBeamWorkspaceConfig(workspacePath).catch(() => null),
      ]);
      setTemplates(nextTemplates);
      setWallets(nextWallets);
      if (!selectedTemplateId && nextTemplates[0]) {
        setSelectedTemplateId(nextTemplates[0].id);
      }
      if (!selectedWallet && nextWallets[0]) {
        setSelectedWallet(nextWallets[0].address);
      }
      if (beamConfig?.chain.chainId === 4337) {
        setNetwork("mainnet");
      } else if (beamConfig?.chain.chainId === 13337) {
        setNetwork("testnet");
      }
    } catch (nextError) {
      setError(readErrorMessage(nextError, "Failed to load Beam deploy card"));
    } finally {
      setIsLoading(false);
    }
  }, [selectedTemplateId, selectedWallet, workspacePath]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    if (!selectedTemplateId) return;
    let cancelled = false;
    setError(null);
    void tauri
      .getBeamTemplate(selectedTemplateId)
      .then((detail) => {
        if (cancelled) return;
        setTemplateDetail(detail);
        if (detail.recommendedNetwork === "mainnet" || detail.recommendedNetwork === "testnet") {
          setNetwork(detail.recommendedNetwork);
        }
      })
      .catch((nextError) => {
        if (!cancelled) {
          setError(readErrorMessage(nextError, "Failed to load template"));
        }
      });
    return () => {
      cancelled = true;
    };
  }, [selectedTemplateId]);

  const selectedWalletLabel = useMemo(
    () => wallets.find((wallet) => wallet.address === selectedWallet)?.label ?? null,
    [selectedWallet, wallets],
  );

  const handleScaffold = useCallback(async () => {
    if (!selectedTemplateId) return;
    setIsScaffolding(true);
    setError(null);
    try {
      await tauri.scaffoldBeamTemplate(workspacePath, selectedTemplateId);
      await refresh();
    } catch (nextError) {
      setError(readErrorMessage(nextError, "Failed to scaffold template"));
    } finally {
      setIsScaffolding(false);
    }
  }, [refresh, selectedTemplateId, workspacePath]);

  const handlePrepare = useCallback(async () => {
    if (!selectedTemplateId || !selectedWallet) {
      setError("Select a Beam template and wallet before preparing a deployment.");
      return;
    }
    setIsPreparing(true);
    setError(null);
    try {
      const launch = await tauri.prepareWorkspaceLaunch(workspacePath, "beam_deploy");
      const plan = await tauri.prepareBeamTemplateDeployment({
        workspacePath,
        templateId: selectedTemplateId,
        network,
        walletAddress: selectedWallet,
        requestId: launch.requestId,
      });
      setDeployPlan(plan);
      setReviewOpen(true);
      await onDeploymentRecorded?.();
    } catch (nextError) {
      setError(readErrorMessage(nextError, "Failed to prepare deployment"));
    } finally {
      setIsPreparing(false);
    }
  }, [
    network,
    onDeploymentRecorded,
    selectedTemplateId,
    selectedWallet,
    workspacePath,
  ]);

  const handleDeploy = useCallback(async () => {
    if (!deployPlan) return;
    setIsDeploying(true);
    setError(null);
    try {
      const result = await tauri.deployBeamTemplate({
        workspacePath,
        templateId: deployPlan.template.id,
        network,
        walletAddress: deployPlan.walletAddress,
        requestId: deployPlan.requestId ?? null,
      });
      setDeploymentResult(result);
      setReviewOpen(false);
      await onDeploymentRecorded?.();
    } catch (nextError) {
      setError(readErrorMessage(nextError, "Beam deployment failed"));
    } finally {
      setIsDeploying(false);
    }
  }, [deployPlan, network, onDeploymentRecorded, workspacePath]);

  return (
    <>
      <div className="rounded-2xl border border-white/10 bg-background/40 p-5 backdrop-blur-md">
        <div className="flex flex-col gap-4">
          <div className="flex items-center justify-between gap-4">
            <div>
              <div className="flex items-center gap-2">
                <Rocket className="size-4 text-primary" />
                <p className="text-sm font-semibold text-foreground">
                  Beam Templates Library + One-Click Deploy
                </p>
              </div>
              <p className="mt-1 text-[12px] leading-relaxed text-muted-foreground">
                Launchpad-governed Beam deployments with template preview, local compile,
                transaction estimate, and Airlock-backed broadcast.
              </p>
            </div>
            <Button
              size="sm"
              variant="secondary"
              onPress={() => void handleScaffold()}
              isDisabled={!selectedTemplateId || isScaffolding}
            >
              <FileCode2 className="mr-1 size-3.5" />
              {isScaffolding ? "Scaffolding..." : "Scaffold"}
            </Button>
          </div>

          <div className="grid gap-3 md:grid-cols-3">
            <label className="flex flex-col gap-1.5 text-[11px] text-muted-foreground">
              Template
              <select
                className="rounded-xl border border-white/10 bg-background px-3 py-2 text-sm text-foreground outline-none"
                value={selectedTemplateId}
                onChange={(event) => {
                  setSelectedTemplateId(event.target.value);
                  setDeployPlan(null);
                  setDeploymentResult(null);
                }}
                disabled={isLoading}
              >
                {templates.map((template) => (
                  <option key={template.id} value={template.id}>
                    {template.title}
                  </option>
                ))}
              </select>
            </label>

            <label className="flex flex-col gap-1.5 text-[11px] text-muted-foreground">
              Network
              <select
                className="rounded-xl border border-white/10 bg-background px-3 py-2 text-sm text-foreground outline-none"
                value={network}
                onChange={(event) => setNetwork(event.target.value as "mainnet" | "testnet")}
              >
                <option value="testnet">Beam Testnet</option>
                <option value="mainnet">Beam Mainnet</option>
              </select>
            </label>

            <label className="flex flex-col gap-1.5 text-[11px] text-muted-foreground">
              Wallet
              <select
                className="rounded-xl border border-white/10 bg-background px-3 py-2 text-sm text-foreground outline-none"
                value={selectedWallet}
                onChange={(event) => setSelectedWallet(event.target.value)}
              >
                <option value="">Select local wallet</option>
                {wallets.map((wallet) => (
                  <option key={wallet.address} value={wallet.address}>
                    {wallet.label ? `${wallet.label} · ` : ""}
                    {wallet.address.slice(0, 10)}…{wallet.address.slice(-6)}
                  </option>
                ))}
              </select>
            </label>
          </div>

          {templateDetail && (
            <>
              <div className="grid gap-3 lg:grid-cols-[1.25fr,0.75fr]">
                <div className="rounded-2xl border border-white/10 bg-white/5">
                  <div className="flex items-center gap-2 border-b border-white/10 px-4 py-3">
                    <ScrollText className="size-3.5 text-primary" />
                    <p className="text-[11px] font-semibold uppercase tracking-[0.16em] text-muted-foreground">
                      Solidity Preview
                    </p>
                  </div>
                  <pre className="max-h-[360px] overflow-auto px-4 py-3 text-[11px] leading-relaxed text-foreground/90">
                    {templateDetail.sourceCode}
                  </pre>
                </div>

                <div className="flex flex-col gap-3">
                  <InfoBlock
                    title={templateDetail.title}
                    body={templateDetail.summary}
                    meta={[
                      `Contract: ${templateDetail.contractName}`,
                      `Category: ${templateDetail.category}`,
                      `Recommended: ${templateDetail.recommendedNetwork}`,
                    ]}
                  />
                  <InfoBlock
                    title="Memory Overlay"
                    body="Each template seeds Beam-specific MEMORY.md and GUARDRAILS.md blocks into the workspace overlay before compile and deploy."
                    meta={[
                      "Launchpad context preserved",
                      "Beam guardrails appended once",
                      "Ready for durable handoff",
                    ]}
                  />
                  <InfoBlock
                    title="Wallet"
                    body={
                      selectedWallet
                        ? `${selectedWalletLabel ?? "Local wallet"} · ${selectedWallet}`
                        : "Create or select a local Beam wallet first."
                    }
                    meta={selectedWallet ? ["AES-256-GCM local vault", "Airlock L2 broadcast"] : []}
                  />
                </div>
              </div>

              <div className="flex flex-wrap items-center gap-3">
                <Button
                  variant="primary"
                  onPress={() => void handlePrepare()}
                  isDisabled={isPreparing || isDeploying || !selectedWallet}
                >
                  <Zap className="mr-1 size-4" />
                  {isPreparing ? "Preparing..." : "Beam Deploy"}
                </Button>
                <p className="text-[12px] text-muted-foreground">
                  One click prepares the execution contract, compiles with `solcjs`, estimates Beam gas,
                  and opens a review modal before Airlock asks for the final approval.
                </p>
              </div>
            </>
          )}

          {deploymentResult && (
            <div className="rounded-2xl border border-emerald-500/20 bg-emerald-500/10 p-4">
              <div className="flex items-center gap-2">
                <CheckCircle2 className="size-4 text-emerald-500" />
                <p className="text-sm font-medium text-emerald-500">
                  Deployment broadcasted
                </p>
              </div>
              <div className="mt-2 space-y-1 text-[12px] text-foreground/85">
                <p>Template: {deploymentResult.plan.template.title}</p>
                <p>Network: {deploymentResult.receipt.network}</p>
                <p>Transaction: {deploymentResult.receipt.txHash}</p>
                {deploymentResult.receipt.contractAddress && (
                  <p>Contract: {deploymentResult.receipt.contractAddress}</p>
                )}
              </div>
            </div>
          )}

          {error && (
            <div className="flex items-start gap-2 rounded-xl bg-red-500/10 px-3 py-2">
              <AlertCircle className="mt-0.5 size-3.5 shrink-0 text-red-500" />
              <p className="text-xs text-red-500">{error}</p>
            </div>
          )}
        </div>
      </div>

      <Modal isOpen={reviewOpen} onOpenChange={setReviewOpen}>
        <Modal.Backdrop className="bg-background/60 backdrop-blur-xl">
          <Modal.Container>
            <Modal.Dialog className="w-full max-w-2xl rounded-[28px] border border-white/10 bg-background/95 p-0">
              <Modal.Header className="border-b border-white/10 px-6 py-5">
                <div>
                  <Modal.Heading className="text-lg font-semibold text-foreground">
                    Review Beam Transaction
                  </Modal.Heading>
                  <p className="mt-1 text-[12px] text-muted-foreground">
                    This is the first confirmation. The second confirmation will be the Airlock
                    approval dialog before broadcast.
                  </p>
                </div>
              </Modal.Header>
              <Modal.Body className="space-y-4 px-6 py-5">
                {deployPlan ? (
                  <>
                    <ReviewRow label="Template" value={deployPlan.template.title} />
                    <ReviewRow label="Contract" value={deployPlan.transactionPreview.contractName} />
                    <ReviewRow label="Network" value={deployPlan.network} />
                    <ReviewRow label="Wallet" value={deployPlan.walletAddress} />
                    <ReviewRow
                      label="Gas Estimate"
                      value={`${deployPlan.gasEstimate.gasLimit.toLocaleString()} gas · ${deployPlan.gasEstimate.estimatedFeeBeam}`}
                    />
                    <ReviewRow
                      label="Compiler"
                      value={`${deployPlan.compilation.compilerVersion} · ${deployPlan.compilation.bytecodeSizeBytes} bytes`}
                    />
                    <div className="rounded-2xl border border-white/10 bg-white/5 p-4">
                      <p className="text-[11px] font-semibold uppercase tracking-[0.16em] text-muted-foreground">
                        Build Artifacts
                      </p>
                      <div className="mt-2 space-y-1 text-[12px] text-foreground/80">
                        <p>{deployPlan.compilation.abiPath}</p>
                        <p>{deployPlan.compilation.bytecodePath}</p>
                        <p>{deployPlan.memoryFilePath}</p>
                        <p>{deployPlan.guardrailsFilePath}</p>
                      </div>
                    </div>
                  </>
                ) : (
                  <p className="text-sm text-muted-foreground">Preparing deployment preview…</p>
                )}
              </Modal.Body>
              <Modal.Footer className="border-t border-white/10 px-6 py-5">
                <div className="flex w-full justify-end gap-3">
                  <Button variant="ghost" onPress={() => setReviewOpen(false)}>
                    Cancel
                  </Button>
                  <Button
                    variant="primary"
                    onPress={() => void handleDeploy()}
                    isDisabled={!deployPlan || isDeploying}
                  >
                    <Rocket className="mr-1 size-4" />
                    {isDeploying ? "Waiting for Airlock..." : "Continue to Airlock"}
                  </Button>
                </div>
              </Modal.Footer>
            </Modal.Dialog>
          </Modal.Container>
        </Modal.Backdrop>
      </Modal>
    </>
  );
}

function readErrorMessage(error: unknown, fallback: string) {
  if (error instanceof Error && error.message.trim()) {
    return error.message;
  }
  if (typeof error === "string" && error.trim()) {
    return error;
  }
  try {
    const serialized = JSON.stringify(error);
    if (serialized && serialized !== "{}") {
      return serialized;
    }
  } catch {
    // ignore serialization failure
  }
  return fallback;
}

function InfoBlock({
  title,
  body,
  meta,
}: {
  title: string;
  body: string;
  meta: string[];
}) {
  return (
    <div className="rounded-2xl border border-white/10 bg-white/5 p-4">
      <p className="text-sm font-medium text-foreground">{title}</p>
      <p className="mt-1 text-[12px] leading-relaxed text-muted-foreground">{body}</p>
      {meta.length > 0 && (
        <div className="mt-3 flex flex-wrap gap-2">
          {meta.map((item) => (
            <div
              key={`${title}-${item}`}
              className="rounded-lg border border-white/10 bg-background/60 px-2 py-1 text-[11px] text-foreground/75"
            >
              {item}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function ReviewRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-start justify-between gap-3 rounded-xl border border-white/10 bg-white/5 px-4 py-3 text-sm">
      <span className="text-muted-foreground">{label}</span>
      <span className="max-w-[65%] text-right text-foreground">{value}</span>
    </div>
  );
}
