import { useMemo, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { Button, Input, ListBox, Select, Slider, Switch } from "@heroui/react";
import { toast } from "sonner";
import { Brain } from "lucide-react";
import type { MemoryConfig, KnowledgeFile } from "../../../types/memory";
import * as tauri from "../../../services/tauri";

interface MemoryPanelProps {
  agentId: string;
  memoryConfig: MemoryConfig;
  onChange: (memoryConfig: MemoryConfig) => void;
}

const sectionTitleClass =
  "text-[10px] font-bold uppercase tracking-widest text-muted-foreground";
const controlClass =
  "w-full bg-background/85 dark:bg-background/20 border-default-300/70 dark:border-white/15 data-[hover=true]:bg-background/90 dark:data-[hover=true]:bg-background/35 shadow-sm";
const softButtonClass =
  "bg-background/85 dark:bg-background/35 border border-default-300/70 dark:border-white/15 text-foreground data-[hover=true]:bg-background/90 dark:data-[hover=true]:bg-background/45";
const selectTriggerClass =
  "h-11 bg-background/85 dark:bg-background/20 border border-default-300/70 dark:border-white/15 rounded-xl text-foreground";

const selectionToValue = (selection: unknown): string | null => {
  if (typeof selection === "string") return selection;
  if (selection instanceof Set) {
    const first = selection.values().next().value;
    return typeof first === "string" ? first : null;
  }
  return null;
};

function upsertKnowledgeFile(
  files: KnowledgeFile[],
  next: KnowledgeFile,
): KnowledgeFile[] {
  const kept = files.filter((file) => file.id !== next.id && file.path !== next.path);
  return [next, ...kept].sort((a, b) => b.indexed_at - a.indexed_at);
}

export function MemoryPanel({ agentId, memoryConfig, onChange }: MemoryPanelProps) {
  const [isIndexing, setIsIndexing] = useState(false);
  const [query, setQuery] = useState("");
  const [isQuerying, setIsQuerying] = useState(false);
  const [results, setResults] = useState<tauri.AgentMemoryResult[]>([]);

  const indexedCount = useMemo(
    () => memoryConfig.knowledge.indexed_files.length,
    [memoryConfig.knowledge.indexed_files],
  );

  const updateRetrieval = (updates: Partial<MemoryConfig["retrieval"]>) => {
    onChange({
      ...memoryConfig,
      retrieval: {
        ...memoryConfig.retrieval,
        ...updates,
      },
    });
  };

  const updatePersistence = (updates: Partial<MemoryConfig["persistence"]>) => {
    onChange({
      ...memoryConfig,
      persistence: {
        ...memoryConfig.persistence,
        ...updates,
      },
    });
  };

  const handleIndexFile = async () => {
    try {
      const selected = await open({
        directory: false,
        multiple: false,
        filters: [
          {
            name: "Knowledge Files",
            extensions: ["md", "txt", "json", "csv", "yaml", "yml", "log"],
          },
        ],
      });

      if (!selected || Array.isArray(selected)) {
        return;
      }

      setIsIndexing(true);
      const indexed = await tauri.indexKnowledgeFile(agentId, selected);
      onChange({
        ...memoryConfig,
        knowledge: {
          enabled: true,
          indexed_files: upsertKnowledgeFile(
            memoryConfig.knowledge.indexed_files,
            indexed.file,
          ),
        },
      });
      toast.success(`Indexed ${indexed.file.name} (${indexed.chunks_indexed} chunks)`);
    } catch (error) {
      console.error("Failed to index knowledge file:", error);
      toast.error(`Indexing failed: ${error}`);
    } finally {
      setIsIndexing(false);
    }
  };

  const handleQuery = async () => {
    if (!query.trim()) {
      setResults([]);
      return;
    }
    try {
      setIsQuerying(true);
      const searchResults = await tauri.queryAgentMemory(
        agentId,
        query,
        memoryConfig.strategy,
        6,
      );
      setResults(searchResults);
    } catch (error) {
      console.error("Failed to query memory:", error);
      toast.error(`Memory query failed: ${error}`);
    } finally {
      setIsQuerying(false);
    }
  };

  return (
    <div className="space-y-8 animate-appear">
      <div className="relative overflow-hidden rounded-2xl border border-border/20 bg-card/40 backdrop-blur-xl p-5">
        <div className="absolute -top-20 right-[-60px] w-[280px] h-[280px] rounded-full bg-primary/10 blur-[85px] pointer-events-none" />
        <div className="absolute -bottom-24 left-[-80px] w-[260px] h-[260px] rounded-full bg-foreground/[0.04] blur-[90px] pointer-events-none" />
        <div className="relative z-10 flex flex-col gap-1">
          <h3 className="text-2xl font-bold text-foreground tracking-tight flex items-center gap-2">
            <Brain className="size-5 text-primary" />
            Memory
          </h3>
          <p className="text-muted-foreground text-sm">
            Configure retrieval and persist indexed knowledge for cross-session use.
          </p>
        </div>
      </div>

      <section className="grid grid-cols-1 md:grid-cols-2 gap-8 rounded-2xl border border-border/20 bg-card/35 backdrop-blur-md p-5">
        <div className="space-y-3">
          <label className={sectionTitleClass}>Retrieval Strategy</label>
          <Select
            className={`${controlClass} h-12`}
            selectedKey={memoryConfig.strategy}
            onSelectionChange={(selection) => {
              const value = selectionToValue(selection);
              if (!value) return;
              onChange({
                ...memoryConfig,
                strategy: value as "vector" | "simple_buffer" | "hybrid",
              });
            }}
          >
            <Select.Trigger className={selectTriggerClass}>
              <Select.Value className="text-foreground" />
              <Select.Indicator />
            </Select.Trigger>
            <Select.Popover className="bg-background/95 dark:bg-background/35 border border-default-200/70 dark:border-white/15 backdrop-blur-xl">
              <ListBox className="bg-transparent">
                <ListBox.Item id="hybrid" textValue="Hybrid">
                  Hybrid
                  <ListBox.ItemIndicator />
                </ListBox.Item>
                <ListBox.Item id="vector" textValue="Vector">
                  Vector
                  <ListBox.ItemIndicator />
                </ListBox.Item>
                <ListBox.Item id="simple_buffer" textValue="Simple Buffer">
                  Simple Buffer
                  <ListBox.ItemIndicator />
                </ListBox.Item>
              </ListBox>
            </Select.Popover>
          </Select>
        </div>

        <div className="space-y-6">
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <label className={sectionTitleClass}>Retention</label>
              <span className="font-mono text-xs text-foreground">
                {memoryConfig.retrieval.retention_days} days
              </span>
            </div>
            <Slider
              minValue={1}
              maxValue={90}
              step={1}
              value={memoryConfig.retrieval.retention_days}
              onChange={(value) =>
                updateRetrieval({
                  retention_days: Math.max(
                    1,
                    Array.isArray(value) ? Number(value[0] ?? 1) : Number(value),
                  ),
                })
              }
              className="max-w-full"
            >
              <Slider.Track className="h-1.5 bg-default-200 dark:bg-white/10 rounded-full">
                <Slider.Fill className="bg-primary h-full rounded-full" />
                <Slider.Thumb className="size-4 bg-background border-2 border-primary rounded-full shadow-md" />
              </Slider.Track>
            </Slider>
          </div>

          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <label className={sectionTitleClass}>Context Window</label>
              <span className="font-mono text-xs text-foreground">
                {memoryConfig.retrieval.max_tokens} tokens
              </span>
            </div>
            <Slider
              minValue={512}
              maxValue={32000}
              step={512}
              value={memoryConfig.retrieval.max_tokens}
              onChange={(value) =>
                updateRetrieval({
                  max_tokens: Math.max(
                    512,
                    Array.isArray(value) ? Number(value[0] ?? 512) : Number(value),
                  ),
                })
              }
              className="max-w-full"
            >
              <Slider.Track className="h-1.5 bg-default-200 dark:bg-white/10 rounded-full">
                <Slider.Fill className="bg-primary h-full rounded-full" />
                <Slider.Thumb className="size-4 bg-background border-2 border-primary rounded-full shadow-md" />
              </Slider.Track>
            </Slider>
          </div>
        </div>
      </section>

      <section className="space-y-4 rounded-2xl border border-border/20 bg-card/35 backdrop-blur-md p-5">
        <h4 className={sectionTitleClass}>Persistence</h4>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          <Switch
            isSelected={memoryConfig.persistence.cross_session}
            onChange={(cross_session) => updatePersistence({ cross_session })}
          >
            <Switch.Control>
              <Switch.Thumb />
            </Switch.Control>
            Cross-session
          </Switch>
          <Switch
            isSelected={memoryConfig.persistence.per_connector_isolation}
            onChange={(per_connector_isolation) =>
              updatePersistence({ per_connector_isolation })
            }
          >
            <Switch.Control>
              <Switch.Thumb />
            </Switch.Control>
            Per-connector isolation
          </Switch>
          <Select
            className={controlClass}
            selectedKey={memoryConfig.persistence.session_scope}
            onSelectionChange={(selection) => {
              const value = selectionToValue(selection);
              if (!value) return;
              updatePersistence({
                session_scope: value as "per_user" | "per_channel" | "global",
              });
            }}
          >
            <Select.Trigger className={selectTriggerClass}>
              <Select.Value className="text-foreground" />
              <Select.Indicator />
            </Select.Trigger>
            <Select.Popover className="bg-background/95 dark:bg-background/35 border border-default-200/70 dark:border-white/15 backdrop-blur-xl">
              <ListBox className="bg-transparent">
                <ListBox.Item id="per_user" textValue="Per User">
                  Per User
                  <ListBox.ItemIndicator />
                </ListBox.Item>
                <ListBox.Item id="per_channel" textValue="Per Channel">
                  Per Channel
                  <ListBox.ItemIndicator />
                </ListBox.Item>
                <ListBox.Item id="global" textValue="Global">
                  Global
                  <ListBox.ItemIndicator />
                </ListBox.Item>
              </ListBox>
            </Select.Popover>
          </Select>
        </div>
      </section>

      <section className="space-y-4 rounded-2xl border border-border/20 bg-card/35 backdrop-blur-md p-5">
        <div className="flex items-center justify-between">
          <h4 className={sectionTitleClass}>Knowledge Files</h4>
          <Button
            size="sm"
            variant="secondary"
            className={softButtonClass}
            onPress={handleIndexFile}
            isDisabled={isIndexing}
          >
            {isIndexing ? "Indexing..." : "+ Add knowledge file"}
          </Button>
        </div>

        <Switch
          isSelected={memoryConfig.knowledge.enabled}
          onChange={(enabled) =>
            onChange({
              ...memoryConfig,
              knowledge: {
                ...memoryConfig.knowledge,
                enabled,
              },
            })
          }
        >
          <Switch.Control>
            <Switch.Thumb />
          </Switch.Control>
          Enable knowledge injection ({indexedCount} indexed)
        </Switch>

        {indexedCount === 0 ? (
          <div className="p-4 rounded-xl border border-dashed border-border/30 text-sm text-muted-foreground bg-card/20">
            No knowledge files indexed yet.
          </div>
        ) : (
          <div className="space-y-2">
            {memoryConfig.knowledge.indexed_files.map((file) => (
              <div
                key={file.id}
                className="p-3 rounded-xl border border-border/20 bg-card/30 flex items-start justify-between gap-4"
              >
                <div className="min-w-0">
                  <p className="text-sm text-foreground truncate">{file.name}</p>
                  <p className="text-xs text-muted-foreground truncate">{file.path}</p>
                </div>
                <span className="text-xs font-mono text-muted-foreground shrink-0">
                  {file.chunk_count} chunks
                </span>
              </div>
            ))}
          </div>
        )}
      </section>

      <section className="space-y-3 rounded-2xl border border-border/20 bg-card/35 backdrop-blur-md p-5">
        <h4 className={sectionTitleClass}>Query Preview</h4>
        <div className="grid grid-cols-1 md:grid-cols-[1fr_auto] gap-2">
          <Input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Ask memory to retrieve relevant context"
            className={controlClass}
          />
          <Button
            variant="secondary"
            className={softButtonClass}
            onPress={handleQuery}
            isDisabled={isQuerying || !query.trim()}
          >
            {isQuerying ? "Searching..." : "Search"}
          </Button>
        </div>

        {results.length > 0 && (
          <div className="space-y-2">
            {results.map((result) => (
              <div key={result.id} className="p-3 rounded-xl border border-border/20 bg-card/30">
                <div className="flex items-center justify-between gap-3 mb-1">
                  <p className="text-xs font-semibold text-foreground/90 truncate">
                    {result.file_name}
                  </p>
                  <span className="text-[10px] font-mono text-primary">
                    score {result.score.toFixed(2)}
                  </span>
                </div>
                <p className="text-xs text-muted-foreground leading-relaxed line-clamp-4">
                  {result.content}
                </p>
              </div>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}
