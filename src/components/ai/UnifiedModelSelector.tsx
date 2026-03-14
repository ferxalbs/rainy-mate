import { useEffect, useMemo, useState } from "react";
import {
  Brain,
  Check,
  ChevronDown,
  Cpu,
  Filter,
  Globe,
  Search,
  Sparkles,
  Zap,
} from "lucide-react";

import * as tauri from "../../services/tauri";
import { Badge } from "../ui/badge";
import { Button } from "../ui/button";
import { Input } from "../ui/input";
import { Popover, PopoverContent, PopoverTrigger } from "../ui/popover";
import { cn } from "../../lib/utils";

export type UnifiedModel = tauri.UnifiedModel;

interface UnifiedModelSelectorProps {
  selectedModelId: string;
  onSelect: (modelId: string) => void;
  onModelResolved?: (model: UnifiedModel | null) => void;
  className?: string;
  filter?: "all" | "chat" | "processing";
}

const REASONING_LEVELS = ["minimal", "low", "medium", "high"] as const;

function getDisplayModelName(modelName: string): string {
  const name = modelName.trim();
  if (!/[-_]/.test(name)) return name;

  const suffixes = new Set(["minimal", "low", "medium", "high", "preview", "latest"]);
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
  if (maxContext >= 1_000_000) {
    const value = maxContext / 1_000_000;
    return Number.isInteger(value) ? `${value}M` : `${value.toFixed(1)}M`;
  }

  const value = maxContext / 1_000;
  return Number.isInteger(value) ? `${value}k` : `${value.toFixed(1)}k`;
}

export function getReasoningOptions(model: UnifiedModel | null): string[] {
  if (!model?.capabilities.reasoning) return [];

  const normalizedId = model.id.toLowerCase();
  if (normalizedId.includes("gemini-3-pro")) {
    return ["low", "high"];
  }

  if (normalizedId.includes("gpt-5") || /(^|:)o[134]/.test(normalizedId)) {
    return ["low", "medium", "high"];
  }

  if (normalizedId.includes("gemini-3")) {
    return [...REASONING_LEVELS];
  }

  return model.reasoning_level === "dynamic"
    ? ["low", "medium", "high"]
    : [...REASONING_LEVELS];
}

function getRecommendedReasoning(model: UnifiedModel | null): string | undefined {
  const options = getReasoningOptions(model);
  if (!options.length) return undefined;
  if (model?.reasoning_level && options.includes(model.reasoning_level)) {
    return model.reasoning_level;
  }
  return options.includes("medium") ? "medium" : options[0];
}

function getProviderIcon(model: UnifiedModel, className = "size-4") {
  const provider = model.provider.toLowerCase();
  const id = model.id.toLowerCase();

  if (provider.includes("google") || id.includes("gemini")) {
    return <Sparkles className={cn(className, "text-sky-500")} />;
  }
  if (provider.includes("openai") || id.includes("gpt") || /(^|:)o[134]/.test(id)) {
    return <Brain className={cn(className, "text-emerald-500")} />;
  }
  if (provider.includes("rainy")) {
    return <Zap className={cn(className, "text-amber-500")} />;
  }
  if (provider.includes("xai") || id.includes("grok")) {
    return <Globe className={cn(className, "text-cyan-500")} />;
  }
  return <Cpu className={cn(className, "text-muted-foreground")} />;
}

export function UnifiedModelSelector({
  selectedModelId,
  onSelect,
  onModelResolved,
  className,
  filter = "all",
}: UnifiedModelSelectorProps) {
  const [models, setModels] = useState<UnifiedModel[]>([]);
  const [searchQuery, setSearchQuery] = useState("");
  const [open, setOpen] = useState(false);

  useEffect(() => {
    void loadModels();
  }, []);

  useEffect(() => {
    if (open) {
      void loadModels();
    }
  }, [open]);

  const loadModels = async () => {
    try {
      const fetchedModels = await tauri.getUnifiedModels();
      setModels(fetchedModels || []);
    } catch (error) {
      console.error("Failed to load unified models:", error);
      setModels([]);
    }
  };

  const selectedModel = useMemo(
    () => models.find((model) => model.id === selectedModelId) ?? null,
    [models, selectedModelId],
  );

  useEffect(() => {
    onModelResolved?.(selectedModel);
  }, [onModelResolved, selectedModel]);

  const filteredModels = useMemo(() => {
    return models.filter((model) => {
      const displayName = getDisplayModelName(model.name).toLowerCase();
      const matchesSearch =
        !searchQuery ||
        model.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
        displayName.includes(searchQuery.toLowerCase()) ||
        model.provider.toLowerCase().includes(searchQuery.toLowerCase());

      if (!matchesSearch) return false;
      if (filter === "chat") return model.processing_mode === "rainy_api";
      if (filter === "processing") return model.processing_mode === "cowork";
      return true;
    });
  }, [filter, models, searchQuery]);

  const groupedModels = useMemo(() => {
    const groups = new Map<string, UnifiedModel[]>();
    for (const model of filteredModels) {
      const group = groups.get(model.provider) || [];
      group.push(model);
      groups.set(model.provider, group);
    }
    return Array.from(groups.entries());
  }, [filteredModels]);

  const triggerLabel = selectedModel ? getDisplayModelName(selectedModel.name) : "Select model";
  const triggerHint = selectedModel
    ? `${formatContextWindow(selectedModel.capabilities.max_context)} context`
    : "Runtime model";

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger>
        <Button
          variant="ghost"
          className={cn(
            "h-9 min-w-[10.75rem] justify-between rounded-lg border border-white/8 bg-background/70 px-3 py-2 text-left shadow-none backdrop-blur-sm backdrop-saturate-150 transition-all hover:bg-background/85 dark:bg-background/10 dark:hover:bg-background/16",
            className,
          )}
        >
          <span className="flex min-w-0 items-center gap-2.5">
            <span className="flex size-7 items-center justify-center rounded-md bg-foreground/6 text-foreground dark:bg-white/10">
              {selectedModel ? getProviderIcon(selectedModel) : <Sparkles className="size-4 text-muted-foreground" />}
            </span>
            <span className="flex min-w-0 flex-col">
              <span className="truncate text-sm font-medium text-foreground">{triggerLabel}</span>
              <span className="truncate text-[10px] text-muted-foreground">{triggerHint}</span>
            </span>
          </span>
          <ChevronDown className="size-3.5 text-muted-foreground" />
        </Button>
      </PopoverTrigger>

      <PopoverContent
        align="start"
        sideOffset={8}
        className="w-[21rem] gap-0 overflow-hidden rounded-lg border border-white/10 bg-background/90 p-0 shadow-[0_16px_48px_rgba(0,0,0,0.16)] backdrop-blur-xl backdrop-saturate-150 dark:bg-background/20"
      >
        <div className="border-b border-border/30 px-3 py-3">
          <div className="relative">
            <Search className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
            <Input
              placeholder="Search models"
              value={searchQuery}
              onChange={(event) => setSearchQuery(event.target.value)}
              className="h-9 rounded-lg border-white/8 bg-background/70 pl-10 backdrop-blur-sm dark:bg-background/20"
            />
          </div>
        </div>

        <div className="max-h-[22rem] overflow-y-auto px-2 py-2">
          {groupedModels.map(([provider, providerModels]) => (
            <div key={provider} className="mb-3 last:mb-0">
              <div className="px-2 pb-2 text-[10px] font-semibold uppercase tracking-[0.14em] text-muted-foreground/80">
                {provider}
              </div>
              <div className="space-y-1">
                {providerModels.map((model) => {
                  const active = selectedModelId === model.id;
                  const reasoningOptions = getReasoningOptions(model);
                  const recommendedReasoning = getRecommendedReasoning(model);

                  return (
                    <button
                      key={model.id}
                      type="button"
                      onClick={() => {
                        onSelect(model.id);
                        setOpen(false);
                      }}
                      className={cn(
                        "flex w-full items-start gap-3 rounded-lg border px-3 py-2.5 text-left transition-all",
                        active
                          ? "border-primary/20 bg-primary/10"
                          : "border-transparent hover:border-white/10 hover:bg-foreground/4",
                      )}
                    >
                      <span className="mt-0.5 flex size-8 shrink-0 items-center justify-center rounded-md bg-background/90 dark:bg-background/20">
                        {getProviderIcon(model, "size-4")}
                      </span>

                      <span className="min-w-0 flex-1">
                        <span className="flex items-center justify-between gap-3">
                          <span className="truncate text-sm font-medium text-foreground">
                            {getDisplayModelName(model.name)}
                          </span>
                          {active && <Check className="size-4 shrink-0 text-primary" />}
                        </span>

                        <span className="mt-1.5 flex flex-wrap items-center gap-1.5">
                          <Badge variant="outline" className="rounded-md border-white/10 bg-background/70 px-2 py-0.5 text-[10px] backdrop-blur-sm dark:bg-background/20">
                            {formatContextWindow(model.capabilities.max_context)} context
                          </Badge>
                          {model.capabilities.web_search && (
                            <Badge variant="outline" className="rounded-md border-sky-500/20 bg-sky-500/10 px-2 py-0.5 text-[10px] text-sky-600 dark:text-sky-300">
                              <Globe className="size-3" />
                              web
                            </Badge>
                          )}
                          {model.capabilities.reasoning && (
                            <Badge variant="outline" className="rounded-md border-amber-500/20 bg-amber-500/10 px-2 py-0.5 text-[10px] text-amber-700 dark:text-amber-300">
                              <Brain className="size-3" />
                              {recommendedReasoning || "reasoning"}
                            </Badge>
                          )}
                        </span>

                        <span className="mt-1.5 block text-[11px] text-muted-foreground">
                          {model.provider}
                          {reasoningOptions.length > 0 && ` · ${reasoningOptions.join(" / ")}`}
                        </span>
                      </span>
                    </button>
                  );
                })}
              </div>
            </div>
          ))}

          {filteredModels.length === 0 && (
            <div className="flex flex-col items-center justify-center gap-2 px-6 py-10 text-center text-muted-foreground">
              <Filter className="size-7 opacity-35" />
              <p className="text-sm">No models matched this search.</p>
            </div>
          )}
        </div>
      </PopoverContent>
    </Popover>
  );
}
