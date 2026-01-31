import { useState, useEffect, useMemo } from "react";
import {
  Button,
  Popover,
  PopoverTrigger,
  PopoverContent,
  Input,
} from "@heroui/react";
import {
  Check,
  ChevronDown,
  Search,
  Sparkles,
  Zap,
  Cpu,
  Globe,
  Brain,
  Filter,
} from "lucide-react";
import * as tauri from "../../services/tauri";

// Define UnifiedModel interface based on backend struct
export interface UnifiedModel {
  id: string;
  name: string;
  provider: string; // rainy, cowork, openai, anthropic, xai, local
  capabilities: {
    chat: boolean;
    streaming: boolean;
    function_calling: boolean;
    vision: boolean;
    web_search: boolean;
    max_context: number;
    thinking?: boolean; // Supports reasoning/thinking
  };
  enabled: boolean;
  processing_mode: "rainy_api" | "cowork" | "direct";
  thinkingLevel?: "minimal" | "low" | "medium" | "high"; // Available thinking levels
}

interface UnifiedModelSelectorProps {
  selectedModelId: string;
  onSelect: (modelId: string) => void;
  className?: string;
  filter?: "all" | "chat" | "processing";
}

export function UnifiedModelSelector({
  selectedModelId,
  onSelect,
  className,
  filter = "all",
}: UnifiedModelSelectorProps) {
  // const [loading, setLoading] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [models, setModels] = useState<UnifiedModel[]>([]);

  const [isPopoverOpen, setIsPopoverOpen] = useState(false);

  // Fetch models on mount
  useEffect(() => {
    loadModels();
  }, []);

  const loadModels = async () => {
    // setLoading(true); // Removed
    try {
      // Use the new unified command
      // @ts-ignore - Command might not be in types yet
      // const fetchedModels =
      //   await tauri.invoke<UnifiedModel[]>("get_unified_models");
      // setModels(fetchedModels || []);
      // Fetch unified models and preferences
      const [fetchedModels, userPrefs] = await Promise.all([
        tauri.getUnifiedModels(),
        tauri.getUserPreferences(),
      ]);
      setModels(fetchedModels || []);
      // setPreferences(userPrefs);
    } catch (err) {
      console.error("Failed to load unified models:", err);
      // Fallback/Mock for development if backend fails
      setModels(MOCK_MODELS);
    } finally {
      // setLoading(false); // Removed
    }
  };

  const selectedModel = useMemo(
    () => models.find((m) => m.id === selectedModelId),
    [models, selectedModelId],
  );

  const filteredModels = useMemo(() => {
    return models.filter((model) => {
      // Search filter
      if (
        searchQuery &&
        !model.name.toLowerCase().includes(searchQuery.toLowerCase()) &&
        !model.provider.toLowerCase().includes(searchQuery.toLowerCase())
      ) {
        return false;
      }

      // Type filter
      if (filter === "chat") {
        return model.processing_mode === "rainy_api";
      } else if (filter === "processing") {
        return model.processing_mode === "cowork";
      }

      return true;
    });
  }, [models, searchQuery, filter]);

  const groupedModels = useMemo(() => {
    const groups: Record<string, UnifiedModel[]> = {};
    filteredModels.forEach((model) => {
      const key = model.provider;
      if (!groups[key]) groups[key] = [];
      groups[key].push(model);
    });
    return groups;
  }, [filteredModels]);

  const getProviderIcon = (provider: string) => {
    switch (provider.toLowerCase()) {
      case "rainy":
      case "rainy_api":
        return <Zap className="size-3.5 text-yellow-500" />;
      case "cowork":
        return <Brain className="size-3.5 text-purple-500" />;
      case "openai":
        return <Sparkles className="size-3.5 text-green-500" />;
      case "anthropic":
        return <Cpu className="size-3.5 text-orange-500" />;
      case "xai":
        return <Globe className="size-3.5 text-blue-500" />;
      default:
        return <Zap className="size-3.5 text-muted-foreground" />;
    }
  };

  // Helper to check if model supports thinking/reasoning
  const supportsThinking = (modelId: string): boolean => {
    return modelId.includes("gemini-3") || modelId.includes("gemini-2.5");
  };

  // Get thinking level for a model
  const getThinkingLevel = (modelId: string): string | null => {
    if (modelId.includes("gemini-3-pro")) return "high";
    if (modelId.includes("gemini-3-flash")) return "medium";
    if (modelId.includes("gemini-2.5-pro")) return "high";
    if (modelId.includes("gemini-2.5-flash")) return "medium";
    return null;
  };

  return (
    <Popover isOpen={isPopoverOpen} onOpenChange={setIsPopoverOpen}>
      <PopoverTrigger>
        <Button
          variant="ghost"
          className={`h-auto py-1.5 px-3 gap-2 font-normal rounded-full transition-all duration-300
            shadow-sm hover:shadow-md
            ${
              selectedModel?.provider === "Cowork"
                ? "bg-purple-100/60 dark:bg-primary/30 border-primary/50 dark:border-primary/30 text-purple-900 dark:text-purple-100"
                : selectedModel?.provider === "Rainy API" ||
                    selectedModel?.provider === "Rainy"
                  ? "bg-amber-900/60 dark:bg-primary/30 border-primary/50 dark:border-primary/30 text-amber-900 dark:text-amber-100"
                  : selectedModel?.provider === "OpenAI"
                    ? "bg-green-100/60 dark:bg-primary/30 border-primary/50 dark:border-primary/30 text-green-900 dark:text-green-100"
                    : selectedModel?.provider === "Anthropic"
                      ? "bg-orange-100/60 dark:bg-primary/30 border-primary/50 dark:border-primary/30 text-orange-900 dark:text-orange-100"
                      : "bg-white/60 dark:bg-black/30 border-black/5 dark:border-white/5"
            }
            backdrop-blur-2xl
            ${className}`}
        >
          {selectedModel ? (
            <>
              <div className="flex items-center gap-2">
                <div
                  className={`size-5 rounded-full flex items-center justify-center ${
                    selectedModel.provider === "Cowork"
                      ? "bg-purple-500/10 text-purple-600"
                      : "bg-amber-500/10 text-amber-600"
                  }`}
                >
                  {getProviderIcon(selectedModel.provider)}
                </div>
                <div className="flex flex-col items-start">
                  <span className="text-xs font-medium leading-tight text-foreground/90">
                    {selectedModel.name}
                  </span>
                </div>
              </div>
            </>
          ) : (
            <span className="text-muted-foreground text-xs">
              Select model...
            </span>
          )}
          <ChevronDown className="size-3 text-muted-foreground/70" />
        </Button>
      </PopoverTrigger>

      <PopoverContent className="w-80 p-0 bg-background/30 backdrop-blur-2xl border border-white/10 shadow-2xl rounded-2xl overflow-hidden">
        <div className="flex flex-col">
          {/* Search */}
          <div className="p-3 border-b border-border/10">
            <div className="relative">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 size-4 text-muted-foreground" />
              <Input
                placeholder="Search models..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="bg-background/30 w-full pl-9 text-sm"
              />
            </div>
          </div>

          {/* Model List */}
          <div className="max-h-[300px] overflow-y-auto py-2">
            {Object.entries(groupedModels).map(([provider, providerModels]) => (
              <div key={provider} className="px-2 py-1">
                <div className="px-2 py-1.5 text-xs font-medium text-muted-foreground uppercase tracking-wide">
                  {provider}
                </div>
                {providerModels.map((model) => (
                  <button
                    key={model.id}
                    onClick={() => {
                      onSelect(model.id);
                      setIsPopoverOpen(false);
                    }}
                    className={`w-full flex items-center gap-3 px-2 py-2 rounded-lg text-left transition-colors ${
                      selectedModelId === model.id
                        ? "bg-accent"
                        : "hover:bg-accent/50"
                    }`}
                  >
                    <div
                      className={`size-8 rounded-lg flex items-center justify-center shrink-0 ${
                        model.processing_mode === "cowork"
                          ? "bg-purple-500/10"
                          : "bg-yellow-500/10"
                      }`}
                    >
                      {getProviderIcon(model.provider)}
                    </div>
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center justify-between">
                        <span className="text-sm font-medium truncate">
                          {model.name}
                        </span>
                        {model.processing_mode === "cowork" && (
                          <Sparkles className="size-3 text-purple-500" />
                        )}
                      </div>
                      <div className="flex items-center gap-2 mt-0.5">
                        <span className="text-[10px] text-muted-foreground truncate">
                          {(model.capabilities.max_context / 1000).toString()}k
                          context
                        </span>
                        {model.capabilities.web_search && (
                          <span className="flex items-center gap-0.5 text-[10px] text-blue-500 bg-blue-500/5 px-1 rounded">
                            <Globe className="size-2.5" /> web
                          </span>
                        )}
                        {supportsThinking(model.id) && (
                          <span className="flex items-center gap-0.5 text-[10px] text-amber-500 bg-amber-500/10 px-1 rounded font-medium">
                            <Brain className="size-2.5" />{" "}
                            {getThinkingLevel(model.id)}
                          </span>
                        )}
                      </div>
                    </div>
                    {selectedModelId === model.id && (
                      <Check className="size-4 shrink-0" />
                    )}
                  </button>
                ))}
              </div>
            ))}

            {filteredModels.length === 0 && (
              <div className="py-8 text-center text-muted-foreground">
                <Filter className="size-8 mx-auto opacity-20 mb-2" />
                <p className="text-xs">No models found</p>
              </div>
            )}
          </div>

          {/* Footer */}
          <div className="p-2 border-t border-border/10 bg-muted/10 text-[10px] text-center text-muted-foreground">
            {filter === "chat"
              ? "Showing fast chat models only"
              : filter === "processing"
                ? "Showing deep processing models only"
                : "Showing all available models"}
          </div>
        </div>
      </PopoverContent>
    </Popover>
  );
}

// Fallback data for development
// Models that actually exist in the Rainy SDK and are available via the API
const MOCK_MODELS: UnifiedModel[] = [
  // GEMINI 3 SERIES - Advanced reasoning models with thinking capabilities
  {
    id: "rainy:gemini-3-pro-preview",
    name: "Gemini 3 Pro (Preview)",
    provider: "Rainy API",
    capabilities: {
      chat: true,
      streaming: true,
      function_calling: true,
      vision: true,
      web_search: true,
      max_context: 2000000,
      thinking: true,
    },
    enabled: true,
    processing_mode: "rainy_api",
    thinkingLevel: "high",
  },
  {
    id: "rainy:gemini-3-flash-preview",
    name: "Gemini 3 Flash (Preview)",
    provider: "Rainy API",
    capabilities: {
      chat: true,
      streaming: true,
      function_calling: true,
      vision: true,
      web_search: true,
      max_context: 1000000,
      thinking: true,
    },
    enabled: true,
    processing_mode: "rainy_api",
    thinkingLevel: "medium",
  },
  {
    id: "rainy:gemini-3-pro-image-preview",
    name: "Gemini 3 Pro Image (Preview)",
    provider: "Rainy API",
    capabilities: {
      chat: true,
      streaming: true,
      function_calling: true,
      vision: true,
      web_search: true,
      max_context: 2000000,
      thinking: true,
    },
    enabled: true,
    processing_mode: "rainy_api",
    thinkingLevel: "high",
  },
  // GEMINI 2.5 SERIES - Stable models with thinking budget support
  {
    id: "rainy:gemini-2.5-pro",
    name: "Gemini 2.5 Pro",
    provider: "Rainy API",
    capabilities: {
      chat: true,
      streaming: true,
      function_calling: true,
      vision: true,
      web_search: true,
      max_context: 2000000,
      thinking: true,
    },
    enabled: true,
    processing_mode: "rainy_api",
    thinkingLevel: "high",
  },
  {
    id: "rainy:gemini-2.5-flash",
    name: "Gemini 2.5 Flash",
    provider: "Rainy API",
    capabilities: {
      chat: true,
      streaming: true,
      function_calling: true,
      vision: true,
      web_search: true,
      max_context: 1000000,
      thinking: true,
    },
    enabled: true,
    processing_mode: "rainy_api",
    thinkingLevel: "medium",
  },
  {
    id: "rainy:gemini-2.5-flash-lite",
    name: "Gemini 2.5 Flash Lite",
    provider: "Rainy API",
    capabilities: {
      chat: true,
      streaming: true,
      function_calling: true,
      vision: true,
      web_search: false,
      max_context: 1000000,
      thinking: false,
    },
    enabled: true,
    processing_mode: "rainy_api",
  },
  // GROQ MODELS - High-speed inference
  {
    id: "rainy:llama-3.1-8b-instant",
    name: "Llama 3.1 8B Instant (Groq)",
    provider: "Rainy API",
    capabilities: {
      chat: true,
      streaming: true,
      function_calling: true,
      vision: false,
      web_search: false,
      max_context: 128000,
      thinking: false,
    },
    enabled: true,
    processing_mode: "rainy_api",
  },
  {
    id: "rainy:llama-3.3-70b-versatile",
    name: "Llama 3.3 70B Versatile (Groq)",
    provider: "Rainy API",
    capabilities: {
      chat: true,
      streaming: true,
      function_calling: true,
      vision: false,
      web_search: false,
      max_context: 128000,
      thinking: false,
    },
    enabled: true,
    processing_mode: "rainy_api",
  },
  // KIMI K2 - Via Groq for high-speed inference
  {
    id: "rainy:moonshotai/kimi-k2-instruct-0905",
    name: "Kimi K2 (Groq)",
    provider: "Rainy API",
    capabilities: {
      chat: true,
      streaming: true,
      function_calling: true,
      vision: false,
      web_search: false,
      max_context: 256000,
      thinking: true,
    },
    enabled: true,
    processing_mode: "rainy_api",
    thinkingLevel: "high",
  },
  // CEREBRAS MODELS
  {
    id: "rainy:cerebras/llama3.1-8b",
    name: "Llama 3.1 8B (Cerebras)",
    provider: "Rainy API",
    capabilities: {
      chat: true,
      streaming: true,
      function_calling: true,
      vision: false,
      web_search: false,
      max_context: 128000,
      thinking: false,
    },
    enabled: true,
    processing_mode: "rainy_api",
  },
  // ENOSIS LABS MODELS - Proprietary models
  {
    id: "rainy:astronomer-2-pro",
    name: "Astronomer 2 Pro",
    provider: "Rainy API",
    capabilities: {
      chat: true,
      streaming: true,
      function_calling: true,
      vision: false,
      web_search: false,
      max_context: 128000,
      thinking: false,
    },
    enabled: true,
    processing_mode: "rainy_api",
  },
  {
    id: "rainy:astronomer-2",
    name: "Astronomer 2",
    provider: "Rainy API",
    capabilities: {
      chat: true,
      streaming: true,
      function_calling: true,
      vision: false,
      web_search: false,
      max_context: 128000,
      thinking: false,
    },
    enabled: true,
    processing_mode: "rainy_api",
  },
  {
    id: "rainy:astronomer-1-5",
    name: "Astronomer 1.5",
    provider: "Rainy API",
    capabilities: {
      chat: true,
      streaming: true,
      function_calling: true,
      vision: false,
      web_search: false,
      max_context: 128000,
      thinking: false,
    },
    enabled: true,
    processing_mode: "rainy_api",
  },
];
