import { useState, useEffect } from "react";
import { Zap, Bot, Database } from "lucide-react";
import * as tauri from "../../../services/tauri";
import { useAIProvider } from "../../../hooks";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Separator } from "@/components/ui/separator";
import { Card } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";

const ModelCard = ({
  name,
  description,
}: {
  name: string;
  description: string;
}) => (
  <Card className="p-4 bg-muted/20 border-border/10 hover:bg-muted/30 transition-colors group">
    <div className="flex-1">
      <span className="font-semibold text-sm group-hover:text-primary transition-colors">{name}</span>
      <p className="text-xs text-muted-foreground mt-1 leading-relaxed">{description}</p>
    </div>
  </Card>
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
          <Skeleton className="h-4 w-32" />
          <Skeleton className="h-10 w-full max-w-sm" />
          <Skeleton className="h-10 w-full max-w-sm" />
        </div>
        <Separator className="my-6 opacity-20" />
        <div className="space-y-4">
          <Skeleton className="h-4 w-40" />
          <div className="grid gap-3">
            <Skeleton className="h-20 w-full" />
            <Skeleton className="h-20 w-full" />
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="animate-in fade-in slide-in-from-bottom-2 duration-500">
      <div className="space-y-8">
        <section className="space-y-4">
          <div className="flex items-center gap-2 mb-2">
            <Bot className="size-4 text-primary" />
            <h3 className="text-sm font-semibold uppercase tracking-wider text-foreground/70">
              Embedder Configurations
            </h3>
          </div>
          
          <div className="grid gap-4 max-w-sm">
            <div className="space-y-2">
              <label className="text-xs font-medium text-muted-foreground ml-1">Provider</label>
              <Select value={embedderProvider} onValueChange={handleEmbedderProviderChange} disabled>
                <SelectTrigger className="w-full bg-muted/20 border-border/10 h-10 px-4">
                  <SelectValue placeholder="Select provider" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="gemini">Gemini (Active)</SelectItem>
                  <SelectItem value="openai">OpenAI (Coming Soon)</SelectItem>
                </SelectContent>
              </Select>
            </div>

            <div className="space-y-2">
              <label className="text-xs font-medium text-muted-foreground ml-1">Model</label>
              <Select value={embedderModel} onValueChange={handleEmbedderModelChange} disabled>
                <SelectTrigger className="w-full bg-muted/20 border-border/10 h-10 px-4">
                  <SelectValue placeholder="Select model" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="gemini-embedding-001">gemini-embedding-001 (3072d)</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>
        </section>

        <section className="space-y-4">
          <div className="flex items-center gap-2 mb-2">
            <Database className="size-4 text-primary" />
            <h3 className="text-sm font-semibold uppercase tracking-wider text-foreground/70">
              Memory Storage
            </h3>
          </div>
          
          <div className="space-y-2 max-w-sm">
            <label className="text-xs font-medium text-muted-foreground ml-1">Vector Store</label>
            <Select value="turso" disabled>
              <SelectTrigger className="w-full bg-muted/20 border-border/10 h-10 px-4">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="turso">Turso (libSQL Enclave)</SelectItem>
              </SelectContent>
            </Select>
            <p className="text-[10px] text-muted-foreground pt-1 ml-1 italic opacity-60">
              * Currently locked to Turso for hardware-backed encryption.
            </p>
          </div>
        </section>
      </div>

      <Separator className="my-10 opacity-10" />

      <div className="space-y-10">
        {hasApiKey("rainy_api") && (
          <section className="space-y-4">
            <h3 className="text-sm font-medium text-muted-foreground mb-4 flex items-center gap-2">
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
            <h3 className="text-sm font-medium text-muted-foreground mb-4 flex items-center gap-2">
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
