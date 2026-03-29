// Rainy MaTE - Workspace List
// Component for overview and management of workspaces

import { useState } from "react";
import {
    Button,
    Card,
    Chip,
} from "@heroui/react";
import {
    Folder,
    Settings,
    Trash2,
    Plus,
    Users,
    Database,
    Shield,
    Eye,
} from "lucide-react";
import { useWorkspace } from "../../hooks";
import type { Workspace } from "../../types";

interface WorkspaceListProps {
    onCreateNew?: () => void;
    onEditSettings?: (workspace: Workspace) => void;
    className?: string;
}

export function WorkspaceList({ onCreateNew, onEditSettings, className }: WorkspaceListProps) {
    const { workspaces, currentWorkspace, selectWorkspace, deleteWorkspace, isLoading } = useWorkspace();
    const [deletingId, setDeletingId] = useState<string | null>(null);

    const handleDelete = async (workspaceId: string) => {
        if (confirm("Are you sure you want to delete this workspace? This action cannot be undone.")) {
            setDeletingId(workspaceId);
            try {
                await deleteWorkspace(workspaceId);
            } catch (error) {
                console.error("Failed to delete workspace:", error);
            } finally {
                setDeletingId(null);
            }
        }
    };

    const formatMemorySize = (bytes: number) => {
        const mb = bytes / 1024 / 1024;
        return `${mb.toFixed(1)}MB`;
    };

    const getPermissionCount = (permissions: Workspace["permissions"]) => {
        return Object.values(permissions).filter(Boolean).length;
    };

    return (
        <div className={className}>
            <div className="flex items-center justify-between mb-6">
                <div>
                    <h2 className="text-xl font-semibold">Workspaces</h2>
                    <p className="text-sm text-muted-foreground">
                        Manage your workspace configurations and settings
                    </p>
                </div>
                <Button onPress={onCreateNew} variant="secondary">
                    <Plus className="size-4 mr-2" />
                    New Workspace
                </Button>
            </div>

            {isLoading && workspaces.length === 0 ? (
                <div className="text-center py-8">
                    <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary mx-auto mb-4"></div>
                    <p className="text-muted-foreground">Loading workspaces...</p>
                </div>
            ) : workspaces.length === 0 ? (
                <Card className="p-8 text-center">
                    <Folder className="size-12 mx-auto mb-4 text-muted-foreground" />
                    <h3 className="text-lg font-medium mb-2">No workspaces found</h3>
                    <p className="text-muted-foreground mb-4">
                        Create your first workspace to get started with advanced project management.
                    </p>
                    <Button onPress={onCreateNew}>
                        <Plus className="size-4 mr-2" />
                        Create Workspace
                    </Button>
                </Card>
            ) : (
                <div className="grid gap-4">
                    {workspaces.map((workspace) => (
                        <Card key={workspace.id} className="p-6">
                            <div className="flex items-start justify-between">
                                <div className="flex-1">
                                    <div className="flex items-center gap-3 mb-3">
                                        <Folder className="size-5 text-primary" />
                                        <h3 className="text-lg font-medium">{workspace.name}</h3>
                                        {currentWorkspace?.id === workspace.id && (
                                            <Chip variant="secondary" size="sm">
                                                Active
                                            </Chip>
                                        )}
                                    </div>

                                    <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-4">
                                        <div className="flex items-center gap-2 text-sm text-muted-foreground">
                                            <Database className="size-4" />
                                            <span>{formatMemorySize(workspace.memory.maxSize)}</span>
                                        </div>
                                        <div className="flex items-center gap-2 text-sm text-muted-foreground">
                                            <Users className="size-4" />
                                            <span>{workspace.agents.length} agents</span>
                                        </div>
                                        <div className="flex items-center gap-2 text-sm text-muted-foreground">
                                            <Shield className="size-4" />
                                            <span>{getPermissionCount(workspace.permissions)} permissions</span>
                                        </div>
                                        <div className="flex items-center gap-2 text-sm text-muted-foreground">
                                            <Eye className="size-4" />
                                            <span>{workspace.allowedPaths.length} paths</span>
                                        </div>
                                    </div>

                                    <div className="flex flex-wrap gap-1 mb-4">
                                        {workspace.allowedPaths.slice(0, 3).map((path) => (
                                            <Chip key={path} variant="secondary" size="sm">
                                                {path}
                                            </Chip>
                                        ))}
                                        {workspace.allowedPaths.length > 3 && (
                                            <Chip variant="secondary" size="sm">
                                                +{workspace.allowedPaths.length - 3} more
                                            </Chip>
                                        )}
                                    </div>

                                    <div className="flex items-center gap-4 text-sm">
                                        <span className="text-muted-foreground">
                                            Theme: <span className="font-medium">{workspace.settings.theme}</span>
                                        </span>
                                        <span className="text-muted-foreground">
                                            Language: <span className="font-medium">{workspace.settings.language}</span>
                                        </span>
                                        {workspace.settings.autoSave && (
                                            <Chip variant="secondary" size="sm">
                                                Auto-save
                                            </Chip>
                                        )}
                                    </div>
                                </div>

                                <div className="flex items-center gap-2 ml-4">
                                    <Button
                                        variant="ghost"
                                        size="sm"
                                        onPress={() => selectWorkspace(workspace)}
                                        isDisabled={currentWorkspace?.id === workspace.id}
                                    >
                                        {currentWorkspace?.id === workspace.id ? "Active" : "Select"}
                                    </Button>
                                    <Button
                                        variant="ghost"
                                        size="sm"
                                        onPress={() => onEditSettings?.(workspace)}
                                    >
                                        <Settings className="size-4" />
                                    </Button>
                                    <Button
                                        variant="ghost"
                                        size="sm"
                                        onPress={() => handleDelete(workspace.id)}
                                        isDisabled={deletingId === workspace.id}
                                        className="text-red-600 hover:text-red-700"
                                    >
                                        <Trash2 className="size-4" />
                                    </Button>
                                </div>
                            </div>
                        </Card>
                    ))}
                </div>
            )}
        </div>
    );
}