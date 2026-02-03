import { Button } from "@heroui/react";
import { useState } from "react";
import { createAtmAgent } from "../../services/tauri";
import { toast } from "@heroui/react";
import { Save, ArrowLeft, Eye, EyeOff } from "lucide-react";

interface CreateAgentFormProps {
  onSuccess: () => void;
  onCancel: () => void;
}

const AGENT_TYPES = [
  { key: "chat", label: "Chat Assistant" },
  { key: "task", label: "Task Worker" },
  { key: "researcher", label: "Researcher" },
];

// Gemini 3 family + 2.5-flash for 1M context
const GEMINI_MODELS = [
  {
    key: "gemini-3-pro-preview",
    label: "Gemini 3 Pro (1M context)",
    default: true,
  },
  { key: "gemini-3-flash-preview", label: "Gemini 3 Flash (Fast)" },
  {
    key: "gemini-2.5-flash-preview-05-20",
    label: "Gemini 2.5 Flash (1M context)",
  },
];

export function CreateAgentForm({ onSuccess, onCancel }: CreateAgentFormProps) {
  const [name, setName] = useState("");
  const [type, setType] = useState("chat");
  const [prompt, setPrompt] = useState("");
  const [model, setModel] = useState("gemini-3-pro-preview");
  const [temperature, setTemperature] = useState(0.7);
  const [maxTokens, setMaxTokens] = useState(8192);
  const [showPreview, setShowPreview] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);

  const config = {
    prompt,
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
      await createAtmAgent(name, type, config);
      toast.success(`Agent "${name}" deployed successfully`);
      onSuccess();
    } catch (error) {
      console.error("Failed to create agent:", error);
      toast.danger("Failed to create agent");
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="flex flex-col gap-5">
      <div className="flex items-center gap-2 mb-2">
        <Button size="sm" variant="ghost" isIconOnly onPress={onCancel}>
          <ArrowLeft className="size-4" />
        </Button>
        <h3 className="text-lg font-semibold">Deploy New Agent</h3>
      </div>

      <div className="flex flex-col gap-2">
        <label className="text-sm font-medium">Agent Name</label>
        <input
          placeholder="e.g., Sales Assistant"
          value={name}
          onChange={(e) => setName(e.target.value)}
          className="px-3 py-2 rounded-lg border bg-default-100 text-sm focus:outline-none focus:ring-2 focus:ring-primary"
        />
      </div>

      <div className="flex flex-col gap-2">
        <label className="text-sm font-medium">Agent Type</label>
        <select
          value={type}
          onChange={(e) => setType(e.target.value)}
          className="px-3 py-2 rounded-lg border bg-default-100 text-sm focus:outline-none focus:ring-2 focus:ring-primary"
        >
          {AGENT_TYPES.map((t) => (
            <option key={t.key} value={t.key}>
              {t.label}
            </option>
          ))}
        </select>
      </div>

      {/* Model Selector - Gemini 3 Family Only */}
      <div className="flex flex-col gap-2">
        <label className="text-sm font-medium">Model (Gemini 3 Family)</label>
        <select
          value={model}
          onChange={(e) => setModel(e.target.value)}
          className="px-3 py-2 rounded-lg border bg-default-100 text-sm focus:outline-none focus:ring-2 focus:ring-primary"
        >
          {GEMINI_MODELS.map((m) => (
            <option key={m.key} value={m.key}>
              {m.label}
            </option>
          ))}
        </select>
        <p className="text-xs text-muted-foreground">
          All models support up to 1M token input context
        </p>
      </div>

      {/* Temperature Slider */}
      <div className="flex flex-col gap-2">
        <div className="flex justify-between items-center">
          <label className="text-sm font-medium">Temperature</label>
          <span className="text-sm font-mono text-muted-foreground">
            {temperature.toFixed(2)}
          </span>
        </div>
        <input
          type="range"
          min="0"
          max="1"
          step="0.05"
          value={temperature}
          onChange={(e) => setTemperature(parseFloat(e.target.value))}
          className="w-full h-2 bg-default-200 rounded-lg appearance-none cursor-pointer accent-primary"
        />
        <div className="flex justify-between text-xs text-muted-foreground">
          <span>Precise (0)</span>
          <span>Creative (1)</span>
        </div>
      </div>

      {/* Max Tokens */}
      <div className="flex flex-col gap-2">
        <div className="flex justify-between items-center">
          <label className="text-sm font-medium">Max Output Tokens</label>
          <span className="text-sm font-mono text-muted-foreground">
            {maxTokens.toLocaleString()}
          </span>
        </div>
        <input
          type="range"
          min="1024"
          max="65536"
          step="1024"
          value={maxTokens}
          onChange={(e) => setMaxTokens(parseInt(e.target.value))}
          className="w-full h-2 bg-default-200 rounded-lg appearance-none cursor-pointer accent-primary"
        />
        <div className="flex justify-between text-xs text-muted-foreground">
          <span>1K</span>
          <span>32K</span>
          <span>60K+</span>
        </div>
      </div>

      <div className="flex flex-col gap-2">
        <label className="text-sm font-medium">System Prompt</label>
        <textarea
          placeholder="You are a helpful assistant who..."
          value={prompt}
          onChange={(e) => setPrompt(e.target.value)}
          rows={5}
          className="px-3 py-2 rounded-lg border bg-default-100 text-sm focus:outline-none focus:ring-2 focus:ring-primary resize-y"
        />
      </div>

      {/* Config Preview */}
      <div className="flex flex-col gap-2">
        <button
          type="button"
          onClick={() => setShowPreview(!showPreview)}
          className="flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground transition-colors"
        >
          {showPreview ? (
            <EyeOff className="size-4" />
          ) : (
            <Eye className="size-4" />
          )}
          {showPreview ? "Hide" : "Show"} Config Preview
        </button>
        {showPreview && (
          <pre className="text-xs font-mono bg-default-100 p-3 rounded-lg overflow-x-auto">
            {JSON.stringify(config, null, 2)}
          </pre>
        )}
      </div>

      <div className="flex justify-end gap-2 mt-4">
        <Button variant="ghost" onPress={onCancel}>
          Cancel
        </Button>
        <Button
          variant="primary"
          onPress={handleSubmit}
          isDisabled={isSubmitting}
        >
          {isSubmitting ? (
            <div className="size-4 mr-2 border-2 border-current border-t-transparent rounded-full animate-spin" />
          ) : (
            <Save className="size-4 mr-2" />
          )}
          Deploy Agent
        </Button>
      </div>
    </div>
  );
}
