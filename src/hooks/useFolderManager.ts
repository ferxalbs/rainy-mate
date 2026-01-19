// Rainy Cowork - Folder Manager Hook
// Manages user-added folders with native folder picker dialog

import { useState, useCallback, useEffect } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import * as tauri from '../services/tauri';
import type { UserFolder } from '../services/tauri';

export interface UseFolderManagerReturn {
    folders: UserFolder[];
    isLoading: boolean;
    error: string | null;
    addFolder: () => Promise<UserFolder | null>;
    removeFolder: (id: string) => Promise<void>;
    refreshFolders: () => Promise<void>;
}

/**
 * Hook for managing user-added folders with persistence
 * Uses native OS folder picker dialog via Tauri plugin
 */
export function useFolderManager(): UseFolderManagerReturn {
    const [folders, setFolders] = useState<UserFolder[]>([]);
    const [isLoading, setIsLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);

    // Load folders on mount
    const refreshFolders = useCallback(async () => {
        setIsLoading(true);
        setError(null);
        try {
            const userFolders = await tauri.listUserFolders();
            setFolders(userFolders);
        } catch (e) {
            setError(e instanceof Error ? e.message : String(e));
        } finally {
            setIsLoading(false);
        }
    }, []);

    // Load folders on mount
    useEffect(() => {
        refreshFolders();
    }, [refreshFolders]);

    // Add folder via native picker
    const addFolder = useCallback(async (): Promise<UserFolder | null> => {
        setError(null);
        try {
            // Open native folder picker dialog
            const selected = await open({
                directory: true,
                multiple: false,
                title: 'Select Folder to Add',
            });

            // User cancelled the dialog
            if (!selected) {
                return null;
            }

            // Extract folder name from path
            const path = selected as string;
            const name = path.split('/').pop() || path.split('\\').pop() || 'Folder';

            // Add folder via Tauri backend
            const folder = await tauri.addUserFolder(path, name);

            // Update local state
            setFolders(prev => [...prev, folder]);

            return folder;
        } catch (e) {
            const errorMsg = e instanceof Error ? e.message : String(e);
            // Don't show error if folder already exists
            if (!errorMsg.includes('already added')) {
                setError(errorMsg);
            }
            return null;
        }
    }, []);

    // Remove folder by ID
    const removeFolder = useCallback(async (id: string) => {
        setError(null);
        try {
            await tauri.removeUserFolder(id);
            setFolders(prev => prev.filter(f => f.id !== id));
        } catch (e) {
            setError(e instanceof Error ? e.message : String(e));
        }
    }, []);

    return {
        folders,
        isLoading,
        error,
        addFolder,
        removeFolder,
        refreshFolders,
    };
}
