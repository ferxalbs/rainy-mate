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

  const getProviderIcon = (model: UnifiedModel, className = "size-3.5") => {
    const normalizedProvider = model.provider.toLowerCase();
    const normalizedId = model.id.toLowerCase();
    const normalizedName = model.name.toLowerCase();

    // Helper for tinted SVG icons
    const RenderTintedIcon = ({
      src,
      colorClass,
    }: {
      src: string;
      colorClass: string;
    }) => (
      <div
        className={`${className} ${colorClass}`}
        style={{
          maskImage: `url(${src})`,
          WebkitMaskImage: `url(${src})`,
          maskSize: "contain",
          WebkitMaskSize: "contain",
          maskRepeat: "no-repeat",
          WebkitMaskRepeat: "no-repeat",
          maskPosition: "center",
          WebkitMaskPosition: "center",
          backgroundColor: "currentColor",
        }}
      />
    );

    if (
      normalizedProvider.includes("openai") ||
      normalizedId.includes("gpt") ||
      normalizedName.includes("gpt") ||
      normalizedId.includes("o1") ||
      normalizedId.includes("o3") ||
      normalizedId.includes("o4") ||
      normalizedId.includes("o5")
    ) {
      return (
        <RenderTintedIcon
          src="/openai.svg"
          colorClass="text-[#10a37f] dark:text-[#10a37f]"
        />
      );
    }

    if (
      normalizedProvider.includes("anthropic") ||
      normalizedId.includes("claude") ||
      normalizedName.includes("claude")
    ) {
      return (
        <RenderTintedIcon
          src="/antro.svg"
          colorClass="text-[#d97757] dark:text-[#cc785c]"
        />
      );
    }

    if (
      normalizedProvider.includes("google") ||
      normalizedProvider.includes("gemini") ||
      normalizedId.includes("gemini") ||
      normalizedName.includes("gemini")
    ) {
      return (
        <RenderTintedIcon
          src="/google.svg"
          colorClass="text-[#4285F4] dark:text-[#4285F4]"
        />
      );
    }

    if (
      normalizedProvider.includes("moonshot") ||
      normalizedId.includes("kimi") ||
      normalizedName.includes("kimi")
    ) {
      return (
        <RenderTintedIcon
          src="/moonshot.svg"
          colorClass="text-foreground dark:text-foreground"
        />
      );
    }

    if (
      normalizedProvider.includes("meta") ||
      normalizedId.includes("llama") ||
      normalizedName.includes("llama")
    ) {
      return (
        <RenderTintedIcon
          src="/meta.svg"
          colorClass="text-[#0668E1] dark:text-[#0668E1]"
        />
      );
    }

    if (
      normalizedProvider.includes("rainy") ||
      normalizedProvider === "rainy_api"
    ) {
      return <Zap className={`${className} text-yellow-500`} />;
    }

    if (normalizedProvider.includes("cowork")) {
      return <Brain className={`${className} text-purple-500`} />;
    }

    if (normalizedProvider.includes("xai") || normalizedId.includes("grok")) {
      return <Globe className={`${className} text-blue-500`} />;
    }

    return <Zap className={`${className} text-muted-foreground`} />;
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
            bg-white/50 dark:bg-black/20 
            border border-black/5 dark:border-white/5
            hover:bg-black/5 dark:hover:bg-white/5
            backdrop-blur-md
            ${className}`}
        >
          {selectedModel ? (
            <>
              <div className="flex items-center gap-2">
                <div className="flex items-center justify-center">
                  {getProviderIcon(selectedModel, "size-4")}
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

      <PopoverContent className="w-80 p-0 bg-background/80 backdrop-blur-3xl border border-white/10 shadow-2xl rounded-2xl overflow-hidden">
        <div className="flex flex-col">
          {/* Search */}
          <div className="p-3 border-b border-border/10">
            <div className="relative">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 size-4 text-muted-foreground" />
              <Input
                placeholder="Search models..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="bg-muted/30 w-full pl-9 text-sm h-9 rounded-lg border-transparent focus:border-primary/20 transition-all font-medium"
              />
            </div>
          </div>

          {/* Model List */}
          <div className="max-h-[300px] overflow-y-auto py-2 custom-scrollbar">
            {Object.entries(groupedModels).map(([provider, providerModels]) => (
              <div key={provider} className="px-2 py-1">
                <div className="px-2 py-1.5 text-[10px] font-bold text-muted-foreground/60 uppercase tracking-wider">
                  {provider}
                </div>
                {providerModels.map((model) => (
                  <button
                    key={model.id}
                    onClick={() => {
                      onSelect(model.id);
                      setIsPopoverOpen(false);
                    }}
                    className={`w-full flex items-center gap-3 px-2 py-2 rounded-lg text-left transition-all duration-200 group ${
                      selectedModelId === model.id
                        ? "bg-secondary/80 text-foreground ring-1 ring-border/50 shadow-sm"
                        : "hover:bg-muted/50 text-foreground/80 hover:text-foreground"
                    }`}
                  >
                    <div className="flex items-center justify-center shrink-0 w-5 h-5">
                      {getProviderIcon(model, "size-4")}
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
                        <span className="text-[10px] text-muted-foreground/80 truncate font-medium">
                          {(model.capabilities.max_context / 1000).toString()}k
                          context
                        </span>
                        {model.capabilities.web_search && (
                          <span className="flex items-center gap-0.5 text-[10px] text-blue-500 bg-blue-500/10 px-1.5 py-px rounded-md font-medium">
                            <Globe className="size-2.5" /> web
                          </span>
                        )}
                        {supportsThinking(model.id) && (
                          <span className="flex items-center gap-0.5 text-[10px] text-amber-500 bg-amber-500/10 px-1.5 py-px rounded-md font-medium">
                            <Brain className="size-2.5" />{" "}
                            {getThinkingLevel(model.id)}
                          </span>
                        )}
                      </div>
                    </div>
                    {selectedModelId === model.id && (
                      <Check className="size-3.5 shrink-0 text-primary" />
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
  // OPENAI MODELS
  {
    id: "openai:gpt-4o",
    name: "GPT-4o",
    provider: "OpenAI",
    capabilities: {
      chat: true,
      streaming: true,
      function_calling: true,
      vision: true,
      web_search: true,
      max_context: 128000,
      thinking: false,
    },
    enabled: true,
    processing_mode: "rainy_api",
  },
  {
    id: "openai:o1-preview",
    name: "o1 Preview",
    provider: "OpenAI",
    capabilities: {
      chat: true,
      streaming: true,
      function_calling: false,
      vision: false,
      web_search: false,
      max_context: 128000,
      thinking: true,
    },
    enabled: true,
    processing_mode: "rainy_api",
    thinkingLevel: "high",
  },
  // ANTHROPIC MODELS
  {
    id: "anthropic:claude-3-5-sonnet-latest",
    name: "Claude 3.5 Sonnet",
    provider: "Anthropic",
    capabilities: {
      chat: true,
      streaming: true,
      function_calling: true,
      vision: true,
      web_search: true,
      max_context: 200000,
      thinking: true,
    },
    enabled: true,
    processing_mode: "rainy_api",
    thinkingLevel: "high",
  },
  // GEMINI 3 SERIES - Advanced reasoning models with thinking capabilities
  {
    id: "gemini-3-pro-preview",
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
    id: "gemini-3-flash-preview",
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
    id: "gemini-3-pro-image-preview",
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
    id: "gemini-2.5-pro",
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
    id: "gemini-2.5-flash",
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
    id: "gemini-2.5-flash-lite",
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
    id: "llama-3.1-8b-instant",
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
    id: "llama-3.3-70b-versatile",
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
    id: "moonshotai/kimi-k2-instruct-0905",
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
    id: "cerebras/llama3.1-8b",
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
    id: "astronomer-2-pro",
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
    id: "astronomer-2",
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
    id: "astronomer-1-5",
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
