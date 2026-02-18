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
  Cpu,
} from "lucide-react";
import * as tauri from "../../services/tauri";

// Define UnifiedModel interface based on backend struct
export interface UnifiedModel {
  id: string;
  name: string;
  provider: string; // rainy, cowork, openai, anthropic, xai, openrouter, local
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
      setModels([]);
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
      const displayName = getDisplayModelName(model.name).toLowerCase();
      // Hide OpenAI, Claude, and Astronomer models as requested
      // Variables previously used for filtering removed

      // Filter logic removed to allow all available providers

      // Search filter
      if (
        searchQuery &&
        !model.name.toLowerCase().includes(searchQuery.toLowerCase()) &&
        !displayName.includes(searchQuery.toLowerCase()) &&
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

    if (normalizedProvider.includes("openai") || normalizedId.includes("gpt")) {
      return (
        <RenderTintedIcon
          src="/openai.svg"
          colorClass="text-[#10a37f] dark:text-[#10a37f]"
        />
      );
    }

    if (
      normalizedProvider.includes("anthropic") ||
      normalizedId.includes("claude")
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
      normalizedProvider.includes("gemini")
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
      normalizedId.includes("kimi")
    ) {
      return (
        <RenderTintedIcon
          src="/moonshot.svg"
          colorClass="text-foreground dark:text-foreground"
        />
      );
    }

    if (normalizedProvider.includes("meta") || normalizedId.includes("llama")) {
      return (
        <RenderTintedIcon
          src="/meta.svg"
          colorClass="text-[#0668E1] dark:text-[#0668E1]"
        />
      );
    }

    if (normalizedProvider.includes("mistral")) {
      return <Sparkles className={`${className} text-orange-500`} />;
    }

    if (normalizedProvider.includes("perplexity")) {
      return <Search className={`${className} text-teal-500`} />;
    }

    if (normalizedProvider.includes("nvidia")) {
      return <Cpu className={`${className} text-green-500`} />;
    }

    if (normalizedProvider.includes("cerebras")) {
      return <Zap className={`${className} text-blue-600`} />;
    }

    if (normalizedProvider.includes("openrouter")) {
      return <Sparkles className={`${className} text-primary`} />;
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

    // Generic fallback based on name hash/char to give some variety or just generic icon
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

  // Normalize slug-like model names for display (e.g. gemini-3-pro-high -> Gemini 3 Pro (High))
  function getDisplayModelName(modelName: string): string {
    const name = modelName.trim();
    if (!/[-_]/.test(name)) return name;

    const suffixes = new Set([
      "minimal",
      "low",
      "medium",
      "high",
      "preview",
      "latest",
    ]);
    const tokens = name
      .split(/[-_]+/)
      .map((token) => token.trim())
      .filter(Boolean);

    if (!tokens.length) return name;

    let suffix: string | null = null;
    const lastToken = tokens[tokens.length - 1]?.toLowerCase();
    if (lastToken && suffixes.has(lastToken) && tokens.length > 1) {
      suffix = tokens.pop() || null;
    }

    const formattedBase = tokens
      .map((token) => {
        if (/^\d+(\.\d+)?$/.test(token)) return token;
        return token.charAt(0).toUpperCase() + token.slice(1).toLowerCase();
      })
      .join(" ");

    if (!formattedBase) return name;

    if (!suffix) return formattedBase;
    return `${formattedBase} (${suffix.charAt(0).toUpperCase()}${suffix.slice(1).toLowerCase()})`;
  }

  function formatContextWindow(maxContext: number): string {
    if (maxContext >= 1000000) {
      const inMillions = maxContext / 1000000;
      return Number.isInteger(inMillions)
        ? `${inMillions}M`
        : `${inMillions.toFixed(1)}M`;
    }

    const inThousands = maxContext / 1000;
    return Number.isInteger(inThousands)
      ? `${inThousands}k`
      : `${inThousands.toFixed(1)}k`;
  }

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
                    {getDisplayModelName(selectedModel.name)}
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

      <PopoverContent className="w-80 mt-2 p-0 bg-background/60 backdrop-blur-2xl dark:bg-background/20 border border-white/10 rounded-lg overflow-hidden">
        <div className="flex flex-col">
          {/* Search */}
          <div className="p-3 border-b border-border/10">
            <div className="relative">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 size-4" />
              <Input
                placeholder="Search models..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="bg-background/30 w-full pl-9 h-9 rounded-lg border-transparent focus:border-primary/20 transition-all text-white/50 dark:text-white/50"
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
                          {getDisplayModelName(model.name)}
                        </span>
                        {model.processing_mode === "cowork" && (
                          <Sparkles className="size-3 text-purple-500" />
                        )}
                      </div>
                      <div className="flex items-center gap-2 mt-0.5">
                        <span className="text-[10px] text-muted-foreground/80 truncate font-medium">
                          {formatContextWindow(model.capabilities.max_context)}{" "}
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
