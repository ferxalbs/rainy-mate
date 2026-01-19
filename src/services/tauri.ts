// Rainy Cowork - Tauri Service Layer
// Typed wrappers for Tauri command invocation

import { invoke, Channel } from '@tauri-apps/api/core';

// ============ Types ============

export type TaskStatus = 'queued' | 'running' | 'paused' | 'completed' | 'failed' | 'cancelled';
export type ProviderType = 'rainyApi' | 'gemini';
export type FileOperation = 'create' | 'modify' | 'delete' | 'move' | 'rename';

export interface Task {
    id: string;
    title: string;
    description: string;
    status: TaskStatus;
    progress: number;
    provider: ProviderType;
    model: string;
    workspacePath?: string;
    createdAt: string;
    startedAt?: string;
    completedAt?: string;
    error?: string;
}

export interface FileChange {
    id: string;
    path: string;
    filename: string;
    operation: FileOperation;
    timestamp: string;
    taskId?: string;
    versionId?: string;
}

export interface Workspace {
    id: string;
    path: string;
    name: string;
    accessType: 'read-only' | 'full-access';
    createdAt: string;
}

export interface AIProviderConfig {
    provider: ProviderType;
    name: string;
    model: string;
    isAvailable: boolean;
    requiresApiKey: boolean;
}

export interface FileEntry {
    name: string;
    path: string;
    isDirectory: boolean;
    size?: number;
    modified?: string;
}

export type TaskEvent =
    | { event: 'started'; data: { taskId: string } }
    | { event: 'progress'; data: { taskId: string; progress: number; message?: string } }
    | { event: 'stepCompleted'; data: { taskId: string; stepId: string } }
    | { event: 'completed'; data: { taskId: string } }
    | { event: 'failed'; data: { taskId: string; error: string } };

// ============ Task Commands ============

export async function createTask(
    description: string,
    provider: ProviderType,
    model: string,
    workspacePath?: string
): Promise<Task> {
    return invoke<Task>('create_task', {
        description,
        provider,
        model,
        workspacePath,
    });
}

export async function executeTask(
    taskId: string,
    onEvent: (event: TaskEvent) => void
): Promise<void> {
    const channel = new Channel<TaskEvent>();
    channel.onmessage = onEvent;

    return invoke<void>('execute_task', {
        taskId,
        onEvent: channel,
    });
}

export async function pauseTask(taskId: string): Promise<void> {
    return invoke<void>('pause_task', { taskId });
}

export async function resumeTask(taskId: string): Promise<void> {
    return invoke<void>('resume_task', { taskId });
}

export async function cancelTask(taskId: string): Promise<void> {
    return invoke<void>('cancel_task', { taskId });
}

export async function getTask(taskId: string): Promise<Task | null> {
    return invoke<Task | null>('get_task', { taskId });
}

export async function listTasks(): Promise<Task[]> {
    return invoke<Task[]>('list_tasks');
}

// ============ AI Provider Commands ============

export async function listProviders(): Promise<AIProviderConfig[]> {
    return invoke<AIProviderConfig[]>('list_providers');
}

export async function validateApiKey(provider: string, apiKey: string): Promise<boolean> {
    return invoke<boolean>('validate_api_key', { provider, apiKey });
}

export async function storeApiKey(provider: string, apiKey: string): Promise<void> {
    return invoke<void>('store_api_key', { provider, apiKey });
}

export async function getApiKey(provider: string): Promise<string | null> {
    return invoke<string | null>('get_api_key', { provider });
}

export async function deleteApiKey(provider: string): Promise<void> {
    return invoke<void>('delete_api_key', { provider });
}

export async function getProviderModels(provider: string): Promise<string[]> {
    return invoke<string[]>('get_provider_models', { provider });
}

export async function hasApiKey(provider: string): Promise<boolean> {
    return invoke<boolean>('has_api_key', { provider });
}

// ============ Cowork Status Types ============

export interface CoworkFeatures {
    web_research: boolean;
    document_export: boolean;
    image_analysis: boolean;
    priority_support: boolean;
}

export interface CoworkUsage {
    used: number;
    limit: number;
    credits_used: number;
    credits_ceiling: number;
    resets_at: string;
}

export type CoworkPlan = 'free' | 'go_plus' | 'plus' | 'pro' | 'pro_plus';

export interface CoworkStatus {
    has_paid_plan: boolean;
    plan: CoworkPlan;
    plan_name: string;
    is_valid: boolean;
    models: string[];
    features: CoworkFeatures;
    usage: CoworkUsage;
    upgrade_message: string | null;
}

// ============ Cowork Commands ============

export async function getCoworkStatus(): Promise<CoworkStatus> {
    return invoke<CoworkStatus>('get_cowork_status');
}

export async function canUseFeature(feature: string): Promise<boolean> {
    return invoke<boolean>('can_use_feature', { feature });
}

// ============ File Commands ============

export async function setWorkspace(path: string, name: string): Promise<Workspace> {
    return invoke<Workspace>('set_workspace', { path, name });
}

export async function getWorkspace(): Promise<Workspace | null> {
    return invoke<Workspace | null>('get_workspace');
}

export async function listDirectory(path: string): Promise<FileEntry[]> {
    return invoke<FileEntry[]>('list_directory', { path });
}

export async function readFile(path: string): Promise<string> {
    return invoke<string>('read_file', { path });
}

export async function writeFile(
    path: string,
    content: string,
    taskId?: string
): Promise<FileChange> {
    return invoke<FileChange>('write_file', { path, content, taskId });
}

export async function createSnapshot(path: string, taskId: string): Promise<string> {
    return invoke<string>('create_snapshot', { path, taskId });
}

export async function rollbackFile(versionId: string): Promise<void> {
    return invoke<void>('rollback_file', { versionId });
}

export async function listFileChanges(taskId?: string): Promise<FileChange[]> {
    return invoke<FileChange[]>('list_file_changes', { taskId });
}

// ============ User Folder Types ============

export type FolderAccess = 'read-only' | 'full-access';

export interface UserFolder {
    id: string;
    path: string;
    name: string;
    accessType: FolderAccess;
    addedAt: string;
    lastAccessed: string;
}

// ============ User Folder Commands ============

export async function addUserFolder(path: string, name: string): Promise<UserFolder> {
    return invoke<UserFolder>('add_user_folder', { path, name });
}

export async function listUserFolders(): Promise<UserFolder[]> {
    return invoke<UserFolder[]>('list_user_folders');
}

export async function removeUserFolder(id: string): Promise<void> {
    return invoke<void>('remove_user_folder', { id });
}

export async function updateFolderAccess(id: string): Promise<void> {
    return invoke<void>('update_folder_access', { id });
}
