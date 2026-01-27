// Rainy Cowork - useWorkspace Hook
// React hook for advanced workspace management

import { useCallback, useEffect, useState } from 'react';
import * as tauri from '../services/tauri';
import type { Workspace, WorkspaceTemplate, CreateWorkspaceOptions } from '../types';

interface UseWorkspaceResult {
    workspaces: Workspace[];
    currentWorkspace: Workspace | null;
    isLoading: boolean;
    error: string | null;
    createWorkspace: (options: CreateWorkspaceOptions) => Promise<Workspace>;
    loadWorkspace: (id: string) => Promise<Workspace>;
    saveWorkspace: (workspace: Workspace) => Promise<void>;
    deleteWorkspace: (id: string) => Promise<void>;
    refreshWorkspaces: () => Promise<void>;
    selectWorkspace: (workspace: Workspace | null) => void;
    addPermissionOverride: (workspaceId: string, path: string, permissions: {
        canRead: boolean;
        canWrite: boolean;
        canExecute: boolean;
        canDelete: boolean;
        canCreateAgents: boolean;
    }) => Promise<void>;
    removePermissionOverride: (workspaceId: string, path: string) => Promise<void>;
    getPermissionOverrides: (workspaceId: string) => Promise<Array<{
        path: string;
        permissions: {
            canRead: boolean;
            canWrite: boolean;
            canExecute: boolean;
            canDelete: boolean;
            canCreateAgents: boolean;
        };
        inherited: boolean;
    }>>;
    getEffectivePermissions: (workspaceId: string, path: string) => Promise<{
        canRead: boolean;
        canWrite: boolean;
        canExecute: boolean;
        canDelete: boolean;
        canCreateAgents: boolean;
    }>;
    getWorkspaceTemplates: () => Promise<Array<{
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
    }>>;
    createWorkspaceFromTemplate: (templateId: string, name: string, customPaths?: string[]) => Promise<Workspace>;
    saveWorkspaceTemplate: (template: {
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
    }) => Promise<void>;
    deleteWorkspaceTemplate: (templateId: string) => Promise<void>;
}

export function useWorkspace(): UseWorkspaceResult {
    const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
    const [currentWorkspace, setCurrentWorkspace] = useState<Workspace | null>(null);
    const [isLoading, setIsLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);

    const refreshWorkspaces = useCallback(async () => {
        setIsLoading(true);
        setError(null);
        try {
            const workspaceIds = await tauri.listWorkspaces();
            const loadedWorkspaces: Workspace[] = [];

            for (const id of workspaceIds) {
                try {
                    const workspace = await tauri.loadWorkspace(id);
                    loadedWorkspaces.push(workspace);
                } catch (err) {
                    console.warn(`Failed to load workspace ${id}:`, err);
                }
            }

            setWorkspaces(loadedWorkspaces);
        } catch (err) {
            setError(err instanceof Error ? err.message : 'Failed to load workspaces');
        } finally {
            setIsLoading(false);
        }
    }, []);

    const createWorkspace = useCallback(async (options: CreateWorkspaceOptions): Promise<Workspace> => {
        setIsLoading(true);
        setError(null);
        try {
            let template: WorkspaceTemplate | undefined;

            if (options.templateId) {
                // Import templates dynamically to avoid circular dependency
                const { WORKSPACE_TEMPLATES } = await import('../types/workspace');
                template = WORKSPACE_TEMPLATES.find(t => t.id === options.templateId);
            }

            const permissions = options.customPermissions || template?.defaultPermissions || {
                canRead: true,
                canWrite: true,
                canExecute: false,
                canDelete: false,
                canCreateAgents: true,
            };

            const settings = options.customSettings || template?.defaultSettings || {
                theme: 'system',
                language: 'en',
                autoSave: true,
                notificationsEnabled: true,
            };

            const workspace = await tauri.createWorkspace(
                options.name,
                options.allowedPaths
            );

            // Apply template settings if available
            if (template) {
                workspace.permissions = { ...workspace.permissions, ...permissions };
                workspace.settings = { ...workspace.settings, ...settings };
                workspace.memory = { ...workspace.memory, ...template.defaultMemory };

                // Save the updated workspace
                await tauri.saveWorkspace(workspace);
            }

            await refreshWorkspaces();
            return workspace;
        } catch (err) {
            const errorMessage = err instanceof Error ? err.message : 'Failed to create workspace';
            setError(errorMessage);
            throw new Error(errorMessage);
        } finally {
            setIsLoading(false);
        }
    }, [refreshWorkspaces]);

    const loadWorkspace = useCallback(async (id: string): Promise<Workspace> => {
        setIsLoading(true);
        setError(null);
        try {
            const workspace = await tauri.loadWorkspace(id);
            setCurrentWorkspace(workspace);
            return workspace;
        } catch (err) {
            const errorMessage = err instanceof Error ? err.message : 'Failed to load workspace';
            setError(errorMessage);
            throw new Error(errorMessage);
        } finally {
            setIsLoading(false);
        }
    }, []);

    const saveWorkspace = useCallback(async (workspace: Workspace): Promise<void> => {
        setIsLoading(true);
        setError(null);
        try {
            await tauri.saveWorkspace(workspace);
            await refreshWorkspaces();
        } catch (err) {
            const errorMessage = err instanceof Error ? err.message : 'Failed to save workspace';
            setError(errorMessage);
            throw new Error(errorMessage);
        } finally {
            setIsLoading(false);
        }
    }, [refreshWorkspaces]);

    const deleteWorkspace = useCallback(async (id: string): Promise<void> => {
        setIsLoading(true);
        setError(null);
        try {
            await tauri.deleteWorkspace(id);

            // Remove from local state
            setWorkspaces(prev => prev.filter(w => w.id !== id));
            if (currentWorkspace?.id === id) {
                setCurrentWorkspace(null);
            }
        } catch (err) {
            const errorMessage = err instanceof Error ? err.message : 'Failed to delete workspace';
            setError(errorMessage);
            throw new Error(errorMessage);
        } finally {
            setIsLoading(false);
        }
    }, [currentWorkspace]);

    const selectWorkspace = useCallback((workspace: Workspace | null) => {
        setCurrentWorkspace(workspace);
    }, []);

    const addPermissionOverride = useCallback(async (
        workspaceId: string,
        path: string,
        permissions: {
            canRead: boolean;
            canWrite: boolean;
            canExecute: boolean;
            canDelete: boolean;
            canCreateAgents: boolean;
        }
    ): Promise<void> => {
        setIsLoading(true);
        setError(null);
        try {
            await tauri.addPermissionOverride(workspaceId, path, permissions);
            await refreshWorkspaces();
        } catch (err) {
            const errorMessage = err instanceof Error ? err.message : 'Failed to add permission override';
            setError(errorMessage);
            throw new Error(errorMessage);
        } finally {
            setIsLoading(false);
        }
    }, [refreshWorkspaces]);

    const removePermissionOverride = useCallback(async (workspaceId: string, path: string): Promise<void> => {
        setIsLoading(true);
        setError(null);
        try {
            await tauri.removePermissionOverride(workspaceId, path);
            await refreshWorkspaces();
        } catch (err) {
            const errorMessage = err instanceof Error ? err.message : 'Failed to remove permission override';
            setError(errorMessage);
            throw new Error(errorMessage);
        } finally {
            setIsLoading(false);
        }
    }, [refreshWorkspaces]);

    const getPermissionOverrides = useCallback(async (workspaceId: string): Promise<Array<{
        path: string;
        permissions: {
            canRead: boolean;
            canWrite: boolean;
            canExecute: boolean;
            canDelete: boolean;
            canCreateAgents: boolean;
        };
        inherited: boolean;
    }>> => {
        setIsLoading(true);
        setError(null);
        try {
            const overrides = await tauri.getPermissionOverrides(workspaceId);
            return overrides;
        } catch (err) {
            const errorMessage = err instanceof Error ? err.message : 'Failed to get permission overrides';
            setError(errorMessage);
            throw new Error(errorMessage);
        } finally {
            setIsLoading(false);
        }
    }, []);

    const getEffectivePermissions = useCallback(async (workspaceId: string, path: string): Promise<{
        canRead: boolean;
        canWrite: boolean;
        canExecute: boolean;
        canDelete: boolean;
        canCreateAgents: boolean;
    }> => {
        setIsLoading(true);
        setError(null);
        try {
            const permissions = await tauri.getEffectivePermissions(workspaceId, path);
            return permissions;
        } catch (err) {
            const errorMessage = err instanceof Error ? err.message : 'Failed to get effective permissions';
            setError(errorMessage);
            throw new Error(errorMessage);
        } finally {
            setIsLoading(false);
        }
    }, []);

    const getWorkspaceTemplates = useCallback(async (): Promise<Array<{
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
    }>> => {
        setIsLoading(true);
        setError(null);
        try {
            const templates = await tauri.getWorkspaceTemplates();
            return templates;
        } catch (err) {
            const errorMessage = err instanceof Error ? err.message : 'Failed to get workspace templates';
            setError(errorMessage);
            throw new Error(errorMessage);
        } finally {
            setIsLoading(false);
        }
    }, []);

    const createWorkspaceFromTemplate = useCallback(async (
        templateId: string,
        name: string,
        customPaths?: string[]
    ): Promise<Workspace> => {
        setIsLoading(true);
        setError(null);
        try {
            const workspace = await tauri.createWorkspaceFromTemplate(templateId, name, customPaths);
            await refreshWorkspaces();
            return workspace;
        } catch (err) {
            const errorMessage = err instanceof Error ? err.message : 'Failed to create workspace from template';
            setError(errorMessage);
            throw new Error(errorMessage);
        } finally {
            setIsLoading(false);
        }
    }, [refreshWorkspaces]);

    const saveWorkspaceTemplate = useCallback(async (template: {
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
    }): Promise<void> => {
        setIsLoading(true);
        setError(null);
        try {
            await tauri.saveWorkspaceTemplate(template);
        } catch (err) {
            const errorMessage = err instanceof Error ? err.message : 'Failed to save workspace template';
            setError(errorMessage);
            throw new Error(errorMessage);
        } finally {
            setIsLoading(false);
        }
    }, []);

    const deleteWorkspaceTemplate = useCallback(async (templateId: string): Promise<void> => {
        setIsLoading(true);
        setError(null);
        try {
            await tauri.deleteWorkspaceTemplate(templateId);
        } catch (err) {
            const errorMessage = err instanceof Error ? err.message : 'Failed to delete workspace template';
            setError(errorMessage);
            throw new Error(errorMessage);
        } finally {
            setIsLoading(false);
        }
    }, []);

    // Load workspaces on mount
    useEffect(() => {
        refreshWorkspaces();
    }, [refreshWorkspaces]);

    return {
        workspaces,
        currentWorkspace,
        isLoading,
        error,
        createWorkspace,
        loadWorkspace,
        saveWorkspace,
        deleteWorkspace,
        refreshWorkspaces,
        selectWorkspace,
        addPermissionOverride,
        removePermissionOverride,
        getPermissionOverrides,
        getEffectivePermissions,
        getWorkspaceTemplates,
        createWorkspaceFromTemplate,
        saveWorkspaceTemplate,
        deleteWorkspaceTemplate,
    };
}