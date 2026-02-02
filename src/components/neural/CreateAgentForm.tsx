import { Button } from "@heroui/react";
import { useState } from "react";
import { createAtmAgent } from "../../services/tauri";
import { toast } from "@heroui/react";
import { Save, ArrowLeft } from "lucide-react";

interface CreateAgentFormProps {
  onSuccess: () => void;
  onCancel: () => void;
}

const AGENT_TYPES = [
  { key: "chat", label: "Chat Assistant" },
  { key: "task", label: "Task Worker" },
  { key: "researcher", label: "Researcher" },
];

export function CreateAgentForm({ onSuccess, onCancel }: CreateAgentFormProps) {
  const [name, setName] = useState("");
  const [type, setType] = useState("chat");
  const [prompt, setPrompt] = useState("");
  const [isSubmitting, setIsSubmitting] = useState(false);

  const handleSubmit = async () => {
    if (!name.trim() || !prompt.trim()) {
      toast.danger("Name and System Prompt are required");
      return;
    }

    setIsSubmitting(true);
    try {
      const config = {
        prompt,
        // Default config that can be expanded later
        model: "gemini-1.5-flash",
        temperature: 0.7,
      };

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
