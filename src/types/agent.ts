// src/types/agent.ts
// Agent Types for Frontend
// Part of Phase 3 - Rainy Cowork

/**
 * Agent task status
 */
export type AgentTaskStatus = "pending" | "running" | "completed" | "failed";

/**
 * Agent task type
 */
export type AgentTaskType = "document" | "research";

/**
 * Agent task progress event
 */
export interface AgentProgress {
  taskId: string;
  step: string;
  progress: number;
  message?: string;
}

/**
 * Agent task result
 */
export interface AgentResult {
  success: boolean;
  content?: string;
  error?: string;
  network?: string;
  generatedAt?: string;
}

/**
 * Agent task representation
 */
export interface AgentTask {
  id: string;
  type: AgentTaskType;
  status: AgentTaskStatus;
  prompt: string;
  templateId?: string;
  progress: number;
  currentStep?: string;
  result?: AgentResult;
  createdAt: string;
  completedAt?: string;
}

/**
 * Document generation request
 */
export interface DocumentGenerateRequest {
  prompt: string;
  templateId?: string;
  context?: Record<string, unknown>;
  async?: boolean;
}

/**
 * Research request
 */
export interface ResearchRequest {
  topic: string;
  depth?: "basic" | "advanced";
  maxSources?: number;
  async?: boolean;
}

/**
 * Agent feature availability
 */
export interface AgentFeatureStatus {
  available: boolean;
  tier?: string;
  message?: string;
}

/**
 * Agent status response
 */
export interface AgentStatus {
  features: {
    document_generation: AgentFeatureStatus;
    web_research: AgentFeatureStatus;
  };
}

/**
 * Document template info
 */
export interface DocumentTemplate {
  id: string;
  name: string;
  description: string;
  category: string;
}

// ============ Chat & Execution Types ============

export interface PlanStep {
  type:
    | "createFile"
    | "modifyFile"
    | "deleteFile"
    | "moveFile"
    | "organizeFolder"
    | "default";
  description: string;
}

export interface TaskPlan {
  id: string;
  steps: PlanStep[];
  warnings: string[];
}

export interface ExecutionResult {
  totalSteps: number;
  totalChanges: number;
  errors: string[];
}

export interface AgentMessage {
  id: string;
  type: "user" | "agent" | "system";
  content: string;
  isLoading?: boolean;
  timestamp: Date;
  thought?: string;
  thinkingLevel?: "minimal" | "low" | "medium" | "high";
  modelUsed?: {
    name: string;
    thinkingEnabled: boolean;
  };
  plan?: TaskPlan;
  result?: ExecutionResult;
}
