// Rainy MaTE - Workspace Creator
// Modal dialog for creating new workspaces with template selection

import { useState } from "react";
import {
    Modal,
    Button,
    Input,
    Label,
    Tabs,
    Card,
    TextField,
    Separator,
} from "@heroui/react";
import { FolderPlus, Code, FileText } from "lucide-react";
import { useWorkspace } from "../../hooks";
import type { WorkspaceTemplate, CreateWorkspaceOptions } from "../../types";
import { WORKSPACE_TEMPLATES } from "../../types/workspace";

interface WorkspaceCreatorProps {
    isOpen: boolean;
    onClose: () => void;
    onWorkspaceCreated?: (workspace: any) => void;
}

const TEMPLATE_ICONS = {
    development: Code,
    research: FileText,
    minimal: FolderPlus,
};

export function WorkspaceCreator({ isOpen, onClose, onWorkspaceCreated }: WorkspaceCreatorProps) {
    const { createWorkspace, isLoading } = useWorkspace();
    const [selectedTemplate, setSelectedTemplate] = useState<string>("development");
    const [workspaceName, setWorkspaceName] = useState("");
    const [allowedPaths, setAllowedPaths] = useState("");

    const handleCreate = async () => {
        if (!workspaceName.trim()) return;

        try {
            const paths = allowedPaths
                .split(",")
                .map(p => p.trim())
                .filter(p => p.length > 0);

            const options: CreateWorkspaceOptions = {
                name: workspaceName.trim(),
                templateId: selectedTemplate,
                allowedPaths: paths.length > 0 ? paths : ["."],
            };

            const workspace = await createWorkspace(options);
            onWorkspaceCreated?.(workspace);
            onClose();

            // Reset form
            setWorkspaceName("");
            setAllowedPaths("");
            setSelectedTemplate("development");
        } catch (error) {
            console.error("Failed to create workspace:", error);
        }
    };

    const handleTemplateSelect = (templateId: string) => {
        setSelectedTemplate(templateId);
    };

    return (
        <Modal.Backdrop isOpen={isOpen} onOpenChange={onClose}>
            <Modal.Container>
                <Modal.Dialog className="sm:max-w-2xl">
                    <Modal.CloseTrigger />
                    <Modal.Header>
                        <Modal.Icon className="bg-accent-soft text-accent-soft-foreground">
                            <FolderPlus className="size-5" />
                        </Modal.Icon>
                        <Modal.Heading>Create New Workspace</Modal.Heading>
                        <p className="text-sm text-muted-foreground">
                            Choose a template and configure your workspace settings
                        </p>
                    </Modal.Header>
                    <Modal.Body>
                        <Tabs
                            selectedKey={selectedTemplate}
                            onSelectionChange={handleTemplateSelect as any}
                            className="w-full"
                        >
                            <Tabs.ListContainer>
                                <Tabs.List aria-label="Workspace templates">
                                    {Object.entries(WORKSPACE_TEMPLATES).map(([id, template]: [string, WorkspaceTemplate]) => {
                                        const IconComponent = TEMPLATE_ICONS[id as keyof typeof TEMPLATE_ICONS] || FolderPlus;
                                        return (
                                            <Tabs.Tab key={id} id={id}>
                                                <div className="flex items-center gap-2">
                                                    <IconComponent className="size-4" />
                                                    <span>{template.name}</span>
                                                </div>
                                                <Tabs.Indicator />
                                            </Tabs.Tab>
                                        );
                                    })}
                                </Tabs.List>
                            </Tabs.ListContainer>

                            {Object.entries(WORKSPACE_TEMPLATES).map(([id, template]: [string, WorkspaceTemplate]) => (
                                <Tabs.Panel key={id} id={id} className="mt-4">
                                    <Card className="p-4">
                                        <div className="space-y-4">
                                            <div>
                                                <h3 className="font-semibold text-lg">{template.name}</h3>
                                                <p className="text-sm text-muted-foreground mt-1">
                                                    {template.description}
                                                </p>
                                            </div>

                                            <Separator />

                                            <div className="grid grid-cols-2 gap-4 text-sm">
                                                <div>
                                                    <span className="font-medium">Category:</span>
                                                    <span className="ml-2 text-muted-foreground">{template.category}</span>
                                                </div>
                                                <div>
                                                    <span className="font-medium">Memory Limit:</span>
                                                    <span className="ml-2 text-muted-foreground">
                                                        {(template.defaultMemory.maxSize / 1024 / 1024).toFixed(0)}MB
                                                    </span>
                                                </div>
                                            </div>

                                            {template.suggestedPaths && template.suggestedPaths.length > 0 && (
                                                <div>
                                                    <span className="font-medium text-sm">Suggested Paths:</span>
                                                    <div className="flex flex-wrap gap-1 mt-1">
                                                        {template.suggestedPaths.map((path) => (
                                                            <span
                                                                key={path}
                                                                className="px-2 py-1 bg-muted rounded text-xs"
                                                            >
                                                                {path}
                                                            </span>
                                                        ))}
                                                    </div>
                                                </div>
                                            )}
                                        </div>
                                    </Card>
                                </Tabs.Panel>
                            ))}
                        </Tabs>

                        <Separator className="my-6" />

                        <div className="space-y-4">
                            <TextField className="w-full">
                                <Label>Workspace Name</Label>
                                <Input
                                    placeholder="Enter workspace name"
                                    value={workspaceName}
                                    onChange={(e) => setWorkspaceName(e.target.value)}
                                />
                            </TextField>

                            <TextField className="w-full">
                                <Label>Allowed Paths (comma-separated)</Label>
                                <Input
                                    placeholder="e.g., src, tests, docs (leave empty for current directory)"
                                    value={allowedPaths}
                                    onChange={(e) => setAllowedPaths(e.target.value)}
                                />
                            </TextField>
                        </div>
                    </Modal.Body>
                    <Modal.Footer>
                        <Button slot="close" variant="secondary">
                            Cancel
                        </Button>
                        <Button
                            slot="close"
                            onPress={handleCreate}
                            isDisabled={!workspaceName.trim() || isLoading}
                        >
                            {isLoading ? "Creating..." : "Create Workspace"}
                        </Button>
                    </Modal.Footer>
                </Modal.Dialog>
            </Modal.Container>
        </Modal.Backdrop>
    );
}
