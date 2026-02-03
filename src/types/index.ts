// Rainy Cowork - Type Definitions
// Updated for Rainy API + Gemini provider model

export * from "./web";
export * from "./document";
export * from "./image";
export * from "./theme";
export * from "./agent";
export * from "./workspace";
export * from "./versioning";
export * from "./neural";

/**
 * Task status enum for tracking task lifecycle
 */
export type TaskStatus =
  | "queued"
  | "running"
  | "paused"
  | "completed"
  | "failed"
  | "cancelled";

/**
 * AI Provider identifiers
 * - rainyApi: Enosis Labs backend (GPT-4, Claude, etc. via OpenAI format)
 * - gemini: User's own Google Gemini API key
 */
export type ProviderType = "rainyApi" | "coworkApi" | "gemini";

/**
 * File operation types for tracking changes
 */
export type FileOperation = "create" | "modify" | "delete" | "move" | "rename";

/**
 * AI Provider configuration
 */
export interface AIProvider {
  id: ProviderType;
  name: string;
  model: string;
  isAvailable: boolean;
  requiresApiKey: boolean;
  description?: string;
}

/**
 * Task definition
 */
export interface Task {
  id: string;
  title: string;
  description?: string;
  status: TaskStatus;
  progress: number; // 0-100
  provider: ProviderType;
  model: string;
  workspacePath?: string;
  createdAt: Date;
  startedAt?: Date;
  completedAt?: Date;
  error?: string;
  steps?: TaskStep[];
}

/**
 * Individual task step for detailed progress
 */
export interface TaskStep {
  id: string;
  name: string;
  status: "pending" | "running" | "completed" | "failed";
  startedAt?: Date;
  completedAt?: Date;
}

/**
 * File change record
 */
export interface FileChange {
  id: string;
  path: string;
  filename: string;
  operation: FileOperation;
  timestamp: Date;
  taskId?: string;
  previousPath?: string; // For move/rename operations
  versionId?: string; // For undo/redo
}

/**
 * Folder with access permissions (workspace)
 */
export interface Folder {
  id: string;
  path: string;
  name: string;
  accessType: "read-only" | "full-access";
  isExpanded?: boolean;
}

/**
 * Application settings
 */
export interface AppSettings {
  theme: "light" | "dark" | "system";
  defaultProvider: ProviderType;
  sidebarCollapsed: boolean;
  showNotifications: boolean;
}

/**
 * Available AI providers with their configurations
 */
export const AI_PROVIDERS: AIProvider[] = [
  {
    id: "rainyApi",
    name: "Rainy API (Pay-As-You-Go)",
    model: "gpt-4o",
    isAvailable: true,
    requiresApiKey: true,
    description: "Standard API usage (1:1 Dollar/Token)",
  },
  {
    id: "coworkApi",
    name: "Cowork Subscription",
    model: "gemini-3-pro-preview",
    isAvailable: true,
    requiresApiKey: true,
    description: "Monthly credits plan (Free, Plus, Pro)",
  },
  {
    id: "gemini",
    name: "Google Gemini",
    model: "gemini-3-pro-preview",
    isAvailable: true,
    requiresApiKey: true,
    description: "Gemini 3 with thinking capabilities",
  },
];

/**
 * Model options per provider
 */
export const PROVIDER_MODELS: Record<ProviderType, string[]> = {
  rainyApi: [
    "gpt-4o",
    "gpt-4o-mini",
    "gpt-4-turbo",
    "claude-3.5-sonnet",
    "claude-3-opus",
  ],
  coworkApi: [
    "gemini-3-pro-preview",
    "gemini-3-flash-preview",
    "gemini-2.5-pro",
    "gemini-2.5-flash",
  ],
  gemini: [
    // Gemini 3 - Latest with thinking levels
    "gemini-3-pro-preview",
    "gemini-3-flash-preview",
    // Gemini 2.5 - Thinking budget
    "gemini-2.5-pro",
    "gemini-2.5-flash",
    "gemini-2.5-flash-lite",
  ],
};

/**
 * Thinking levels for Gemini 3 models
 */
export type ThinkingLevel = "minimal" | "low" | "medium" | "high";

/**
 * Model metadata with thinking capabilities
 */
export const GEMINI_MODEL_INFO: Record<
  string,
  {
    name: string;
    description: string;
    thinkingLevels?: ThinkingLevel[];
  }
> = {
  "gemini-3-pro-preview": {
    name: "Gemini 3 Pro (Preview)",
    description: "Most intelligent - reasoning, coding, agents",
    thinkingLevels: ["low", "high"],
  },
  "gemini-3-flash-preview": {
    name: "Gemini 3 Flash (Preview)",
    description: "Fast with good quality - general backend",
    thinkingLevels: ["minimal", "low", "medium", "high"],
  },
  "gemini-2.5-pro": {
    name: "Gemini 2.5 Pro",
    description: "Deep analysis, long context",
  },
  "gemini-2.5-flash": {
    name: "Gemini 2.5 Flash",
    description: "General backend, high QPS",
  },
  "gemini-2.5-flash-lite": {
    name: "Gemini 2.5 Flash Lite",
    description: "Cost-sensitive, minimal latency",
  },
};
