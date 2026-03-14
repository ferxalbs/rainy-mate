import { useState, useEffect } from "react";
import { Zap, Bot, Database } from "lucide-react";
import * as tauri from "../../../services/tauri";
import { useAIProvider } from "../../../hooks";
import { Select, ListBox, Card, Skeleton } from "@heroui/react";

const ModelCard = ({
  name,
  description,
}: {
  name: string;
  description: string;
}) => (
  <Card className="bg-success/5 border border-success/10 shadow-none hover:bg-success/10 transition-colors group p-4 rounded-xl">
    <div className="flex-1">
      <span className="font-bold text-sm text-foreground/90 group-hover:text-success transition-colors">{name}</span>
      <p className="text-xs text-muted-foreground mt-1 leading-relaxed">{description}</p>
    </div>
  </Card>
);

const selectionToValue = (selection: unknown): string | null => {
  if (typeof selection === "string") return selection;
  if (selection && typeof selection === "object" && "values" in selection) {
    // Check if it's a Set
    if (selection instanceof Set) {
      const first = selection.values().next().value;
      return typeof first === "string" ? first : null;
    }
  }
  return null;
};

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

  const handleEmbedderProviderChange = async (provider: string | null) => {
    if (!provider) return;
    setEmbedderProvider(provider);
    try {
      await tauri.setEmbedderProvider(provider);
    } catch (error) {
      console.error("Failed to save embedder provider:", error);
    }
  };

  const handleEmbedderModelChange = async (model: string | null) => {
    if (!model) return;
    setEmbedderModel(model);
    try {
      await tauri.setEmbedderModel(model);
    } catch (error) {
      console.error("Failed to save embedder model:", error);
    }
  };

  if (isLoading) {
    return (
      <div className="space-y-6">
        <div className="space-y-3">
          <Skeleton className="h-4 w-32 rounded-md" />
          <Skeleton className="h-10 w-full max-w-sm rounded-xl" />
          <Skeleton className="h-10 w-full max-w-sm rounded-xl" />
        </div>
        <div className="h-px bg-success/10 w-full my-6 opacity-20" />
        <div className="space-y-4">
          <Skeleton className="h-4 w-40 rounded-md" />
          <div className="grid gap-3">
            <Skeleton className="h-20 w-full rounded-2xl" />
            <Skeleton className="h-20 w-full rounded-2xl" />
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="animate-in fade-in duration-500">
      <div className="space-y-8">
        <section className="space-y-4">
          <div className="flex items-center gap-2 mb-2">
            <Bot className="size-4 text-success" />
            <h3 className="text-sm font-bold uppercase tracking-wider text-foreground/70">
              Embedder Configurations
            </h3>
          </div>
          
          <div className="grid gap-4 max-w-sm">
            <div className="space-y-1">
              <label className="text-xs font-medium text-muted-foreground ml-1">Provider</label>
              <Select 
                className="w-full"
                selectedKey={embedderProvider}
                placeholder="Select provider"
                onSelectionChange={(selection) => {
                  const value = selectionToValue(selection);
                  if (!value) return;
                  handleEmbedderProviderChange(value);
                }}
                isDisabled
              >
                <Select.Trigger className="h-10 px-4 bg-success/5 border border-success/10 rounded-xl hover:bg-success/10 text-foreground">
                  <Select.Value />
                  <Select.Indicator />
                </Select.Trigger>
                <Select.Popover className="bg-background/95 dark:bg-background/35 border border-default-200/70 dark:border-white/15 backdrop-blur-xl">
                  <ListBox className="bg-transparent">
                    <ListBox.Item id="gemini" textValue="Gemini (Active)">Gemini (Active)</ListBox.Item>
                    <ListBox.Item id="openai" textValue="OpenAI (Coming Soon)">OpenAI (Coming Soon)</ListBox.Item>
                  </ListBox>
                </Select.Popover>
              </Select>
            </div>

            <div className="space-y-1">
              <label className="text-xs font-medium text-muted-foreground ml-1">Model</label>
              <Select 
                className="w-full"
                selectedKey={embedderModel}
                placeholder="Select model"
                onSelectionChange={(selection) => {
                  const value = selectionToValue(selection);
                  if (!value) return;
                  handleEmbedderModelChange(value);
                }}
                isDisabled
              >
                <Select.Trigger className="h-10 px-4 bg-success/5 border border-success/10 rounded-xl hover:bg-success/10 text-foreground">
                  <Select.Value />
                  <Select.Indicator />
                </Select.Trigger>
                <Select.Popover className="bg-background/95 dark:bg-background/35 border border-default-200/70 dark:border-white/15 backdrop-blur-xl">
                  <ListBox className="bg-transparent">
                    <ListBox.Item id="gemini-embedding-001" textValue="gemini-embedding-001 (3072d)">gemini-embedding-001 (3072d)</ListBox.Item>
                  </ListBox>
                </Select.Popover>
              </Select>
            </div>
          </div>
        </section>

        <section className="space-y-4">
          <div className="flex items-center gap-2 mb-2">
            <Database className="size-4 text-success" />
            <h3 className="text-sm font-bold uppercase tracking-wider text-foreground/70">
              Memory Storage
            </h3>
          </div>
          
          <div className="space-y-1 max-w-sm">
            <label className="text-xs font-medium text-muted-foreground ml-1">Vector Store</label>
            <Select 
              className="w-full"
              selectedKey="turso"
              placeholder="Select store"
              isDisabled
            >
              <Select.Trigger className="h-10 px-4 bg-success/5 border border-success/10 rounded-xl hover:bg-success/10 text-foreground">
                <Select.Value />
                <Select.Indicator />
              </Select.Trigger>
              <Select.Popover className="bg-background/95 dark:bg-background/35 border border-default-200/70 dark:border-white/15 backdrop-blur-xl">
                <ListBox className="bg-transparent">
                  <ListBox.Item id="turso" textValue="Turso (libSQL Enclave)">Turso (libSQL Enclave)</ListBox.Item>
                </ListBox>
              </Select.Popover>
            </Select>
            <p className="text-[10px] text-muted-foreground pt-1 ml-1 italic opacity-60">
              * Currently locked to Turso for hardware-backed encryption.
            </p>
          </div>
        </section>
      </div>

      <div className="h-px bg-success/10 w-full my-10" />

      <div className="space-y-10">
        {hasApiKey("rainy_api") && (
          <section className="space-y-4">
            <h3 className="text-sm font-bold text-muted-foreground mb-4 flex items-center gap-2">
              <Zap className="size-4 text-amber-500" />
              Pay-As-You-Go Models (Rainy API)
            </h3>
            <div className="grid gap-3 sm:grid-cols-2">
              {rainyApiModels.map((model) => (
                <ModelCard
                  key={model}
                  name={model}
                  description="High-performance billing via 1:1 token usage."
                />
              ))}
            </div>
          </section>
        )}

        {hasApiKey("gemini") && (
          <section className="space-y-4">
            <h3 className="text-sm font-bold text-muted-foreground mb-4 flex items-center gap-2">
              <Bot className="size-4 text-blue-400" />
              Free Tier Models (Bring Your Own Key)
            </h3>
            <div className="grid gap-3 sm:grid-cols-2">
              {geminiModels.map((model) => (
                <ModelCard
                  key={model}
                  name={model}
                  description="Experimental access via personal Gemini API key."
                />
              ))}
            </div>
          </section>
        )}
      </div>
    </div>
  );
}
