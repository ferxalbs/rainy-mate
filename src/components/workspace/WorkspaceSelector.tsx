// Rainy MaTE - Workspace Selector
// Component for switching between workspaces

import { Label, ListBox, Select } from "@heroui/react";
import { Folder, Plus } from "lucide-react";
import { useWorkspace } from "../../hooks";

interface WorkspaceSelectorProps {
    onCreateNew?: () => void;
    className?: string;
}

export function WorkspaceSelector({ onCreateNew, className }: WorkspaceSelectorProps) {
    const { workspaces, currentWorkspace, selectWorkspace, isLoading } = useWorkspace();

    const handleSelectionChange = (key: string | null) => {
        if (key === "create-new") {
            onCreateNew?.();
            return;
        }

        const workspace = workspaces.find(w => w.id === key);
        selectWorkspace(workspace || null);
    };

    const selectedKey = currentWorkspace?.id || null;

    return (
        <Select
            className={className}
            placeholder="Select workspace"
            value={selectedKey}
            onChange={handleSelectionChange as any}
            isDisabled={isLoading}
        >
            <Label>Workspace</Label>
            <Select.Trigger>
                <Select.Value>
                    {({ defaultChildren, isPlaceholder }) => {
                        if (isPlaceholder) {
                            return (
                                <div className="flex items-center gap-2">
                                    <Folder className="size-4" />
                                    <span>Select workspace</span>
                                </div>
                            );
                        }

                        if (currentWorkspace) {
                            return (
                                <div className="flex items-center gap-2">
                                    <Folder className="size-4" />
                                    <span>{currentWorkspace.name}</span>
                                </div>
                            );
                        }

                        return defaultChildren;
                    }}
                </Select.Value>
                <Select.Indicator />
            </Select.Trigger>
            <Select.Popover>
                <ListBox>
                    {workspaces.map((workspace) => (
                        <ListBox.Item
                            key={workspace.id}
                            id={workspace.id}
                            textValue={workspace.name}
                        >
                            <div className="flex items-center gap-2">
                                <Folder className="size-4" />
                                <div className="flex flex-col">
                                    <span className="font-medium">{workspace.name}</span>
                                    <span className="text-xs text-muted-foreground">
                                        {workspace.allowedPaths.length} path{workspace.allowedPaths.length !== 1 ? 's' : ''}
                                    </span>
                                </div>
                            </div>
                            <ListBox.ItemIndicator />
                        </ListBox.Item>
                    ))}
                    {onCreateNew && (
                        <ListBox.Item
                            key="create-new"
                            id="create-new"
                            textValue="Create new workspace"
                        >
                            <div className="flex items-center gap-2">
                                <Plus className="size-4" />
                                <span>Create new workspace</span>
                            </div>
                            <ListBox.ItemIndicator />
                        </ListBox.Item>
                    )}
                </ListBox>
            </Select.Popover>
        </Select>
    );
}