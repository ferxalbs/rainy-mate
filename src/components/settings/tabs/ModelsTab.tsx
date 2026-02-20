import { useState, useEffect } from "react";
import { Spinner, Select, ListBox, Separator } from "@heroui/react";
import { Zap, Bot } from "lucide-react";
import * as tauri from "../../../services/tauri";
import { useAIProvider } from "../../../hooks";

const selectionToValue = (selection: unknown): string | null => {
  if (typeof selection === "string") return selection;
  if (selection instanceof Set) {
    const first = selection.values().next().value;
    return typeof first === "string" ? first : null;
  }
  return null;
};

const ModelCard = ({
  name,
  description,
}: {
  name: string;
  description: string;
}) => (
  <div className="p-4 rounded-xl border border-transparent bg-transparent">
    <div className="flex-1">
      <span className="font-medium">{name}</span>
      <p className="text-sm text-muted-foreground mt-1">{description}</p>
    </div>
  </div>
);

export function ModelsTab() {
  const [isLoading, setIsLoading] = useState(true);
  const [rainyApiModels, setRainyApiModels] = useState<string[]>([]);
  const [geminiModels, setGeminiModels] = useState<string[]>([]);
  const [embedderProvider, setEmbedderProvider] = useState<string>("gemini");
  const [embedderModel, setEmbedderModel] = useState<string>(
    "gemini-embedding-001",
  );
  const { hasApiKey } = useAIProvider();

  useEffect(() => {
    async function loadData() {
      try {
        const [rainyModels, geminiModelsList, eProvider, eModel] =
          await Promise.all([
            tauri.getProviderModels("rainy_api").catch(() => []),
            tauri.getProviderModels("gemini").catch(() => []),
            tauri.getEmbedderProvider().catch(() => "gemini"),
            tauri.getEmbedderModel().catch(() => "gemini-embedding-001"),
          ]);
        setRainyApiModels(rainyModels || []);
        setGeminiModels(geminiModelsList || []);
        setEmbedderProvider("gemini");
        setEmbedderModel("gemini-embedding-001");

        if (eProvider !== "gemini")
          tauri.setEmbedderProvider("gemini").catch(console.error);
        if (eModel !== "gemini-embedding-001")
          tauri.setEmbedderModel("gemini-embedding-001").catch(console.error);
      } catch (error) {
        console.error("Failed to load settings:", error);
      } finally {
        setIsLoading(false);
      }
    }
    loadData();
  }, []);

  const handleEmbedderProviderChange = async (provider: string) => {
    setEmbedderProvider(provider);
    try {
      await tauri.setEmbedderProvider(provider);
    } catch (error) {
      console.error("Failed to save embedder provider:", error);
    }
  };

  const handleEmbedderModelChange = async (model: string) => {
    setEmbedderModel(model);
    try {
      await tauri.setEmbedderModel(model);
    } catch (error) {
      console.error("Failed to save embedder model:", error);
    }
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-12">
        <Spinner size="lg" />
      </div>
    );
  }

  return (
    <>
      <div className="space-y-6">
        <div>
          <h3 className="text-sm font-medium text-foreground mb-3">
            Embedder Provider
          </h3>
          <Select
            className="w-full max-w-sm bg-background/30"
            isDisabled
            aria-label="Embedder Provider"
          >
            <Select.Trigger>
              <Select.Value />
              <Select.Indicator />
            </Select.Trigger>
            <Select.Popover>
              <ListBox
                selectionMode="single"
                selectedKeys={new Set([embedderProvider])}
                onSelectionChange={(selection: unknown) => {
                  const val = selectionToValue(selection);
                  if (val) handleEmbedderProviderChange(val);
                }}
              >
                <ListBox.Item id="gemini" textValue="Gemini">
                  Gemini
                  <ListBox.ItemIndicator />
                </ListBox.Item>
                <ListBox.Item id="openai" textValue="OpenAI (Coming Soon)">
                  OpenAI (Coming Soon)
                  <ListBox.ItemIndicator />
                </ListBox.Item>
              </ListBox>
            </Select.Popover>
          </Select>

          <Select
            className="w-full max-w-sm mt-3"
            isDisabled
            aria-label="Embedder Model"
            placeholder="Select a model"
          >
            <Select.Trigger>
              <Select.Value />
              <Select.Indicator />
            </Select.Trigger>
            <Select.Popover>
              <ListBox
                selectionMode="single"
                selectedKeys={new Set([embedderModel])}
                onSelectionChange={(selection: unknown) => {
                  const val = selectionToValue(selection);
                  if (val) handleEmbedderModelChange(val);
                }}
              >
                <ListBox.Item
                  id="gemini-embedding-001"
                  textValue="gemini-embedding-001 (3072 dimensions)"
                >
                  gemini-embedding-001 (3072 dimensions)
                  <ListBox.ItemIndicator />
                </ListBox.Item>
              </ListBox>
            </Select.Popover>
          </Select>
        </div>

        <div className="pt-2">
          <h3 className="text-sm font-medium text-foreground mb-3">
            Vector Store Provider
          </h3>
          <Select
            className="w-full max-w-sm"
            isDisabled
            aria-label="Vector Store Provider"
          >
            <Select.Trigger>
              <Select.Value />
              <Select.Indicator />
            </Select.Trigger>
            <Select.Popover>
              <ListBox selectionMode="single" selectedKeys={new Set(["turso"])}>
                <ListBox.Item id="turso" textValue="Turso (libSQL)">
                  Turso (libSQL)
                  <ListBox.ItemIndicator />
                </ListBox.Item>
              </ListBox>
            </Select.Popover>
          </Select>
          <p className="text-xs text-muted-foreground mt-2">
            Currently locked to Turso for encrypted memory vault.
          </p>
        </div>
      </div>

      <Separator className="my-6" />

      {hasApiKey("rainy_api") && (
        <div>
          <h3 className="text-sm font-medium text-muted-foreground mb-3 flex items-center gap-2">
            <Zap className="size-4" />
            Pay-As-You-Go Models (Rainy API)
          </h3>
          <div className="grid gap-3">
            {rainyApiModels.map((model) => (
              <ModelCard
                key={model}
                name={model}
                description="Billed per usage (1:1 Token)"
              />
            ))}
          </div>
        </div>
      )}

      {hasApiKey("gemini") && (
        <div>
          <h3 className="text-sm font-medium text-muted-foreground mb-3 flex items-center gap-2">
            <Bot className="size-4" />
            Free Tier (Gemini BYOK)
          </h3>
          <div className="grid gap-3">
            {geminiModels.map((model) => (
              <ModelCard
                key={model}
                name={model}
                description="Uses your own Gemini API Key"
              />
            ))}
          </div>
        </div>
      )}
    </>
  );
}
