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

/** A file attached by the user to a chat message. */
export interface ChatAttachment {
  id: string;
  /** Original file path (used when submitting to Tauri). */
  path: string;
  filename: string;
  mimeType: string;
  sizeBytes: number;
  /** "image" | "document" | "text" | "unknown" */
  type: string;
  /** Base64 data URI for image thumbnails (frontend preview only). */
  thumbnailDataUri?: string;
}

export type ChatArtifactKind = "image" | "pdf" | "docx" | "xlsx";
export type ChatArtifactOpenMode = "inline" | "preview" | "system_default";
export type ChatArtifactAction = "open";

export interface ChatArtifact {
  id: string;
  path: string;
  filename: string;
  kind: ChatArtifactKind;
  mimeType: string;
  openMode: ChatArtifactOpenMode;
  availableActions: ChatArtifactAction[];
  originTool: string;
}

export interface AgentMessage {
  id: string;
  type: "user" | "agent" | "system";
  content: string;
  /** Files attached to this message (user messages only). */
  attachments?: ChatAttachment[];
  artifacts?: ChatArtifact[];
  isLoading?: boolean;
  timestamp: Date;
  thought?: string;
  thinkingLevel?: "minimal" | "low" | "medium" | "high";
  modelUsed?: {
    name: string;
    thinkingEnabled: boolean;
  };
  thoughtDuration?: number; // Duration in ms
  plan?: TaskPlan;
  result?: ExecutionResult;
  toolCalls?: Array<{
    skill: string;
    method: string;
    params: Record<string, any>;
  }>;
  isExecuted?: boolean;
  /** Real-time neural state from backend agent events */
  neuralState?: string;
  /** Name of the tool currently being executed by the agent */
  activeToolName?: string;
  /** Highest Airlock tier reached so far this run (0=Safe, 1=Sensitive, 2=Dangerous). Ratchets up, never down. */
  airlockLevel?: number;
  supervisorPlan?: {
    summary: string;
    steps: string[];
    verificationRequired?: boolean;
    mode?: string;
    delegationPolicy?: string;
    maxDepth?: number;
    maxThreads?: number;
    maxParallelSubagents?: number;
    internalCoordinationLanguage?: string;
    finalResponseLanguageMode?: string;
  };
  specialists?: SpecialistRunState[];
  ragTelemetry?: {
    historySource?: string;
    retrievalMode?: string;
    embeddingProfile?: string;
    executionMode?: string;
    workspaceMemoryEnabled?: boolean;
    workspaceMemoryRoot?: string;
    lastModel?: string;
    promptTokens?: number;
    completionTokens?: number;
    totalTokens?: number;
    compressionApplied?: boolean;
    compressionTriggerTokens?: number;
  };
  runState?: "running" | "completed" | "cancelled" | "failed";
  requestContext?: {
    runId?: string;
    prompt?: string;
    modelId?: string;
    reasoningEffort?: string;
    workspaceId?: string;
    agentSpecId?: string;
    chatScopeId?: string;
    startedAtMs?: number;
    completedAtMs?: number;
  };
  trace?: AgentTraceEntry[];
}

export interface AgentTraceEntry {
  id: string;
  phase:
    | "think"
    | "act"
    | "tool"
    | "approval"
    | "retry"
    | "error"
    | "done"
    | "cancelled";
  label: string;
  timestamp: Date;
  attempt?: number;
  toolName?: string;
  preview?: string;
}

export interface SpecialistRunState {
  agentId: string;
  role: string;
  status:
    | "pending"
    | "planning"
    | "running"
    | "waiting_on_airlock"
    | "verifying"
    | "completed"
    | "failed"
    | "cancelled";
  detail?: string;
  activeTool?: string;
  summary?: string;
  responsePreview?: string;
  error?: string;
  dependsOn?: string[];
  startedAtMs?: number;
  finishedAtMs?: number;
  toolCount?: number;
  writeLikeUsed?: boolean;
  parentAgentId?: string;
  depth?: number;
  branchId?: string;
  spawnReason?: string;
}
