// Rainy Cowork - File Version History Component
// UI component for viewing and managing file version history

import { useState, useEffect } from 'react';
import { Card, Button, Input, Label, Alert, Separator } from '@heroui/react';
import { History, RotateCcw, Save, Clock, FileText, AlertCircle } from 'lucide-react';
import { useFileVersioning } from '../../hooks/useFileVersioning';

interface FileVersionHistoryProps {
    filePath: string;
    onClose?: () => void;
}

function formatFileSize(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${(bytes / Math.pow(k, i)).toFixed(2)} ${sizes[i]}`;
}

function formatTimeAgo(timestamp: string): string {
    const now = new Date();
    const then = new Date(timestamp);
    const diffMs = now.getTime() - then.getTime();
    const diffMins = Math.floor(diffMs / 60000);
    const diffHours = Math.floor(diffMs / 3600000);
    const diffDays = Math.floor(diffMs / 86400000);

    if (diffMins < 1) return 'just now';
    if (diffMins < 60) return `${diffMins}m ago`;
    if (diffHours < 24) return `${diffHours}h ago`;
    return `${diffDays}d ago`;
}

export function FileVersionHistory({ filePath, onClose }: FileVersionHistoryProps) {
    const {
        versions,
        loading,
        error,
        createVersion,
        loadVersions,
        restoreVersion,
    } = useFileVersioning();

    const [newVersionDescription, setNewVersionDescription] = useState('');
    const [creatingVersion, setCreatingVersion] = useState(false);
    const [restoringVersion, setRestoringVersion] = useState<string | null>(null);
    const [successMessage, setSuccessMessage] = useState<string | null>(null);

    useEffect(() => {
        loadVersions(filePath);
    }, [filePath, loadVersions]);

    const handleCreateVersion = async () => {
        if (!newVersionDescription.trim()) return;

        setCreatingVersion(true);
        setSuccessMessage(null);
        try {
            await createVersion(filePath, newVersionDescription);
            setNewVersionDescription('');
            setSuccessMessage('Version created successfully!');
            setTimeout(() => setSuccessMessage(null), 3000);
        } catch (err) {
            console.error('Failed to create version:', err);
        } finally {
            setCreatingVersion(false);
        }
    };

    const handleRestoreVersion = async (versionId: string) => {
        setRestoringVersion(versionId);
        setSuccessMessage(null);
        try {
            await restoreVersion(filePath, versionId);
            setSuccessMessage('Version restored successfully!');
            setTimeout(() => setSuccessMessage(null), 3000);
        } catch (err) {
            console.error('Failed to restore version:', err);
        } finally {
            setRestoringVersion(null);
        }
    };

    const fileName = filePath.split('/').pop() || filePath;

    return (
        <div className="space-y-4">
            <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                    <FileText className="size-5 text-primary" />
                    <div>
                        <h3 className="text-lg font-semibold">Version History</h3>
                        <p className="text-sm text-muted-foreground">{fileName}</p>
                    </div>
                </div>
                {onClose && (
                    <Button variant="ghost" size="sm" onPress={onClose}>
                        Close
                    </Button>
                )}
            </div>

            {successMessage && (
                <Alert status="success" className="mb-4">
                    <Alert.Indicator />
                    <Alert.Content>
                        <Alert.Title>{successMessage}</Alert.Title>
                    </Alert.Content>
                </Alert>
            )}

            {error && (
                <Alert status="danger" className="mb-4">
                    <Alert.Indicator>
                        <AlertCircle className="size-4" />
                    </Alert.Indicator>
                    <Alert.Content>
                        <Alert.Title>{error}</Alert.Title>
                    </Alert.Content>
                </Alert>
            )}

            {/* Create New Version */}
            <Card variant="secondary" className="p-4">
                <div className="space-y-3">
                    <Label className="text-sm font-medium">Create New Version</Label>
                    <div className="flex gap-2">
                        <Input
                            placeholder="Describe this version (e.g., 'Before major changes')"
                            value={newVersionDescription}
                            onChange={(e) => setNewVersionDescription(e.target.value)}
                            onKeyDown={(e) => e.key === 'Enter' && handleCreateVersion()}
                            disabled={creatingVersion || loading}
                        />
                        <Button
                            onPress={handleCreateVersion}
                            isDisabled={creatingVersion || loading || !newVersionDescription.trim()}
                            isPending={creatingVersion}
                        >
                            <Save className="size-4 mr-2" />
                            Save
                        </Button>
                    </div>
                </div>
            </Card>

            <Separator />

            {/* Version List */}
            {loading && !versions ? (
                <Card variant="secondary" className="p-8 text-center">
                    <p className="text-sm text-muted-foreground">Loading versions...</p>
                </Card>
            ) : versions && versions.versions.length > 0 ? (
                <div className="space-y-2">
                    <div className="flex items-center gap-2 mb-3">
                        <History className="size-4 text-muted-foreground" />
                        <Label className="text-sm font-medium">
                            {versions.totalVersions} version{versions.totalVersions !== 1 ? 's' : ''} available
                        </Label>
                    </div>

                    {versions.versions.slice().reverse().map((version) => (
                        <Card
                            key={version.id}
                            variant={version.versionNumber === versions.currentVersion ? 'default' : 'secondary'}
                            className="p-4"
                        >
                            <div className="flex items-start justify-between gap-4">
                                <div className="flex-1 space-y-2">
                                    <div className="flex items-center gap-2">
                                        <span className="text-sm font-semibold">
                                            Version {version.versionNumber}
                                        </span>
                                        {version.versionNumber === versions.currentVersion && (
                                            <span className="text-xs bg-primary/10 text-primary px-2 py-0.5 rounded-full">
                                                Current
                                            </span>
                                        )}
                                    </div>

                                    {version.description && (
                                        <p className="text-sm text-muted-foreground">
                                            {version.description}
                                        </p>
                                    )}

                                    <div className="flex items-center gap-4 text-xs text-muted-foreground">
                                        <div className="flex items-center gap-1">
                                            <Clock className="size-3" />
                                            <span>{formatTimeAgo(version.timestamp)}</span>
                                        </div>
                                        <span>{formatFileSize(version.size)}</span>
                                        <span className="font-mono">
                                            {version.contentHash.slice(0, 8)}...
                                        </span>
                                    </div>
                                </div>

                                <Button
                                    variant="ghost"
                                    size="sm"
                                    onPress={() => handleRestoreVersion(version.id)}
                                    isDisabled={
                                        restoringVersion === version.id ||
                                        version.versionNumber === versions.currentVersion ||
                                        loading
                                    }
                                    isPending={restoringVersion === version.id}
                                >
                                    <RotateCcw className="size-4 mr-2" />
                                    Restore
                                </Button>
                            </div>
                        </Card>
                    ))}
                </div>
            ) : (
                <Card variant="secondary" className="p-8 text-center">
                    <History className="size-12 text-muted-foreground mx-auto mb-4" />
                    <p className="text-sm text-muted-foreground mb-2">
                        No versions found for this file
                    </p>
                    <p className="text-xs text-muted-foreground">
                        Create a version to start tracking changes
                    </p>
                </Card>
            )}
        </div>
    );
}
