import { useState } from "react";
import { TextArea, Button, Select, Label, ListBox } from "@heroui/react";
import { Play, Sparkles } from "lucide-react";
import { AI_PROVIDERS, type ProviderType } from "../../types";

interface TaskInputProps {
    onSubmit?: (task: string, provider: ProviderType) => void;
    isLoading?: boolean;
}

export function TaskInput({ onSubmit, isLoading = false }: TaskInputProps) {
    const [taskDescription, setTaskDescription] = useState("");
    const [selectedProvider, setSelectedProvider] = useState<ProviderType>("openai");

    const handleSubmit = () => {
        if (taskDescription.trim() && onSubmit) {
            onSubmit(taskDescription.trim(), selectedProvider);
            setTaskDescription("");
        }
    };

    const handleKeyDown = (e: React.KeyboardEvent) => {
        if (e.key === "Enter" && e.metaKey) {
            e.preventDefault();
            handleSubmit();
        }
    };

    const selectedProviderObj = AI_PROVIDERS.find((p) => p.id === selectedProvider);

    return (
        <div className="space-y-4">
            <div className="flex items-center gap-2">
                <Sparkles className="size-5 text-primary shrink-0" />
                <h2 className="text-lg font-semibold">New Task</h2>
            </div>

            <div className="space-y-4">
                <TextArea
                    value={taskDescription}
                    onChange={(e) => setTaskDescription(e.target.value)}
                    onKeyDown={handleKeyDown}
                    placeholder="What would you like me to do? (⌘+Enter to submit)"
                    className="min-h-[100px] sm:min-h-[120px] w-full"
                    disabled={isLoading}
                    rows={4}
                    aria-label="Task description"
                />

                <div className="flex flex-col sm:flex-row items-stretch sm:items-center justify-between gap-3 sm:gap-4">
                    <div className="flex items-center gap-2">
                        <Label className="text-sm text-muted-foreground whitespace-nowrap">AI Provider:</Label>
                        <Select
                            aria-label="Select AI Provider"
                            className="w-full sm:w-48"
                            placeholder="Select provider"
                            selectedKey={selectedProvider}
                            onSelectionChange={(key) => setSelectedProvider(key as ProviderType)}
                        >
                            <Select.Trigger>
                                <Select.Value>
                                    {selectedProviderObj?.name || "Select provider"}
                                </Select.Value>
                                <Select.Indicator />
                            </Select.Trigger>
                            <Select.Popover>
                                <ListBox>
                                    {AI_PROVIDERS.map((provider) => (
                                        <ListBox.Item
                                            key={provider.id}
                                            id={provider.id}
                                            textValue={provider.name}
                                        >
                                            <div className="flex flex-col">
                                                <span className="font-medium">{provider.name}</span>
                                                <span className="text-xs text-muted-foreground">{provider.model}</span>
                                            </div>
                                            <ListBox.ItemIndicator />
                                        </ListBox.Item>
                                    ))}
                                </ListBox>
                            </Select.Popover>
                        </Select>
                    </div>

                    <Button
                        variant="primary"
                        size="md"
                        onPress={handleSubmit}
                        isDisabled={!taskDescription.trim() || isLoading}
                        className="w-full sm:w-auto"
                    >
                        <Play className="size-4" />
                        Start Task
                    </Button>
                </div>
            </div>
        </div>
    );
}
