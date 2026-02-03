export enum AirlockLevel {
  Safe = 0,
  Sensitive = 1,
  Dangerous = 2,
}

export interface ApprovalRequest {
  commandId: string;
  intent: string;
  payloadSummary: string;
  airlockLevel: AirlockLevel;
  timestamp: number;
}
