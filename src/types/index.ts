// Rainy Cowork - Type Definitions
// Updated for Rainy API + Gemini provider model

/**
 * Task status enum for tracking task lifecycle
 */
export type TaskStatus = 'queued' | 'running' | 'paused' | 'completed' | 'failed' | 'cancelled';

/**
 * AI Provider identifiers
 * - rainyApi: Enosis Labs backend (GPT-4, Claude, etc. via OpenAI format)
 * - gemini: User's own Google Gemini API key
 */
export type ProviderType = 'rainyApi' | 'gemini';

/**
 * File operation types for tracking changes
 */
export type FileOperation = 'create' | 'modify' | 'delete' | 'move' | 'rename';

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
    status: 'pending' | 'running' | 'completed' | 'failed';
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
    accessType: 'read-only' | 'full-access';
    isExpanded?: boolean;
}

/**
 * Application settings
 */
export interface AppSettings {
    theme: 'light' | 'dark' | 'system';
    defaultProvider: ProviderType;
    sidebarCollapsed: boolean;
    showNotifications: boolean;
}

/**
 * Available AI providers with their configurations
 */
export const AI_PROVIDERS: AIProvider[] = [
    {
        id: 'rainyApi',
        name: 'Rainy API',
        model: 'gpt-4o',
        isAvailable: true,
        requiresApiKey: true,
        description: 'Access GPT-4, Claude & more via Enosis Labs',
    },
    {
        id: 'gemini',
        name: 'Google Gemini',
        model: 'gemini-1.5-pro',
        isAvailable: true,
        requiresApiKey: true,
        description: 'Use your own Google API key',
    },
];

/**
 * Model options per provider
 */
export const PROVIDER_MODELS: Record<ProviderType, string[]> = {
    rainyApi: ['gpt-4o', 'gpt-4o-mini', 'gpt-4-turbo', 'claude-3.5-sonnet', 'claude-3-opus'],
    gemini: ['gemini-1.5-pro', 'gemini-1.5-flash', 'gemini-2.0-flash-exp'],
};
