import { useEffect, useMemo, useState } from "react";
import {
  Check,
  ChevronDown,
} from "lucide-react";

import * as tauri from "../../services/tauri";
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

export function getReasoningOptions(model: UnifiedModel | null): string[] {
  if (!model?.capabilities.reasoning) return [];
  return model.capabilities.reasoning_options ?? [];
}



export function UnifiedModelSelector({
  selectedModelId,
  onSelect,
  onModelResolved,
  className,
  filter = "all",
}: UnifiedModelSelectorProps) {
  const [models, setModels] = useState<UnifiedModel[]>([]);
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
      if (filter === "chat") return model.processing_mode === "rainy_api";
      if (filter === "processing") return model.processing_mode === "cowork";
      return true;
    });
  }, [filter, models]);

  const triggerLabel = selectedModel ? getDisplayModelName(selectedModel.name) : "Select model";


  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger
        render={
          <button
            type="button"
            className={cn(
              "group flex items-center gap-1.5 rounded-md px-1.5 py-1 text-xs font-medium text-muted-foreground transition-colors hover:text-foreground",
              className,
            )}
          />
        }
      >
        <span className="truncate">{triggerLabel}</span>
        <ChevronDown className="size-3 opacity-50 transition-transform group-data-[state=open]:rotate-180" />
      </PopoverTrigger>

      <PopoverContent
        align="start"
        sideOffset={12}
        className="w-[240px] overflow-hidden rounded-xl border border-white/10 bg-background/20 p-1 shadow-2xl backdrop-blur-md"
      >
        <div className="flex flex-col">
          <div className="px-3 pb-1.5 pt-2 text-[10px] font-bold uppercase tracking-wider text-muted-foreground/40">
            Select model
          </div>
          
          <div className="max-h-[300px] overflow-y-auto custom-scrollbar">
            {filteredModels.map((model) => {
              const active = selectedModelId === model.id;
              return (
                <button
                  key={model.id}
                  type="button"
                  onClick={() => {
                    onSelect(model.id);
                    setOpen(false);
                  }}
                  className={cn(
                    "flex w-full items-center justify-between gap-3 rounded-lg px-3 py-2 text-left text-xs transition-colors",
                    active
                      ? "bg-white/10 text-foreground"
                      : "text-muted-foreground hover:bg-white/5 hover:text-foreground",
                  )}
                >
                  <span className="truncate">{getDisplayModelName(model.name)}</span>
                  {active && <Check className="size-3.5 shrink-0" />}
                </button>
              );
            })}
          </div>
        </div>
      </PopoverContent>
    </Popover>
  );
}
