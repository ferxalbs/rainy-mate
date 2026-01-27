// Rainy Cowork - File Versioning Hook
// React hook for file version history and management

import { useState, useCallback } from 'react';
import {
    createFileVersion,
    getFileVersions,
    restoreFileVersion,
    beginFileTransaction,
    commitFileTransaction,
    rollbackFileTransaction,
    getFileTransaction,
    undoFileOperationEnhanced,
    redoFileOperation,
    listEnhancedFileOperations,
    setFileOpsWorkspace,
    type FileVersion,
    type FileVersionInfo,
    type Transaction,
    type FileOpChange,
} from '../services/tauri';

export interface UseFileVersioningResult {
    // Versioning
    versions: FileVersionInfo | null;
    loading: boolean;
    error: string | null;
    createVersion: (filePath: string, description: string) => Promise<FileVersion>;
    loadVersions: (filePath: string) => Promise<void>;
    restoreVersion: (filePath: string, versionId: string) => Promise<FileOpChange>;
    // Transactions
    beginTransaction: (description: string) => Promise<string>;
    commitTransaction: (transactionId: string) => Promise<FileOpChange[]>;
    rollbackTransaction: (transactionId: string) => Promise<FileOpChange[]>;
    getTransaction: (transactionId: string) => Promise<Transaction | null>;
    // Undo/Redo
    undoOperation: (operationId: string) => Promise<FileOpChange[]>;
    redoOperation: () => Promise<FileOpChange[]>;
    listOperations: () => Promise<[string, string, string, string | null][]>;
    // Workspace context
    setWorkspace: (workspaceId: string) => Promise<void>;
}

export function useFileVersioning(): UseFileVersioningResult {
    const [versions, setVersions] = useState<FileVersionInfo | null>(null);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);

    const createVersion = useCallback(async (filePath: string, description: string) => {
        setLoading(true);
        setError(null);
        try {
            const version = await createFileVersion(filePath, description);
            // Reload versions after creating a new one
            await loadVersions(filePath);
            return version;
        } catch (err) {
            setError(err instanceof Error ? err.message : 'Failed to create version');
            throw err;
        } finally {
            setLoading(false);
        }
    }, []);

    const loadVersions = useCallback(async (filePath: string) => {
        setLoading(true);
        setError(null);
        try {
            const versionInfo = await getFileVersions(filePath);
            setVersions(versionInfo);
        } catch (err) {
            setError(err instanceof Error ? err.message : 'Failed to load versions');
            throw err;
        } finally {
            setLoading(false);
        }
    }, []);

    const restoreVersion = useCallback(async (filePath: string, versionId: string) => {
        setLoading(true);
        setError(null);
        try {
            const change = await restoreFileVersion(filePath, versionId);
            // Reload versions after restore
            await loadVersions(filePath);
            return change;
        } catch (err) {
            setError(err instanceof Error ? err.message : 'Failed to restore version');
            throw err;
        } finally {
            setLoading(false);
        }
    }, []);

    const beginTransaction = useCallback(async (description: string) => {
        setLoading(true);
        setError(null);
        try {
            const transactionId = await beginFileTransaction(description);
            return transactionId;
        } catch (err) {
            setError(err instanceof Error ? err.message : 'Failed to begin transaction');
            throw err;
        } finally {
            setLoading(false);
        }
    }, []);

    const commitTransaction = useCallback(async (transactionId: string) => {
        setLoading(true);
        setError(null);
        try {
            const changes = await commitFileTransaction(transactionId);
            return changes;
        } catch (err) {
            setError(err instanceof Error ? err.message : 'Failed to commit transaction');
            throw err;
        } finally {
            setLoading(false);
        }
    }, []);

    const rollbackTransaction = useCallback(async (transactionId: string) => {
        setLoading(true);
        setError(null);
        try {
            const changes = await rollbackFileTransaction(transactionId);
            return changes;
        } catch (err) {
            setError(err instanceof Error ? err.message : 'Failed to rollback transaction');
            throw err;
        } finally {
            setLoading(false);
        }
    }, []);

    const getTransaction = useCallback(async (transactionId: string) => {
        setLoading(true);
        setError(null);
        try {
            const transaction = await getFileTransaction(transactionId);
            return transaction;
        } catch (err) {
            setError(err instanceof Error ? err.message : 'Failed to get transaction');
            throw err;
        } finally {
            setLoading(false);
        }
    }, []);

    const undoOperation = useCallback(async (operationId: string) => {
        setLoading(true);
        setError(null);
        try {
            const changes = await undoFileOperationEnhanced(operationId);
            return changes;
        } catch (err) {
            setError(err instanceof Error ? err.message : 'Failed to undo operation');
            throw err;
        } finally {
            setLoading(false);
        }
    }, []);

    const redoOperation = useCallback(async () => {
        setLoading(true);
        setError(null);
        try {
            const changes = await redoFileOperation();
            return changes;
        } catch (err) {
            setError(err instanceof Error ? err.message : 'Failed to redo operation');
            throw err;
        } finally {
            setLoading(false);
        }
    }, []);

    const listOperations = useCallback(async () => {
        setLoading(true);
        setError(null);
        try {
            const operations = await listEnhancedFileOperations();
            return operations;
        } catch (err) {
            setError(err instanceof Error ? err.message : 'Failed to list operations');
            throw err;
        } finally {
            setLoading(false);
        }
    }, []);

    const setWorkspace = useCallback(async (workspaceId: string) => {
        setLoading(true);
        setError(null);
        try {
            await setFileOpsWorkspace(workspaceId);
        } catch (err) {
            setError(err instanceof Error ? err.message : 'Failed to set workspace');
            throw err;
        } finally {
            setLoading(false);
        }
    }, []);

    return {
        versions,
        loading,
        error,
        createVersion,
        loadVersions,
        restoreVersion,
        beginTransaction,
        commitTransaction,
        rollbackTransaction,
        getTransaction,
        undoOperation,
        redoOperation,
        listOperations,
        setWorkspace,
    };
}
