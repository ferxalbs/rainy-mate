// Rainy Cowork - Workspace Types
// Type definitions for advanced workspace management

/**
 * Workspace permissions
 */
export interface WorkspacePermissions {
    canRead: boolean;
    canWrite: boolean;
    canExecute: boolean;
    canDelete: boolean;
    canCreateAgents: boolean;
}

/**
 * Permission override for a specific path
 */
export interface PermissionOverride {
    path: string;
    permissions: WorkspacePermissions;
    inherited: boolean;
}

/**
 * Workspace settings
 */
export interface WorkspaceSettings {
    theme: string;
    language: string;
    autoSave: boolean;
    notificationsEnabled: boolean;
}

/**
 * Agent configuration within a workspace
 */
export interface AgentConfig {
    id: string;
    name: string;
    agentType: string;
    config: Record<string, any>;
}

/**
 * Workspace memory configuration
 */
export interface WorkspaceMemory {
    maxSize: number;
    currentSize: number;
    retentionPolicy: string;
}

/**
 * Advanced workspace with agents and memory management
 */
export interface Workspace {
    id: string;
    name: string;
    allowedPaths: string[];
    permissions: WorkspacePermissions;
    permissionOverrides: PermissionOverride[];
    agents: AgentConfig[];
    memory: WorkspaceMemory;
    settings: WorkspaceSettings;
}

/**
 * Workspace template for creation
 */
export interface WorkspaceTemplate {
    id: string;
    name: string;
    description: string;
    category: string;
    defaultPermissions: WorkspacePermissions;
    defaultSettings: WorkspaceSettings;
    defaultMemory: WorkspaceMemory;
    suggestedPaths?: string[];
}

/**
 * Predefined workspace templates
 */
export const WORKSPACE_TEMPLATES: WorkspaceTemplate[] = [
    {
        id: 'development',
        name: 'Development Workspace',
        description: 'Full-featured workspace for software development with code analysis agents',
        category: 'Development',
        defaultPermissions: {
            canRead: true,
            canWrite: true,
            canExecute: true,
            canDelete: false,
            canCreateAgents: true,
        },
        defaultSettings: {
            theme: 'dark',
            language: 'en',
            autoSave: true,
            notificationsEnabled: true,
        },
        defaultMemory: {
            maxSize: 104857600, // 100MB
            currentSize: 0,
            retentionPolicy: 'fifo',
        },
        suggestedPaths: ['src', 'tests', 'docs'],
    },
    {
        id: 'research',
        name: 'Research Workspace',
        description: 'Workspace optimized for research and documentation with AI research agents',
        category: 'Research',
        defaultPermissions: {
            canRead: true,
            canWrite: true,
            canExecute: false,
            canDelete: false,
            canCreateAgents: true,
        },
        defaultSettings: {
            theme: 'light',
            language: 'en',
            autoSave: true,
            notificationsEnabled: true,
        },
        defaultMemory: {
            maxSize: 524288000, // 500MB
            currentSize: 0,
            retentionPolicy: 'lru',
        },
        suggestedPaths: ['research', 'notes', 'references'],
    },
    {
        id: 'minimal',
        name: 'Minimal Workspace',
        description: 'Basic workspace with minimal permissions for simple file operations',
        category: 'General',
        defaultPermissions: {
            canRead: true,
            canWrite: true,
            canExecute: false,
            canDelete: false,
            canCreateAgents: false,
        },
        defaultSettings: {
            theme: 'system',
            language: 'en',
            autoSave: false,
            notificationsEnabled: false,
        },
        defaultMemory: {
            maxSize: 10485760, // 10MB
            currentSize: 0,
            retentionPolicy: 'fifo',
        },
    },
];

/**
 * Workspace creation options
 */
export interface CreateWorkspaceOptions {
    name: string;
    templateId?: string;
    allowedPaths: string[];
    customPermissions?: Partial<WorkspacePermissions>;
    customSettings?: Partial<WorkspaceSettings>;
}