// Beam RPC + Secure Local Signing Bridge — TypeScript types
// Matches the Rust structs in src-tauri/src/services/beam_rpc.rs

export type BeamNetworkId = "mainnet" | "testnet";

export interface BeamChainConfig {
  network: string;
  chainId: number;
  chainIdHex: string;
  rpcUrl: string;
  wsUrl: string;
  explorerUrl: string;
  currencySymbol: string;
  currencyDecimals: number;
}

export interface BeamWorkspaceConfig {
  schemaVersion: number;
  connectedAt: string;
  network: string;
  chain: BeamChainConfig;
}

export interface WalletInfo {
  address: string;
  label: string | null;
  createdAt: number;
}

export interface GasEstimate {
  gasLimit: number;
  gasPrice: number;
  estimatedFeeWei: string;
  estimatedFeeBeam: string;
  network: string;
  rpcUrl: string;
}

export interface SignedTransaction {
  rawTx: string;
  txHash: string;
  from: string;
  to: string | null;
  valueHex: string;
  gasLimit: number;
  gasPrice: number;
  nonce: number;
  chainId: number;
  network: string;
}

export interface TransactionReceipt {
  txHash: string;
  network: string;
  explorerUrl: string;
  status?: string | null;
  blockNumber?: number | null;
  gasUsed?: number | null;
  contractAddress?: string | null;
}

export interface AllBeamChainConfigs {
  mainnet: BeamChainConfig;
  testnet: BeamChainConfig;
}

export interface BeamTemplateSummary {
  id: string;
  title: string;
  summary: string;
  description: string;
  contractName: string;
  contractFile: string;
  category: string;
  recommendedNetwork: string;
  tags: string[];
}

export interface BeamTemplateDetail {
  id: string;
  title: string;
  summary: string;
  description: string;
  contractName: string;
  contractFile: string;
  category: string;
  recommendedNetwork: string;
  tags: string[];
  templateRoot: string;
  sourceCode: string;
  memoryMarkdown: string;
  guardrailsMarkdown: string;
}

export interface BeamTemplateScaffoldResult {
  templateId: string;
  workspacePath: string;
  sourcePath: string;
  memoryFilePath: string;
  guardrailsFilePath: string;
  scaffoldedFiles: string[];
}

export interface BeamCompilationArtifact {
  abi: unknown;
  abiPath: string;
  bytecode: string;
  bytecodePath: string;
  bytecodeSizeBytes: number;
  compilerVersion: string;
}

export interface BeamDeploymentTransactionPreview {
  kind: string;
  from: string;
  to: string | null;
  network: string;
  chainId: number;
  gasLimit: number;
  gasPrice: number;
  estimatedFeeWei: string;
  estimatedFeeBeam: string;
  explorerUrl: string;
  rpcUrl: string;
  dataBytes: number;
  contractName: string;
}

export interface BeamDeploymentPlan {
  requestId?: string | null;
  workspacePath: string;
  template: BeamTemplateSummary;
  network: string;
  walletAddress: string;
  sourcePath: string;
  buildDir: string;
  scaffoldedFiles: string[];
  memoryFilePath: string;
  guardrailsFilePath: string;
  compilation: BeamCompilationArtifact;
  gasEstimate: GasEstimate;
  transactionPreview: BeamDeploymentTransactionPreview;
}

export interface BeamDeploymentResult {
  plan: BeamDeploymentPlan;
  receipt: TransactionReceipt;
}
