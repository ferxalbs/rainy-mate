export const AirlockLevel = {
  Safe: 0,
  Sensitive: 1,
  Dangerous: 2,
} as const;
export type AirlockLevel = typeof AirlockLevel[keyof typeof AirlockLevel];

export interface ApprovalRequest {
  commandId: string;
  intent: string;
  toolName?: string | null;
  payloadSummary: string;
  airlockLevel: AirlockLevel;
  timeoutSecs?: number | null;
  expiresAt?: number | null;
  timestamp: number;
}
