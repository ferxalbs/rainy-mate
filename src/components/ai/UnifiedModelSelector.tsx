import { useState, useEffect, useMemo } from "react";
import {
  Button,
  Popover,
  PopoverTrigger,
  PopoverContent,
  Chip,
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

      // Context filter
      if (filter === "chat" && model.processing_mode === "cowork") return false;
      if (filter === "processing" && model.processing_mode === "rainy_api")
        return false;

      return true;
    });
  }, [models, searchQuery, filter]);

  // Group models by provider
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

  const getProcessingModeBadge = (mode: string) => {
    switch (mode) {
      case "rainy_api":
        return (
          <span className="text-[10px] bg-yellow-500/10 text-yellow-600 px-1.5 py-0.5 rounded font-medium ml-2">
            FAST
          </span>
        );
      case "cowork":
        return (
          <span className="text-[10px] bg-purple-500/10 text-purple-600 px-1.5 py-0.5 rounded font-medium ml-2">
            DEEP
          </span>
        );
      default:
        return null;
    }
  };

  // Helper to check if model supports thinking/reasoning
  const supportsThinking = (modelId: string): boolean => {
    return modelId.includes("gemini-3") || 
           modelId.includes("gemini-2.5") ||
           modelId.includes("claude") ||
           modelId.includes("kimi");
  };

  // Get thinking level for a model
  const getThinkingLevel = (modelId: string): string | null => {
    if (modelId.includes("gemini-3-pro")) return "high";
    if (modelId.includes("gemini-3-flash")) return "medium";
    if (modelId.includes("gemini-2.5-pro")) return "high";
    if (modelId.includes("gemini-2.5-flash")) return "medium";
    if (modelId.includes("claude")) return "high";
    if (modelId.includes("kimi")) return "high";
    return null;
  };

  return (
    <Popover isOpen={isPopoverOpen} onOpenChange={setIsPopoverOpen}>
      <PopoverTrigger>
        <Button
          variant="ghost"
          size="sm"
          className={`h-8 gap-2 bg-background/50 hover:bg-background/80 border border-border/40 rounded-full px-3 transition-all ${className}`}
        >
          {selectedModel ? (
            <>
              <div className="flex items-center justify-center size-5 rounded-full bg-secondary/50">
                {getProviderIcon(selectedModel.provider)}
              </div>
              <span className="text-sm font-medium truncate max-w-[150px]">
                {selectedModel.name}
              </span>
              {getProcessingModeBadge(selectedModel.processing_mode)}
            </>
          ) : (
            <>
              <Sparkles className="size-4 text-muted-foreground" />
              <span className="text-sm text-muted-foreground">
                Select Model
              </span>
            </>
          )}
          <ChevronDown className="size-3 text-muted-foreground opacity-50 ml-1" />
        </Button>
      </PopoverTrigger>
      <PopoverContent
        placement="bottom"
        className="p-0 w-[320px] overflow-hidden rounded-xl border border-border/50 shadow-xl bg-background/95 backdrop-blur-xl"
      >
        <div className="flex flex-col max-h-[400px]">
          {/* Header & Search */}
          <div className="p-3 border-b border-border/10 space-y-2 bg-muted/20">
            <div className="flex items-center justify-between px-1">
              <span className="text-xs font-semibold text-muted-foreground uppercase tracking-wider">
                Select AI Model
              </span>
              <div className="flex gap-1">
                <Chip size="sm" variant="secondary" className="h-5 text-[10px]">
                  {filteredModels.length.toString()} available
                </Chip>
              </div>
            </div>
            <div className="relative">
              <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 size-3.5 text-muted-foreground" />
              <Input
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                placeholder="Search models..."
                className="w-full h-8 pl-8 text-xs bg-background/50 border-border/20 focus:ring-1 focus:ring-primary/20 rounded-lg"
                // variant removed to use default or check docs later
              />
            </div>
          </div>

          {/* Models List */}
          <div className="overflow-y-auto flex-1 p-2 space-y-4 min-h-[200px] scrollbar-thin">
            {Object.entries(groupedModels).map(([provider, providerModels]) => (
              <div key={provider} className="space-y-1">
                <div className="px-2 text-[10px] font-bold text-muted-foreground/60 uppercase tracking-wider flex items-center gap-1.5 mb-1.5">
                  {getProviderIcon(provider)}
                  {provider.replace("_", " ")}
                </div>
                {providerModels.map((model) => (
                  <button
                    key={model.id}
                    onClick={() => {
                      onSelect(model.id);
                      setIsPopoverOpen(false);
                    }}
                    className={`w-full flex items-center gap-3 px-2 py-2 rounded-lg text-left transition-all ${
                      selectedModelId === model.id
                        ? "bg-primary/10 text-primary"
                        : "hover:bg-muted/50 text-foreground"
                    }`}
                  >
                    <div
                      className={`size-8 rounded-lg flex items-center justify-center shrink-0 border ${
                        selectedModelId === model.id
                          ? "bg-primary/20 border-primary/20"
                          : "bg-background border-border/30"
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
                          {model.capabilities.max_context / 1000}k context
                        </span>
                        {model.capabilities.web_search && (
                          <span className="flex items-center gap-0.5 text-[10px] text-blue-500 bg-blue-500/5 px-1 rounded">
                            <Globe className="size-2.5" /> web
                          </span>
                        )}
                        {supportsThinking(model.id) && (
                          <span className="flex items-center gap-0.5 text-[10px] text-amber-500 bg-amber-500/10 px-1 rounded font-medium">
                            <Brain className="size-2.5" /> {getThinkingLevel(model.id)}
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
// Enterprise-grade model selection with Gemini 3, Kimi K2, and advanced reasoning
const MOCK_MODELS: UnifiedModel[] = [
  // GEMINI 3 SERIES - Latest with thinking capabilities via Rainy API
  {
    id: "rainy:gemini-3-pro-preview",
    name: "Gemini 3 Pro (Preview)",
    provider: "rainy",
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
    provider: "rainy",
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
  // KIMI K2 SERIES - Via Groq for high-speed inference
  {
    id: "rainy:kimi-k2-0905",
    name: "Kimi K2 (Groq)",
    provider: "rainy",
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
  // GEMINI 2.5 SERIES - Thinking budget support
  {
    id: "cowork:gemini-2.5-pro",
    name: "Gemini 2.5 Pro",
    provider: "cowork",
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
    processing_mode: "cowork",
    thinkingLevel: "high",
  },
  {
    id: "cowork:gemini-2.5-flash",
    name: "Gemini 2.5 Flash",
    provider: "cowork",
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
    processing_mode: "cowork",
    thinkingLevel: "medium",
  },
  // CLAUDE SERIES
  {
    id: "cowork:claude-3-5-sonnet",
    name: "Claude 3.5 Sonnet",
    provider: "cowork",
    capabilities: {
      chat: true,
      streaming: true,
      function_calling: true,
      vision: true,
      web_search: false,
      max_context: 200000,
      thinking: true,
    },
    enabled: true,
    processing_mode: "cowork",
    thinkingLevel: "high",
  },
  // OPENAI SERIES - Fallback options
  {
    id: "rainy:gpt-4o",
    name: "GPT-4o",
    provider: "rainy",
    capabilities: {
      chat: true,
      streaming: true,
      function_calling: true,
      vision: true,
      web_search: false,
      max_context: 128000,
    },
    enabled: true,
    processing_mode: "rainy_api",
  },
  {
    id: "rainy:gpt-4o-mini",
    name: "GPT-4o Mini",
    provider: "rainy",
    capabilities: {
      chat: true,
      streaming: true,
      function_calling: true,
      vision: false,
      web_search: false,
      max_context: 128000,
    },
    enabled: true,
    processing_mode: "rainy_api",
  },
];
