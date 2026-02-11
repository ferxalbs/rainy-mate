import { useCallback, useEffect, useMemo, useState } from "react";
// @ts-ignore
import { Button, Spinner } from "@heroui/react";
import { toast } from "sonner";
import { AgentSpec } from "../../../types/agent-spec";
import * as tauri from "../../../services/tauri";
import { useTheme } from "../../../hooks/useTheme";
import { normalizeAgentSpec } from "../builder/specDefaults";
import {
  Bot,
  RefreshCw,
  Rocket,
  Pencil,
  Plus,
  Save,
  Search,
} from "lucide-react";

type StoreTab = "review" | "edit" | "json";

interface AgentStorePageProps {
  onCreateAgent: () => void;
  onEditInBuilder: (spec: AgentSpec) => void;
}

function cloneSpec(spec: AgentSpec): AgentSpec {
  return JSON.parse(JSON.stringify(spec)) as AgentSpec;
}

// Reusable Raw HTML Input
const RawInput = ({
  value,
  onChange,
  placeholder,
  className = "",
  type = "text",
}: {
  value: string;
  onChange: (e: React.ChangeEvent<HTMLInputElement>) => void;
  placeholder?: string;
  className?: string;
  type?: string;
}) => (
  <input
    type={type}
    value={value}
    onChange={onChange}
    placeholder={placeholder}
    className={`w-full bg-transparent border-b border-border/40 text-sm py-2 px-0 focus:outline-none focus:border-primary transition-colors placeholder:text-muted-foreground/30 ${className}`}
  />
);

// Reusable Raw HTML TextArea
const RawTextArea = ({
  value,
  onChange,
  placeholder,
  className = "",
  rows = 3,
  readOnly = false,
}: {
  value: string;
  onChange?: (e: React.ChangeEvent<HTMLTextAreaElement>) => void;
  placeholder?: string;
  className?: string;
  rows?: number;
  readOnly?: boolean;
}) => (
  <textarea
    value={value}
    onChange={onChange}
    placeholder={placeholder}
    rows={rows}
    readOnly={readOnly}
    className={`w-full bg-card/40 hover:bg-card/60 backdrop-blur-md rounded-xl px-4 py-3 text-sm text-foreground placeholder:text-muted-foreground/50 leading-relaxed border border-border/20 focus:outline-none focus:border-primary/50 focus:ring-1 focus:ring-primary/20 transition-all resize-none shadow-sm ${className}`}
  />
);

export function AgentStorePage({
  onCreateAgent,
  onEditInBuilder,
}: AgentStorePageProps) {
  const [agents, setAgents] = useState<AgentSpec[]>([]);
  const [selectedId, setSelectedId] = useState<string>("");
  const [draft, setDraft] = useState<AgentSpec | null>(null);
  const [search, setSearch] = useState("");
  const [activeTab, setActiveTab] = useState<StoreTab>("review");
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [isDeploying, setIsDeploying] = useState(false);
  const { mode } = useTheme();
  const isDark = mode === "dark";

  const loadAgents = useCallback(async () => {
    setIsLoading(true);
    try {
      const specs = ((await tauri.listAgentSpecs()) as any[]).map((spec) =>
        normalizeAgentSpec(spec),
      );
      setAgents(specs);
      if (specs.length > 0 && !selectedId) {
        setSelectedId(specs[0].id);
      } else if (specs.length === 0) {
        setSelectedId("");
        setDraft(null);
      }
    } catch (error) {
      console.error("Failed to load agents:", error);
      toast.error("Failed to load saved agents");
    } finally {
      setIsLoading(false);
    }
  }, [selectedId]);

  useEffect(() => {
    loadAgents();
  }, [loadAgents]);

  useEffect(() => {
    if (!selectedId) {
      setDraft(null);
      return;
    }
    const selected = agents.find((agent) => agent.id === selectedId);
    setDraft(selected ? cloneSpec(selected) : null);
  }, [agents, selectedId]);

  const filteredAgents = useMemo(() => {
    const query = search.trim().toLowerCase();
    if (!query) return agents;
    return agents.filter((agent) => {
      const name = agent.soul.name.toLowerCase();
      const description = agent.soul.description.toLowerCase();
      return name.includes(query) || description.includes(query);
    });
  }, [agents, search]);

  const selectedOriginal = useMemo(
    () => agents.find((agent) => agent.id === selectedId) ?? null,
    [agents, selectedId],
  );

  const isDirty = useMemo(() => {
    if (!selectedOriginal || !draft) return false;
    return JSON.stringify(selectedOriginal) !== JSON.stringify(draft);
  }, [selectedOriginal, draft]);

  const setMemoryNumber = (
    field: "retention_days" | "max_tokens",
    value: string,
  ) => {
    if (!draft) return;
    const parsed = Number.parseInt(value || "0", 10);
    const nextValue =
      field === "retention_days"
        ? Math.max(1, parsed || 1)
        : Math.max(512, parsed || 512);
    setDraft({
      ...draft,
      memory_config: {
        ...draft.memory_config,
        retrieval: {
          ...draft.memory_config.retrieval,
          [field]: nextValue,
        },
      },
    });
  };

  const handleSave = async () => {
    if (!draft) return;
    if (!draft.soul.name.trim()) {
      toast.error("Agent name is required");
      return;
    }

    setIsSaving(true);
    try {
      await tauri.saveAgentSpec(draft);
      toast.success("Agent updated");
      await loadAgents();
    } catch (error) {
      console.error("Failed to save agent:", error);
      toast.error("Failed to save agent");
    } finally {
      setIsSaving(false);
    }
  };

  const handleDeploy = async () => {
    if (!draft) return;
    setIsDeploying(true);
    try {
      const hasCredentials = await tauri.ensureAtmCredentialsLoaded();
      if (!hasCredentials) {
        throw new Error(
          "Rainy-ATM is not authenticated. Configure ATM credentials first.",
        );
      }
      const result = await tauri.deployAgentSpec(draft);
      const action =
        result && typeof result === "object" && "action" in result
          ? String((result as { action?: unknown }).action || "")
          : "";
      toast.success(
        action === "updated"
          ? "Agent updated in Rainy-ATM"
          : "Agent deployed to Rainy-ATM",
      );
    } catch (error) {
      console.error("Failed to deploy agent:", error);
      toast.error(`Deploy failed: ${error}`);
    } finally {
      setIsDeploying(false);
    }
  };

  return (
    <div className="h-full w-full bg-background p-3 flex gap-3 overflow-hidden font-sans selection:bg-primary selection:text-primary-foreground relative">
      {/* Draggable Background Layer */}
      <div
        className="absolute inset-0 w-full h-full z-0"
        data-tauri-drag-region
      />

      {/* LEFT PANEL: List & Search */}
      <aside
        className={`w-[260px] shrink-0 rounded-[1.5rem] border border-border/40 flex flex-col shadow-xl overflow-hidden relative z-10 ${isDark ? "bg-card/20" : "bg-card/60"} backdrop-blur-2xl`}
      >
        {/* Header - Explicitly Draggable */}
        <div className="p-6 pb-2" data-tauri-drag-region>
          <div className="flex items-start justify-between mb-4">
            <h1 className="text-xl font-bold text-foreground tracking-tight leading-tight pointer-events-none">
              Agent
              <br />
              Store
            </h1>
            <div className="flex items-center justify-center bg-primary/10 text-primary font-bold text-[10px] px-2 py-0.5 rounded-full mt-1">
              {agents.length}
            </div>
          </div>

          {/* Search */}
          <div className="relative group mb-2">
            <Search className="absolute left-0 top-1/2 -translate-y-1/2 ml-0 size-3.5 text-muted-foreground group-focus-within:text-primary transition-colors" />
            <input
              type="text"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder="Search..."
              className="w-full bg-transparent border-b border-border/40 text-xs py-1.5 pl-5 pr-2 focus:outline-none focus:border-primary transition-colors placeholder:text-muted-foreground/40"
            />
          </div>
        </div>

        {/* Toolbar */}
        <div className="px-3 py-2 space-y-1 relative z-20">
          <Button
            variant="ghost"
            className="w-full justify-start text-muted-foreground hover:text-foreground h-9 px-3"
            onPress={loadAgents}
            isDisabled={isLoading}
          >
            <RefreshCw
              className={`size-4 mr-3 ${isLoading ? "animate-spin" : ""}`}
            />
            <span className="text-sm font-medium">Refresh List</span>
          </Button>

          <Button
            variant="ghost"
            className="w-full justify-start text-muted-foreground hover:text-foreground h-9 px-3"
            onPress={onCreateAgent}
          >
            <Plus className="size-4 mr-3" />
            <span className="text-sm font-medium">New Agent</span>
          </Button>
        </div>

        {/* Agent List */}
        <div className="flex-1 overflow-y-auto px-3 py-2 space-y-1 relative z-20">
          {isLoading ? (
            <div className="py-12 flex justify-center">
              <Spinner size="md" color="current" className="text-primary" />
            </div>
          ) : filteredAgents.length === 0 ? (
            <div className="py-12 px-4 text-center text-muted-foreground text-xs">
              No agents found.
            </div>
          ) : (
            filteredAgents.map((agent) => {
              const isSelected = selectedId === agent.id;
              return (
                <button
                  key={agent.id}
                  onClick={() => setSelectedId(agent.id)}
                  className={`w-full text-left px-4 py-3 rounded-2xl transition-all duration-300 group relative overflow-hidden flex items-center gap-3 ${
                    isSelected
                      ? "bg-primary text-primary-foreground shadow-md shadow-primary/10"
                      : "hover:bg-foreground/5 text-muted-foreground hover:text-foreground"
                  }`}
                >
                  {/* Background Selection Indicator (Builder Style) */}
                  <div className="flex items-center gap-3 relative z-10 w-full">
                    <div
                      className={`p-1.5 rounded-full ${
                        isSelected
                          ? "bg-black/10"
                          : "bg-white/5 group-hover:bg-white/10"
                      }`}
                    >
                      <Bot className="size-4" />
                    </div>
                    <div className="min-w-0 flex-1">
                      <span
                        className={`block text-sm font-bold truncate ${isSelected ? "text-primary-foreground" : "text-foreground"}`}
                      >
                        {agent.soul.name || "Untitled"}
                      </span>
                      <span
                        className={`text-[10px] block truncate uppercase tracking-wider ${isSelected ? "text-primary-foreground/70" : "text-muted-foreground"}`}
                      >
                        v{agent.version}
                      </span>
                    </div>
                  </div>
                </button>
              );
            })
          )}
        </div>
      </aside>

      {/* RIGHT PANEL: Details & Editor */}
      <main
        className={`flex-1 rounded-[1.5rem] border border-border/40 shadow-xl flex flex-col overflow-hidden relative z-10 ${isDark ? "bg-card/20" : "bg-card/60"} backdrop-blur-2xl`}
      >
        {/* Background Gradients */}
        <div className="absolute top-0 right-0 w-[400px] h-[400px] bg-primary/[0.03] blur-[100px] rounded-full pointer-events-none z-0" />

        {!draft ? (
          <div className="flex-1 flex flex-col items-center justify-center text-muted-foreground p-8">
            <div className="p-4 rounded-full bg-foreground/5 mb-4">
              <Bot className="size-8 opacity-50" />
            </div>
            <p className="text-sm font-medium">
              Select an agent to view details
            </p>
          </div>
        ) : (
          <>
            {/* Header - Explicitly Draggable */}
            <header
              className="h-16 shrink-0 flex items-center justify-between px-8 border-b border-border/10 bg-background/20 backdrop-blur-xl z-20 relative"
              data-tauri-drag-region
            >
              <div className="flex items-center gap-3">
                <h2 className="text-lg font-bold text-foreground tracking-tight">
                  {draft.soul.name || "Untitled Agent"}
                </h2>
                <div className="flex items-center gap-1.5 px-2 py-0.5 rounded-full bg-foreground/5 border border-foreground/5">
                  <span className="w-1.5 h-1.5 rounded-full bg-primary" />
                  <span className="text-xs text-muted-foreground font-mono">
                    v{draft.version}
                  </span>
                </div>
              </div>

              <div className="flex items-center gap-2">
                <Button
                  onPress={() => onEditInBuilder(draft)}
                  isDisabled={isSaving || isDeploying}
                  variant="ghost"
                  size="sm"
                  className="text-muted-foreground hover:text-primary font-medium"
                >
                  <Pencil className="size-3.5 mr-1.5" />
                  Edit Visual
                </Button>
                <Button
                  onPress={handleSave}
                  isDisabled={(!isDirty && !draft) || isSaving || isDeploying}
                  variant="ghost"
                  size="sm"
                  className={`text-muted-foreground hover:text-primary font-medium ${isDirty ? "text-primary" : ""}`}
                >
                  <Save className="size-3.5 mr-1.5" />
                  Save
                </Button>
                <Button
                  onPress={handleDeploy}
                  isDisabled={isDeploying || isSaving}
                  className="bg-primary text-primary-foreground hover:bg-primary/90 font-bold px-6 h-8 min-w-0 rounded-full shadow-lg shadow-primary/20 text-sm"
                >
                  <Rocket className="size-3.5 mr-1.5" />
                  {isDeploying ? "Deploying..." : "Deploy"}
                </Button>
              </div>
            </header>

            {/* Tabs */}
            <div className="px-8 pt-4 pb-0 flex gap-6 border-b border-border/10 bg-background/5 backdrop-blur-sm z-20 relative">
              {(["review", "edit", "json"] as const).map((tab) => (
                <button
                  key={tab}
                  onClick={() => setActiveTab(tab)}
                  className={`pb-3 text-sm font-medium border-b-2 transition-all ${
                    activeTab === tab
                      ? "border-primary text-primary"
                      : "border-transparent text-muted-foreground hover:text-foreground hover:border-border/50"
                  }`}
                >
                  {tab.charAt(0).toUpperCase() + tab.slice(1)}
                </button>
              ))}
            </div>

            {/* Content Area */}
            <div className="flex-1 overflow-y-auto p-8 z-10 scrollbar-hide">
              <div className="max-w-3xl mx-auto pb-16">
                {activeTab === "review" && (
                  <div className="space-y-8 animate-appear">
                    {/* Details Grid */}
                    <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                      <div className="p-5 rounded-2xl bg-card/40 border border-border/20 backdrop-blur-md">
                        <h4 className="text-[10px] font-bold uppercase tracking-widest text-muted-foreground mb-3">
                          Personality
                        </h4>
                        <p className="text-sm leading-relaxed text-foreground/80">
                          {draft.soul.personality || "No personality defined."}
                        </p>
                      </div>
                      <div className="p-5 rounded-2xl bg-card/40 border border-border/20 backdrop-blur-md">
                        <h4 className="text-[10px] font-bold uppercase tracking-widest text-muted-foreground mb-3">
                          Config
                        </h4>
                        <div className="space-y-2">
                          <div className="flex justify-between text-xs border-b border-border/10 pb-2">
                            <span className="text-muted-foreground">
                              Memory Strategy
                            </span>
                            <span className="font-mono text-primary">
                              {draft.memory_config.strategy}
                            </span>
                          </div>
                          <div className="flex justify-between text-xs border-b border-border/10 pb-2">
                            <span className="text-muted-foreground">
                              Context Window
                            </span>
                            <span className="font-mono text-foreground">
                              {draft.memory_config.retrieval.max_tokens} tks
                            </span>
                          </div>
                          <div className="flex justify-between text-xs pt-0.5">
                            <span className="text-muted-foreground">
                              Skill Rules
                            </span>
                            <span className="font-mono text-foreground">
                              {draft.skills.workflows.length +
                                draft.skills.behaviors.length}
                            </span>
                          </div>
                        </div>
                      </div>
                    </div>

                    {/* Soul Content Preview */}
                    <div>
                      <h4 className="text-[10px] font-bold uppercase tracking-widest text-muted-foreground mb-3">
                        System Prompt
                      </h4>
                      <div className="w-full bg-card/40 rounded-xl border border-border/10 p-4 relative overflow-hidden group">
                        <div className="absolute top-0 left-0 w-1 h-full bg-primary/20 group-hover:bg-primary/50 transition-colors" />
                        <pre className="font-mono text-xs leading-relaxed text-foreground/70 whitespace-pre-wrap">
                          {draft.soul.soul_content ||
                            "No system prompt defined."}
                        </pre>
                      </div>
                    </div>
                  </div>
                )}

                {activeTab === "edit" && (
                  <div className="space-y-8 animate-appear">
                    <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                      <div className="space-y-1">
                        <label className="text-[10px] font-bold uppercase tracking-widest text-muted-foreground">
                          Name
                        </label>
                        <RawInput
                          value={draft.soul.name}
                          onChange={(e) =>
                            setDraft({
                              ...draft,
                              soul: { ...draft.soul, name: e.target.value },
                            })
                          }
                          className="text-lg font-bold"
                        />
                      </div>
                      <div className="space-y-1">
                        <label className="text-[10px] font-bold uppercase tracking-widest text-muted-foreground">
                          Version
                        </label>
                        <RawInput
                          value={draft.version}
                          onChange={(e) =>
                            setDraft({ ...draft, version: e.target.value })
                          }
                          className="font-mono text-primary"
                        />
                      </div>
                    </div>

                    <div className="space-y-1">
                      <label className="text-[10px] font-bold uppercase tracking-widest text-muted-foreground">
                        Description
                      </label>
                      <RawTextArea
                        value={draft.soul.description}
                        onChange={(e) =>
                          setDraft({
                            ...draft,
                            soul: {
                              ...draft.soul,
                              description: e.target.value,
                            },
                          })
                        }
                        rows={2}
                      />
                    </div>

                    {/* Memory Configuration (Restored) */}
                    <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                         <div className="space-y-1">
                            <label className="text-[10px] font-bold uppercase tracking-widest text-muted-foreground">
                            Retention (Days)
                            </label>
                            <RawInput
                            type="number"
                            value={draft.memory_config.retrieval.retention_days.toString()}
                            onChange={(e) => setMemoryNumber("retention_days", e.target.value)}
                            className="font-mono text-foreground"
                            />
                        </div>
                        <div className="space-y-1">
                            <label className="text-[10px] font-bold uppercase tracking-widest text-muted-foreground">
                            Context Window (Tokens)
                            </label>
                            <RawInput
                            type="number"
                            value={draft.memory_config.retrieval.max_tokens.toString()}
                            onChange={(e) => setMemoryNumber("max_tokens", e.target.value)}
                            className="font-mono text-foreground"
                            />
                        </div>
                    </div>

                    <div className="space-y-1">
                      <label className="text-[10px] font-bold uppercase tracking-widest text-muted-foreground">
                        Personality
                      </label>
                      <RawTextArea
                        value={draft.soul.personality}
                        onChange={(e) =>
                          setDraft({
                            ...draft,
                            soul: {
                              ...draft.soul,
                              personality: e.target.value,
                            },
                          })
                        }
                        rows={3}
                      />
                    </div>

                    <div className="space-y-1">
                      <label className="text-[10px] font-bold uppercase tracking-widest text-muted-foreground">
                        System Prompt
                      </label>
                      <div className="relative">
                        <div className="absolute top-3 right-3 text-[10px] font-bold text-muted-foreground/40 pointer-events-none">
                          MARKDOWN
                        </div>
                        <RawTextArea
                          value={draft.soul.soul_content}
                          onChange={(e) =>
                            setDraft({
                              ...draft,
                              soul: {
                                ...draft.soul,
                                soul_content: e.target.value,
                              },
                            })
                          }
                          rows={12}
                          className="font-mono text-xs"
                        />
                      </div>
                    </div>
                  </div>
                )}

                {activeTab === "json" && (
                  <div className="animate-appear">
                    <RawTextArea
                      value={JSON.stringify(draft, null, 2)}
                      readOnly={true}
                      className="font-mono text-xs text-primary/80 h-[600px]"
                      rows={30}
                    />
                  </div>
                )}
              </div>
            </div>
          </>
        )}
      </main>
    </div>
  );
}
