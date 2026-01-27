// Rainy Cowork - Tauri Service Layer
// Typed wrappers for Tauri command invocation

import { invoke, Channel } from '@tauri-apps/api/core';

// ============ Types ============

export type TaskStatus = 'queued' | 'running' | 'paused' | 'completed' | 'failed' | 'cancelled';
export type ProviderType = 'rainyApi' | 'coworkApi' | 'gemini';
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

export type CoworkPlan = 'free' | 'go' | 'go_plus' | 'plus' | 'pro' | 'pro_plus';

export interface CoworkStatus {
    has_paid_plan: boolean;
    plan: string; // Changed from CoworkPlan union to string to support all backend IDs
    plan_name: string;
    is_valid: boolean;
    models: string[];
    features: CoworkFeatures;
    usage: CoworkUsage;
    upgrade_message: string | null;
}

// ============ Cowork Commands ============

export interface CoworkModelsResponse {
    plan: string;
    plan_name: string;
    model_access_level: string;
    models: string[];
    total_models: number;
}

export async function getCoworkStatus(): Promise<CoworkStatus> {
    return invoke<CoworkStatus>('get_cowork_status');
}

export async function getCoworkModels(): Promise<CoworkModelsResponse> {
    return invoke<CoworkModelsResponse>('get_cowork_models');
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

// ============ File Operations Types (AI Agent) ============

export type ConflictStrategy = 'skip' | 'overwrite' | 'rename' | 'ask';
export type OrganizeStrategy = 'by_type' | 'by_date' | 'by_extension' | 'by_content';
export type FileOpType = 'move' | 'copy' | 'rename' | 'delete' | 'create' | 'create_folder';

export interface FileOpChange {
    id: string;
    operation: FileOpType;
    sourcePath: string;
    destPath?: string;
    timestamp: string;
    reversible: boolean;
}

export interface RenamePreview {
    original: string;
    newName: string;
    hasConflict: boolean;
}

export interface OrganizeResult {
    filesMoved: number;
    foldersCreated: number;
    skipped: number;
    errors: string[];
    changes: FileOpChange[];
}

export interface FileTypeStats {
    count: number;
    totalSize: number;
    extensions: string[];
}

export interface FileInfo {
    path: string;
    name: string;
    size: number;
    modified: string;
}

export interface DuplicateGroup {
    size: number;
    files: string[];
}

export interface OptimizationSuggestion {
    suggestionType: 'delete_duplicates' | 'archive_old_files' | 'organize_by_type' | 'compress_images' | 'clean_temp_files';
    description: string;
    potentialSavings?: number;
    affectedFiles: string[];
}

export interface WorkspaceAnalysis {
    totalFiles: number;
    totalFolders: number;
    totalSizeBytes: number;
    fileTypes: Record<string, FileTypeStats>;
    largestFiles: FileInfo[];
    duplicateCandidates: DuplicateGroup[];
    suggestions: OptimizationSuggestion[];
}

// ============ AI Agent Types ============

export type TaskIntent = 'question' | 'command';

export interface ModelInfo {
    provider: string;
    model: string;
    planTier: string;
}

export interface PlannedStep {
    type: 'createFile' | 'modifyFile' | 'moveFile' | 'deleteFile' | 'organizeFolder' | 'batchRename' | 'analyzeContent';
    path?: string;
    source?: string;
    destination?: string;
    content?: string;
    instruction?: string;
    strategy?: OrganizeStrategy;
    files?: string[];
    pattern?: string;
    description: string;
}

export interface TaskPlan {
    id: string;
    instruction: string;
    intent: TaskIntent;
    answer?: string;
    modelUsed?: ModelInfo;
    steps: PlannedStep[];
    estimatedChanges: number;
    requiresConfirmation: boolean;
    warnings: string[];
    createdAt: string;
}

export interface ExecutionResult {
    taskId: string;
    success: boolean;
    totalSteps: number;
    completedSteps: number;
    totalChanges: number;
    changes: FileOpChange[];
    errors: string[];
    durationMs: number;
}

export type AgentEvent =
    | { event: 'planningStarted'; data: { taskId: string } }
    | { event: 'planReady'; data: { taskId: string; plan: TaskPlan } }
    | { event: 'stepStarted'; data: { taskId: string; stepIndex: number; description: string } }
    | { event: 'stepCompleted'; data: { taskId: string; stepIndex: number; changes: FileOpChange[] } }
    | { event: 'stepFailed'; data: { taskId: string; stepIndex: number; error: string } }
    | { event: 'progress'; data: { taskId: string; progress: number; message: string } }
    | { event: 'completed'; data: { taskId: string; totalChanges: number } }
    | { event: 'failed'; data: { taskId: string; error: string } }
    | { event: 'confirmationRequired'; data: { taskId: string; message: string; affectedFiles: string[] } };

// ============ File Operations Commands ============

export async function moveFiles(
    paths: string[],
    destination: string,
    onConflict?: ConflictStrategy
): Promise<FileOpChange[]> {
    return invoke<FileOpChange[]>('move_files', { paths, destination, onConflict });
}

export async function organizeFolder(
    path: string,
    strategy: OrganizeStrategy,
    dryRun?: boolean
): Promise<OrganizeResult> {
    return invoke<OrganizeResult>('organize_folder', { path, strategy, dryRun });
}

export async function batchRename(
    files: string[],
    pattern: string,
    find?: string,
    replace?: string,
    counterStart?: number,
    previewOnly?: boolean
): Promise<RenamePreview[]> {
    return invoke<RenamePreview[]>('batch_rename', {
        files,
        pattern,
        find,
        replace,
        counterStart,
        previewOnly,
    });
}

export async function safeDeleteFiles(paths: string[]): Promise<FileOpChange[]> {
    return invoke<FileOpChange[]>('safe_delete_files', { paths });
}

export async function analyzeWorkspace(path: string): Promise<WorkspaceAnalysis> {
    return invoke<WorkspaceAnalysis>('analyze_workspace', { path });
}

export async function undoFileOperation(operationId: string): Promise<FileOpChange[]> {
    return invoke<FileOpChange[]>('undo_file_operation', { operationId });
}

export async function listFileOperations(): Promise<[string, string, string][]> {
    return invoke<[string, string, string][]>('list_file_operations');
}

// ============ AI Agent Commands ============

export async function planTask(
    instruction: string,
    workspacePath: string
): Promise<TaskPlan> {
    return invoke<TaskPlan>('plan_task', { instruction, workspacePath });
}

export async function executeAgentTask(
    planId: string,
    onEvent: (event: AgentEvent) => void
): Promise<ExecutionResult> {
    const channel = new Channel<AgentEvent>();
    channel.onmessage = onEvent;

    return invoke<ExecutionResult>('execute_agent_task', {
        planId,
        onEvent: channel,
    });
}

export async function getAgentPlan(planId: string): Promise<TaskPlan | null> {
    return invoke<TaskPlan | null>('get_agent_plan', { planId });
}

export async function cancelAgentPlan(planId: string): Promise<void> {
    return invoke<void>('cancel_agent_plan', { planId });
}

export async function agentAnalyzeWorkspace(path: string): Promise<WorkspaceAnalysis> {
    return invoke<WorkspaceAnalysis>('agent_analyze_workspace', { path });
}

// ============ Settings Types ============

export interface ModelOption {
    id: string;
    name: string;
    description: string;
    thinkingLevel: string;
    isPremium: boolean;
    isAvailable: boolean;
    provider: string;
}

export interface UserSettings {
    selectedModel: string;
    theme: string;
    notificationsEnabled: boolean;
}

// ============ Settings Commands ============

export async function getUserSettings(): Promise<UserSettings> {
    return invoke<UserSettings>('get_user_settings');
}

export async function getSelectedModel(): Promise<string> {
    return invoke<string>('get_selected_model');
}

export async function setSelectedModel(model: string): Promise<void> {
    return invoke<void>('set_selected_model', { model });
}

export async function setTheme(theme: string): Promise<void> {
    return invoke<void>('set_theme', { theme });
}

export async function setNotifications(enabled: boolean): Promise<void> {
    return invoke<void>('set_notifications', { enabled });
}

export async function getAvailableModels(): Promise<ModelOption[]> {
    return invoke<ModelOption[]>('get_available_models');
}

// ============ Workspace Types ============

export interface AdvancedWorkspace {
    id: string;
    name: string;
    allowedPaths: string[];
    permissions: {
        canRead: boolean;
        canWrite: boolean;
        canExecute: boolean;
        canDelete: boolean;
        canCreateAgents: boolean;
    };
    permissionOverrides: Array<{
        path: string;
        permissions: {
            canRead: boolean;
            canWrite: boolean;
            canExecute: boolean;
            canDelete: boolean;
            canCreateAgents: boolean;
        };
        inherited: boolean;
    }>;
    agents: Array<{
        id: string;
        name: string;
        agentType: string;
        config: Record<string, any>;
    }>;
    memory: {
        maxSize: number;
        currentSize: number;
        retentionPolicy: string;
    };
    settings: {
        theme: string;
        language: string;
        autoSave: boolean;
        notificationsEnabled: boolean;
    };
}

// ============ Workspace Commands ============

export async function createWorkspace(
    name: string,
    allowedPaths: string[]
): Promise<AdvancedWorkspace> {
    return invoke<AdvancedWorkspace>('create_workspace', { name, allowedPaths });
}

export async function loadWorkspace(id: string): Promise<AdvancedWorkspace> {
    return invoke<AdvancedWorkspace>('load_workspace', { id });
}

export async function saveWorkspace(
    workspace: AdvancedWorkspace,
    format: 'json' | 'toml' = 'json'
): Promise<void> {
    return invoke<void>('save_workspace', { workspace, format });
}

export async function listWorkspaces(): Promise<string[]> {
    return invoke<string[]>('list_workspaces');
}

export async function deleteWorkspace(id: string): Promise<void> {
    return invoke<void>('delete_workspace', { id });
}

export async function getWorkspaceTemplates(): Promise<Array<{
    id: string;
    name: string;
    description: string;
    category: string;
    defaultPermissions: {
        canRead: boolean;
        canWrite: boolean;
        canExecute: boolean;
        canDelete: boolean;
        canCreateAgents: boolean;
    };
    defaultSettings: {
        theme: string;
        language: string;
        autoSave: boolean;
        notificationsEnabled: boolean;
    };
    defaultMemory: {
        maxSize: number;
        currentSize: number;
        retentionPolicy: string;
    };
    suggestedPaths: string[];
}>> {
    return invoke<Array<{
        id: string;
        name: string;
        description: string;
        category: string;
        defaultPermissions: {
            canRead: boolean;
            canWrite: boolean;
            canExecute: boolean;
            canDelete: boolean;
            canCreateAgents: boolean;
        };
        defaultSettings: {
            theme: string;
            language: string;
            autoSave: boolean;
            notificationsEnabled: boolean;
        };
        defaultMemory: {
            maxSize: number;
            currentSize: number;
            retentionPolicy: string;
        };
        suggestedPaths: string[];
    }>>('get_workspace_templates');
}

export async function createWorkspaceFromTemplate(
    templateId: string,
    name: string,
    customPaths?: string[]
): Promise<AdvancedWorkspace> {
    return invoke<AdvancedWorkspace>('create_workspace_from_template', {
        templateId,
        name,
        customPaths,
    });
}

export async function saveWorkspaceTemplate(template: {
    id: string;
    name: string;
    description: string;
    category: string;
    defaultPermissions: {
        canRead: boolean;
        canWrite: boolean;
        canExecute: boolean;
        canDelete: boolean;
        canCreateAgents: boolean;
    };
    defaultSettings: {
        theme: string;
        language: string;
        autoSave: boolean;
        notificationsEnabled: boolean;
    };
    defaultMemory: {
        maxSize: number;
        currentSize: number;
        retentionPolicy: string;
    };
    suggestedPaths: string[];
}): Promise<void> {
    return invoke<void>('save_workspace_template', { template });
}

export async function deleteWorkspaceTemplate(templateId: string): Promise<void> {
    return invoke<void>('delete_workspace_template', { templateId });
}

export async function getWorkspaceAnalytics(
    workspaceId: string
): Promise<{
        workspaceId: string;
        totalFiles: number;
        totalFolders: number;
        totalOperations: number;
        tasksCompleted: number;
        tasksFailed: number;
        memoryUsed: number;
        lastActivity: string;
    }> {
    const analytics = await invoke<{
        workspaceId: string;
        totalFiles: number;
        totalFolders: number;
        totalOperations: number;
        tasksCompleted: number;
        tasksFailed: number;
        memoryUsed: number;
        lastActivity: string;
    }>('get_workspace_analytics', { workspaceId });

    return {
        workspaceId: analytics.workspaceId,
        totalFiles: analytics.totalFiles,
        totalFolders: analytics.totalFolders,
        totalOperations: analytics.totalOperations,
        tasksCompleted: analytics.tasksCompleted,
        tasksFailed: analytics.tasksFailed,
        memoryUsed: analytics.memoryUsed,
        lastActivity: analytics.lastActivity,
    };
}

export async function addPermissionOverride(
    workspaceId: string,
    path: string,
    permissions: {
        canRead: boolean;
        canWrite: boolean;
        canExecute: boolean;
        canDelete: boolean;
        canCreateAgents: boolean;
    }
): Promise<void> {
    return invoke<void>('add_permission_override', { workspaceId, path, permissions });
}

export async function removePermissionOverride(
    workspaceId: string,
    path: string
): Promise<void> {
    return invoke<void>('remove_permission_override', { workspaceId, path });
}

export async function getPermissionOverrides(
    workspaceId: string
): Promise<
    Array<{
        path: string;
        permissions: {
            canRead: boolean;
            canWrite: boolean;
            canExecute: boolean;
            canDelete: boolean;
            canCreateAgents: boolean;
        };
        inherited: boolean;
    }>
> {
    return invoke<
        Array<{
            path: string;
            permissions: {
                canRead: boolean;
                canWrite: boolean;
                canExecute: boolean;
                canDelete: boolean;
                canCreateAgents: boolean;
            };
            inherited: boolean;
        }>
    >('get_permission_overrides', { workspaceId });
}

export async function getEffectivePermissions(
    workspaceId: string,
    path: string
): Promise<{
        canRead: boolean;
        canWrite: boolean;
        canExecute: boolean;
        canDelete: boolean;
        canCreateAgents: boolean;
    }> {
    return invoke<{
        canRead: boolean;
        canWrite: boolean;
        canExecute: boolean;
        canDelete: boolean;
        canCreateAgents: boolean;
    }>('get_effective_permissions', { workspaceId, path });
}

// ============ File Versioning Types ============

export interface FileVersion {
    id: string;
    filePath: string;
    versionNumber: number;
    timestamp: string;
    description: string;
    contentHash: string;
    size: number;
    versionPath: string;
}

export interface FileVersionInfo {
    filePath: string;
    currentVersion: number;
    totalVersions: number;
    versions: FileVersion[];
}

export type TransactionState = 'active' | 'committed' | 'rolled_back' | 'failed';

export interface Transaction {
    id: string;
    description: string;
    state: TransactionState;
    startTime: string;
    endTime?: string;
    operations: FileOpChange[];
    snapshots: FileVersion[];
}

// ============ File Versioning Commands ============

export async function createFileVersion(
    filePath: string,
    description: string
): Promise<FileVersion> {
    return invoke<FileVersion>('create_file_version', { filePath, description });
}

export async function getFileVersions(
    filePath: string
): Promise<FileVersionInfo> {
    return invoke<FileVersionInfo>('get_file_versions', { filePath });
}

export async function restoreFileVersion(
    filePath: string,
    versionId: string
): Promise<FileOpChange> {
    return invoke<FileOpChange>('restore_file_version', { filePath, versionId });
}

// ============ Transaction Commands ============

export async function beginFileTransaction(
    description: string
): Promise<string> {
    return invoke<string>('begin_file_transaction', { description });
}

export async function commitFileTransaction(
    transactionId: string
): Promise<FileOpChange[]> {
    return invoke<FileOpChange[]>('commit_file_transaction', { transactionId });
}

export async function rollbackFileTransaction(
    transactionId: string
): Promise<FileOpChange[]> {
    return invoke<FileOpChange[]>('rollback_file_transaction', { transactionId });
}

export async function getFileTransaction(
    transactionId: string
): Promise<Transaction | null> {
    return invoke<Transaction | null>('get_file_transaction', { transactionId });
}

// ============ Enhanced Undo/Redo Commands ============

export async function undoFileOperationEnhanced(
    operationId: string
): Promise<FileOpChange[]> {
    return invoke<FileOpChange[]>('undo_file_operation_enhanced', { operationId });
}

export async function redoFileOperation(): Promise<FileOpChange[]> {
    return invoke<FileOpChange[]>('redo_file_operation');
}

export async function listEnhancedFileOperations(): Promise<[string, string, string, string | null][]> {
    return invoke<[string, string, string, string | null][]>('list_enhanced_file_operations');
}

export async function setFileOpsWorkspace(
    workspaceId: string
): Promise<void> {
    return invoke<void>('set_file_ops_workspace', { workspaceId });
}

