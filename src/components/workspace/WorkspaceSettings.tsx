// Rainy MaTE - Workspace Settings
// Panel for configuring workspace settings

import { useState } from "react";
import {
    Button,
    Input,
    Label,
    Switch,
    Card,
    Separator,
    TextField,
    Select,
    ListBox,
} from "@heroui/react";
import { Settings, X } from "lucide-react";
import { useWorkspace } from "../../hooks";
import type { Workspace } from "../../types";

interface WorkspaceSettingsProps {
    workspace: Workspace;
    isOpen: boolean;
    onClose: () => void;
    onWorkspaceUpdated?: (workspace: Workspace) => void;
}

const THEME_OPTIONS = [
    { id: "light", name: "Light" },
    { id: "dark", name: "Dark" },
    { id: "system", name: "System" },
];

const LANGUAGE_OPTIONS = [
    { id: "en", name: "English" },
    { id: "es", name: "Spanish" },
    { id: "fr", name: "French" },
    { id: "de", name: "German" },
    { id: "zh", name: "Chinese" },
];

export function WorkspaceSettings({ workspace, isOpen, onClose, onWorkspaceUpdated }: WorkspaceSettingsProps) {
    const { saveWorkspace, isLoading } = useWorkspace();
    const [settings, setSettings] = useState(workspace.settings);
    const [permissions, setPermissions] = useState(workspace.permissions);
    const [name, setName] = useState(workspace.name);
    const [allowedPaths, setAllowedPaths] = useState(workspace.allowedPaths.join(", "));

    const handleSave = async () => {
        try {
            const updatedWorkspace: Workspace = {
                ...workspace,
                name: name.trim(),
                allowedPaths: allowedPaths.split(",").map(p => p.trim()).filter(p => p.length > 0),
                settings,
                permissions,
            };

            await saveWorkspace(updatedWorkspace);
            onWorkspaceUpdated?.(updatedWorkspace);
            onClose();
        } catch (error) {
            console.error("Failed to save workspace settings:", error);
        }
    };

    const handlePermissionChange = (key: keyof typeof permissions, value: boolean) => {
        setPermissions(prev => ({ ...prev, [key]: value }));
    };

    if (!isOpen) return null;

    return (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
            <Card className="w-full max-w-2xl max-h-[90vh] overflow-hidden">
                <div className="flex items-center justify-between p-6 border-b">
                    <div className="flex items-center gap-3">
                        <Settings className="size-5" />
                        <h2 className="text-lg font-semibold">Workspace Settings</h2>
                    </div>
                    <Button variant="ghost" size="sm" onPress={onClose}>
                        <X className="size-4" />
                    </Button>
                </div>

                <div className="p-6 overflow-y-auto max-h-[calc(90vh-120px)]">
                    <div className="space-y-6">
                        {/* Basic Information */}
                        <div className="space-y-4">
                            <h3 className="text-md font-medium">Basic Information</h3>

                            <TextField className="w-full">
                                <Label>Workspace Name</Label>
                                <Input
                                    value={name}
                                    onChange={(e) => setName(e.target.value)}
                                    placeholder="Enter workspace name"
                                />
                            </TextField>

                            <TextField className="w-full">
                                <Label>Allowed Paths</Label>
                                <Input
                                    value={allowedPaths}
                                    onChange={(e) => setAllowedPaths(e.target.value)}
                                    placeholder="Comma-separated paths (e.g., src, tests, docs)"
                                />
                            </TextField>
                        </div>

                        <Separator />

                        {/* Appearance Settings */}
                        <div className="space-y-4">
                            <h3 className="text-md font-medium">Appearance</h3>

                            <Select
                                className="w-full"
                                value={settings.theme}
                                onChange={(value) => setSettings(prev => ({ ...prev, theme: (value as string) || "system" }))}
                            >
                                <Label>Theme</Label>
                                <Select.Trigger>
                                    <Select.Value />
                                    <Select.Indicator />
                                </Select.Trigger>
                                <Select.Popover>
                                    <ListBox>
                                        {THEME_OPTIONS.map((theme) => (
                                            <ListBox.Item key={theme.id} id={theme.id} textValue={theme.name}>
                                                {theme.name}
                                                <ListBox.ItemIndicator />
                                            </ListBox.Item>
                                        ))}
                                    </ListBox>
                                </Select.Popover>
                            </Select>

                            <Select
                                className="w-full"
                                value={settings.language}
                                onChange={(value) => setSettings(prev => ({ ...prev, language: (value as string) || "en" }))}
                            >
                                <Label>Language</Label>
                                <Select.Trigger>
                                    <Select.Value />
                                    <Select.Indicator />
                                </Select.Trigger>
                                <Select.Popover>
                                    <ListBox>
                                        {LANGUAGE_OPTIONS.map((lang) => (
                                            <ListBox.Item key={lang.id} id={lang.id} textValue={lang.name}>
                                                {lang.name}
                                                <ListBox.ItemIndicator />
                                            </ListBox.Item>
                                        ))}
                                    </ListBox>
                                </Select.Popover>
                            </Select>
                        </div>

                        <Separator />

                        {/* Behavior Settings */}
                        <div className="space-y-4">
                            <h3 className="text-md font-medium">Behavior</h3>

                            <Switch
                                isSelected={settings.autoSave}
                                onChange={(value) => setSettings(prev => ({ ...prev, autoSave: value }))}
                            >
                                <Switch.Control>
                                    <Switch.Thumb />
                                </Switch.Control>
                                <Label>Auto-save changes</Label>
                            </Switch>

                            <Switch
                                isSelected={settings.notificationsEnabled}
                                onChange={(value) => setSettings(prev => ({ ...prev, notificationsEnabled: value }))}
                            >
                                <Switch.Control>
                                    <Switch.Thumb />
                                </Switch.Control>
                                <Label>Enable notifications</Label>
                            </Switch>
                        </div>

                        <Separator />

                        {/* Permissions */}
                        <div className="space-y-4">
                            <h3 className="text-md font-medium">Permissions</h3>

                            <div className="space-y-3">
                                <Switch
                                    isSelected={permissions.canRead}
                                    onChange={(value) => handlePermissionChange("canRead", value)}
                                >
                                    <Switch.Control>
                                        <Switch.Thumb />
                                    </Switch.Control>
                                    <Label>Allow reading files</Label>
                                </Switch>

                                <Switch
                                    isSelected={permissions.canWrite}
                                    onChange={(value) => handlePermissionChange("canWrite", value)}
                                >
                                    <Switch.Control>
                                        <Switch.Thumb />
                                    </Switch.Control>
                                    <Label>Allow writing files</Label>
                                </Switch>

                                <Switch
                                    isSelected={permissions.canExecute}
                                    onChange={(value) => handlePermissionChange("canExecute", value)}
                                >
                                    <Switch.Control>
                                        <Switch.Thumb />
                                    </Switch.Control>
                                    <Label>Allow executing commands</Label>
                                </Switch>

                                <Switch
                                    isSelected={permissions.canDelete}
                                    onChange={(value) => handlePermissionChange("canDelete", value)}
                                >
                                    <Switch.Control>
                                        <Switch.Thumb />
                                    </Switch.Control>
                                    <Label>Allow deleting files</Label>
                                </Switch>

                                <Switch
                                    isSelected={permissions.canCreateAgents}
                                    onChange={(value) => handlePermissionChange("canCreateAgents", value)}
                                >
                                    <Switch.Control>
                                        <Switch.Thumb />
                                    </Switch.Control>
                                    <Label>Allow creating AI agents</Label>
                                </Switch>
                            </div>
                        </div>
                    </div>
                </div>

                <div className="flex items-center justify-end gap-3 p-6 border-t">
                    <Button variant="secondary" onPress={onClose}>
                        Cancel
                    </Button>
                    <Button
                        onPress={handleSave}
                        isDisabled={!name.trim() || isLoading}
                    >
                        {isLoading ? "Saving..." : "Save Changes"}
                    </Button>
                </div>
            </Card>
        </div>
    );
}