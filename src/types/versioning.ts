// Rainy Cowork - File Versioning Types
// Types for file version history and management

/**
 * File version snapshot
 */
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

/**
 * Version metadata for a file
 */
export interface FileVersionInfo {
    filePath: string;
    currentVersion: number;
    totalVersions: number;
    versions: FileVersion[];
}

/**
 * Transaction state
 */
export type TransactionState = 'active' | 'committed' | 'rolled_back' | 'failed';

/**
 * Transaction context for batch operations
 */
export interface Transaction {
    id: string;
    description: string;
    state: TransactionState;
    startTime: string;
    endTime?: string;
    operations: FileOpChange[];
    snapshots: FileVersion[];
}

/**
 * File operation change record
 */
export interface FileOpChange {
    id: string;
    operation: FileOpType;
    sourcePath: string;
    destPath?: string;
    timestamp: string;
    reversible: boolean;
}

/**
 * Type of file operation
 */
export type FileOpType = 'move' | 'copy' | 'rename' | 'delete' | 'create' | 'create_folder';

/**
 * Enhanced operation record with versioning support
 */
export interface EnhancedOperationRecord {
    id: string;
    description: string;
    timestamp: string;
    changes: FileOpChange[];
    transactionId?: string;
    versionsCreated: FileVersion[];
}

/**
 * Version comparison result
 */
export interface VersionComparison {
    version1: FileVersion;
    version2: FileVersion;
    sizeDifference: number;
    hashDifferent: boolean;
    timeDifference: number; // in milliseconds
}

/**
 * Version restore options
 */
export interface RestoreOptions {
    createBackup: boolean;
    backupDescription?: string;
}
