import { useState } from "react";
import { TextArea, Button, Select, Label, ListBox } from "@heroui/react";
import { Play, Sparkles } from "lucide-react";
import { AI_PROVIDERS, PROVIDER_MODELS, type ProviderType } from "../../types";

interface TaskInputProps {
  onSubmit?: (task: string, provider: ProviderType, model: string) => void;
  isLoading?: boolean;
}

export function TaskInput({ onSubmit, isLoading = false }: TaskInputProps) {
  const [taskDescription, setTaskDescription] = useState("");
  const [selectedProvider, setSelectedProvider] =
    useState<ProviderType>("rainyapi");
  const [selectedModel, setSelectedModel] = useState("gpt-4o");

  // Update model when provider changes
  const handleProviderChange = (provider: ProviderType) => {
    setSelectedProvider(provider);
    const models = PROVIDER_MODELS[provider];
    setSelectedModel(models[0]); // Default to first model
  };

  const handleSubmit = () => {
    if (taskDescription.trim() && onSubmit) {
      onSubmit(taskDescription.trim(), selectedProvider, selectedModel);
      setTaskDescription("");
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && e.metaKey) {
      e.preventDefault();
      handleSubmit();
    }
  };

  const selectedProviderObj = AI_PROVIDERS.find(
    (p) => p.id === selectedProvider,
  );
  const availableModels = PROVIDER_MODELS[selectedProvider] || [];

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-2">
        <Sparkles className="size-4 text-primary shrink-0" />
        <h2 className="text-sm font-semibold">New Task</h2>
      </div>

      <div className="space-y-4">
        <TextArea
          value={taskDescription}
          onChange={(e) => setTaskDescription(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="What would you like me to do? (⌘+Enter to submit)"
          className="min-h-[100px] w-full bg-muted/30 border-0 focus:ring-1 focus:ring-primary/30 rounded-xl"
          disabled={isLoading}
          rows={4}
          aria-label="Task description"
        />

        <div className="flex flex-col sm:flex-row items-stretch sm:items-center justify-between gap-3">
          <div className="flex items-center gap-2 flex-wrap">
            {/* Provider Select */}
            <div className="flex items-center gap-2">
              <Label className="text-xs text-muted-foreground whitespace-nowrap">
                Provider:
              </Label>
              <Select
                aria-label="Select AI Provider"
                className="w-36"
                placeholder="Select provider"
                selectedKey={selectedProvider}
                onSelectionChange={(key) =>
                  handleProviderChange(key as ProviderType)
                }
              >
                <Select.Trigger className="bg-muted/30 border-0 rounded-lg h-8">
                  <Select.Value>
                    {selectedProviderObj?.name || "Select provider"}
                  </Select.Value>
                  <Select.Indicator />
                </Select.Trigger>
                <Select.Popover className="rounded-xl">
                  <ListBox>
                    {AI_PROVIDERS.map((provider) => (
                      <ListBox.Item
                        key={provider.id}
                        id={provider.id}
                        textValue={provider.name}
                      >
                        <div className="flex flex-col">
                          <span className="font-medium text-sm">
                            {provider.name}
                          </span>
                          <span className="text-xs text-muted-foreground">
                            {provider.description}
                          </span>
                        </div>
                        <ListBox.ItemIndicator />
                      </ListBox.Item>
                    ))}
                  </ListBox>
                </Select.Popover>
              </Select>
            </div>

            {/* Model Select */}
            <div className="flex items-center gap-2">
              <Label className="text-xs text-muted-foreground whitespace-nowrap">
                Model:
              </Label>
              <Select
                aria-label="Select Model"
                className="w-40"
                placeholder="Select model"
                selectedKey={selectedModel}
                onSelectionChange={(key) => setSelectedModel(key as string)}
              >
                <Select.Trigger className="bg-muted/30 border-0 rounded-lg h-8">
                  <Select.Value>{selectedModel}</Select.Value>
                  <Select.Indicator />
                </Select.Trigger>
                <Select.Popover className="rounded-xl">
                  <ListBox>
                    {availableModels.map((model) => (
                      <ListBox.Item key={model} id={model} textValue={model}>
                        <span className="text-sm">{model}</span>
                        <ListBox.ItemIndicator />
                      </ListBox.Item>
                    ))}
                  </ListBox>
                </Select.Popover>
              </Select>
            </div>
          </div>

          <Button
            variant="primary"
            size="sm"
            onPress={handleSubmit}
            isDisabled={!taskDescription.trim() || isLoading}
            className="w-full sm:w-auto rounded-lg px-4"
          >
            <Play className="size-3.5" />
            {isLoading ? "Starting..." : "Start Task"}
          </Button>
        </div>
      </div>
    </div>
  );
}
