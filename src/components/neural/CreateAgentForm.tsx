import {
  Button,
  Select,
  ListBox,
  Slider,
  TextArea,
  TextField,
  Input,
  Label,
  Description,
  Separator,
  Spinner,
} from "@heroui/react";
import { useState, useEffect } from "react";
import { createAtmAgent } from "../../services/tauri";
import { toast } from "@heroui/react";
import { invoke } from "@tauri-apps/api/core";
import {
  Save,
  Eye,
  EyeOff,
  Bot,
  Sparkles,
  Cpu,
  ChevronDown,
} from "lucide-react";
import * as tauri from "../../services/tauri";

interface CreateAgentFormProps {
  onSuccess: () => void;
  onCancel: () => void;
}

const AGENT_TYPES = [
  { key: "chat", label: "Chat Assistant", icon: <Bot className="size-4" /> },
  { key: "task", label: "Task Worker", icon: <Cpu className="size-4" /> },
  {
    key: "researcher",
    label: "Researcher",
    icon: <Sparkles className="size-4" />,
  },
];

export function CreateAgentForm({ onSuccess, onCancel }: CreateAgentFormProps) {
  const [name, setName] = useState("");
  const [type, setType] = useState("chat");
  const [prompt, setPrompt] = useState("");
  const [model, setModel] = useState("");
  const [temperature, setTemperature] = useState(0.7);
  const [maxTokens, setMaxTokens] = useState(8192);
  const [showPreview, setShowPreview] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);

  const [availableModels, setAvailableModels] = useState<tauri.UnifiedModel[]>(
    [],
  );
  const [loadingModels, setLoadingModels] = useState(false);

  useEffect(() => {
    const fetchModels = async () => {
      setLoadingModels(true);
      try {
        const models = await tauri.getUnifiedModels();
        setAvailableModels(models || []);
        // Auto-select first model if none selected and models exist
        if (models && models.length > 0 && !model) {
          setModel(models[0].id);
        }
      } catch (e) {
        console.error("Failed to load models", e);
      } finally {
        setLoadingModels(false);
      }
    };
    fetchModels();
  }, []);

  const config = {
    systemPrompt: prompt,
    model,
    temperature,
    maxTokens,
    provider: "rainy" as const,
  };

  const handleSubmit = async () => {
    if (!name.trim() || !prompt.trim()) {
      toast.danger("Name and System Prompt are required");
      return;
    }

    setIsSubmitting(true);
    try {
      // 1. Save locally to SQLite
      const agentId = crypto.randomUUID();
      await invoke("save_agent_to_db", {
        id: agentId,
        name,
        description: type, // Using type as description for now
        soul: prompt,
      });

      // 2. Deploy to Cloud (ATM)
      await createAtmAgent(name, type, config);

      toast.success(`Agent "${name}" deployed and saved locally`);
      onSuccess();
    } catch (error) {
      console.error("Failed to create agent:", error);
      toast.danger("Failed to create agent");
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="flex flex-col gap-3 p-1 text-foreground relative z-50">
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <TextField className="w-full text-foreground group">
          <Label className="text-[10px] font-bold uppercase tracking-widest text-muted-foreground/70 mb-1.5 ml-1 group-focus-within:text-primary transition-colors">
            Agent Name
          </Label>
          <Input
            placeholder="e.g., Sales Assistant"
            value={name}
            onChange={(e) => setName(e.target.value)}
            className="w-full h-10 bg-default-100/50 dark:bg-background/80 backdrop-blur-sm border-default-200/50 dark:border-white/10 hover:border-default-300 dark:hover:border-white/20 focus:border-primary/50 transition-all rounded-xl text-sm text-foreground placeholder:text-muted-foreground/50 shadow-sm"
          />
        </TextField>

        <Select
          className="w-full group"
          selectedKey={type}
          onSelectionChange={(key) => setType(key as string)}
        >
          <Label className="text-[10px] font-bold uppercase tracking-widest text-muted-foreground/70 mb-1.5 ml-1 group-focus-within:text-primary transition-colors">
            Agent Type
          </Label>
          <Select.Trigger className="h-10 bg-default-100/50 dark:bg-zinc-800/80 backdrop-blur-sm border-default-200/50 dark:border-white/10 hover:border-default-300 dark:hover:border-white/20 focus:border-primary/50 transition-all rounded-xl text-sm text-foreground shadow-sm">
            <Select.Value className="flex items-center gap-2 text-foreground">
              {({ selectedItem, defaultChildren }) => {
                const selectedKey = (selectedItem as any)?.key;
                const selectedType = AGENT_TYPES.find(
                  (t) => t.key === selectedKey,
                );
                return (
                  <div className="flex items-center gap-2">
                    <div className="text-primary">{selectedType?.icon}</div>
                    <span className="text-foreground font-medium">
                      {selectedType?.label || defaultChildren}
                    </span>
                  </div>
                );
              }}
            </Select.Value>
            <Select.Indicator>
              <ChevronDown className="size-4 opacity-50 text-foreground" />
            </Select.Indicator>
          </Select.Trigger>
          <Select.Popover className="bg-background dark:bg-background/20 border border-default-200 dark:border-white/10 backdrop-blur-sm">
            <ListBox className="bg-transparent text-foreground p-1">
              {AGENT_TYPES.map((t) => (
                <ListBox.Item
                  key={t.key}
                  id={t.key}
                  textValue={t.label}
                  className="rounded-lg data-[hover=true]:bg-default-100 dark:data-[hover=true]:bg-white/5"
                >
                  <div className="flex items-center gap-2 py-1">
                    <div className="text-primary/70">{t.icon}</div>
                    <span className="font-medium">{t.label}</span>
                  </div>
                </ListBox.Item>
              ))}
            </ListBox>
          </Select.Popover>
        </Select>
      </div>

      <Select
        className="w-full group"
        selectedKey={model}
        onSelectionChange={(key) => setModel(key as string)}
        isDisabled={loadingModels}
      >
        <Label className="text-[10px] font-bold uppercase tracking-widest text-muted-foreground/70 mb-1.5 ml-1 group-focus-within:text-primary transition-colors">
          Model
        </Label>
        <Select.Trigger className="h-10 font-mono text-xs bg-background/50 dark:bg-background/20 backdrop-blur-sm border-default-200/50 dark:border-white/10 hover:border-default-300 dark:hover:border-white/20 focus:border-primary/50 transition-all rounded-xl text-foreground shadow-sm">
          <Select.Value className="text-foreground" />
          <Select.Indicator>
            <ChevronDown className="size-4 opacity-50 text-foreground" />
          </Select.Indicator>
        </Select.Trigger>
        <Description className="text-[10px] mt-1.5 text-muted-foreground ml-1 font-medium">
          Select any available model from your providers
        </Description>
        <Select.Popover className="bg-background/60 dark:bg-background/20 backdrop-blur-sm dark:border-white/10 rounded-xl">
          <ListBox
            className="bg-background/20 text-foreground p-1"
            items={availableModels}
          >
            {(m) => (
              <ListBox.Item
                key={m.id}
                id={m.id}
                textValue={m.name}
                className="rounded-lg data-[hover=true]:bg-background/50 dark:data-[hover=true]:bg-background/5 py-2"
              >
                <div className="flex flex-col gap-0.5">
                  <div className="flex justify-between items-center">
                    <span className="font-medium text-sm text-foreground">
                      {m.name}
                    </span>
                    <span className="text-[9px] uppercase font-bold text-muted-foreground/50 border border-white/5 px-1 rounded">
                      {m.provider}
                    </span>
                  </div>

                  <span className="text-[10px] text-muted-foreground">
                    {m.id}
                  </span>
                </div>
              </ListBox.Item>
            )}
          </ListBox>
        </Select.Popover>
      </Select>

      <div className="space-y-8 py-2">
        <Slider
          maxValue={1}
          minValue={0}
          step={0.05}
          value={temperature}
          onChange={(v) => setTemperature(v as number)}
          className="max-w-full"
        >
          <div className="flex justify-between items-center mb-2">
            <Label className="text-[10px] font-bold uppercase tracking-widest text-muted-foreground/70">
              Temperature
            </Label>
            <Slider.Output className="text-[10px] font-mono bg-primary/10 text-primary px-2 py-0.5 rounded-md font-bold">
              {temperature.toFixed(2)} (
              {temperature > 0.6
                ? "Creative"
                : temperature < 0.4
                  ? "Precise"
                  : "Balanced"}
              )
            </Slider.Output>
          </div>
          <Slider.Track className="h-1.5 bg-default-200 dark:bg-white/10 rounded-full">
            <Slider.Fill className="bg-primary h-full rounded-full" />
            <Slider.Thumb className="size-4 bg-background border-2 border-primary rounded-full shadow-md" />
          </Slider.Track>
        </Slider>

        <Slider
          maxValue={65536}
          minValue={1024}
          step={1024}
          value={maxTokens}
          onChange={(v) => setMaxTokens(v as number)}
          className="max-w-full"
        >
          <div className="flex justify-between items-center mb-2">
            <Label className="text-[10px] font-bold uppercase tracking-widest text-muted-foreground/70">
              Max Output Tokens
            </Label>
            <Slider.Output className="text-[10px] font-mono bg-primary/10 text-primary px-2 py-0.5 rounded-md font-bold">
              {maxTokens.toLocaleString()}
            </Slider.Output>
          </div>
          <Slider.Track className="h-1.5 bg-default-200 dark:bg-white/10 rounded-full">
            <Slider.Fill className="bg-primary h-full rounded-full" />
            <Slider.Thumb className="size-4 bg-background border-2 border-primary rounded-full shadow-md" />
          </Slider.Track>
        </Slider>
      </div>

      <TextField className="w-full group">
        <Label className="text-[10px] font-bold uppercase tracking-widest text-muted-foreground/70 mb-1.5 ml-1 group-focus-within:text-primary transition-colors">
          System Prompt
        </Label>
        <TextArea
          placeholder="You are a helpful assistant who..."
          value={prompt}
          onChange={(e) => setPrompt(e.target.value)}
          className="w-full min-h-[120px] bg-default-100/50 dark:bg-zinc-800/80 backdrop-blur-sm border-default-200/50 dark:border-white/10 hover:border-default-300 dark:hover:border-white/20 focus:border-primary/50 transition-all rounded-xl text-sm text-foreground placeholder:text-muted-foreground/50 shadow-sm p-3"
        />
      </TextField>

      <div className="space-y-3">
        <Button
          variant="tertiary"
          size="sm"
          onPress={() => setShowPreview(!showPreview)}
          className="text-default-500 h-8"
        >
          <div className="flex items-center gap-2">
            {showPreview ? (
              <EyeOff className="size-4" />
            ) : (
              <Eye className="size-4" />
            )}
            <span>{showPreview ? "Hide" : "Show"} Config Preview</span>
          </div>
        </Button>
        {showPreview && (
          <pre className="text-[10px] font-mono bg-default-100/50 p-4 rounded-2xl overflow-x-auto border border-default-200/50 backdrop-blur-sm">
            {JSON.stringify(config, null, 2)}
          </pre>
        )}
      </div>

      <Separator />

      <div className="flex justify-end gap-3 pt-2">
        <Button
          variant="outline"
          onPress={onCancel}
          className="font-medium h-12 px-6"
        >
          Cancel
        </Button>
        <Button
          variant="primary"
          onPress={handleSubmit}
          className="font-bold h-12 px-10 shadow-lg shadow-primary/20 transition-all hover:scale-105 active:scale-95 isDisabled:opacity-50"
          isDisabled={isSubmitting}
        >
          <div className="flex items-center gap-2">
            {isSubmitting ? (
              <Spinner size="sm" color="current" />
            ) : (
              <Save className="size-5" />
            )}
            <span>Deploy Agent</span>
          </div>
        </Button>
      </div>
    </div>
  );
}
