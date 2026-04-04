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
  to: string;
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
}

export interface AllBeamChainConfigs {
  mainnet: BeamChainConfig;
  testnet: BeamChainConfig;
}
