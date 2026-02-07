import { useCallback, useEffect, useMemo, useState } from "react";
import { Button, Card, Chip, Input, Spinner, TextArea } from "@heroui/react";
import { toast } from "sonner";
import { AgentSpec } from "../../../types/agent-spec";
import * as tauri from "../../../services/tauri";
import {
  Bot,
  RefreshCw,
  Rocket,
  Pencil,
  Eye,
  FileJson,
  Plus,
  Save,
} from "lucide-react";

type StoreTab = "review" | "edit" | "json";

interface AgentStorePageProps {
  onCreateAgent: () => void;
  onEditInBuilder: (spec: AgentSpec) => void;
}

function cloneSpec(spec: AgentSpec): AgentSpec {
  return JSON.parse(JSON.stringify(spec)) as AgentSpec;
}

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

  const loadAgents = useCallback(async () => {
    setIsLoading(true);
    try {
      const specs = (await tauri.listAgentSpecs()) as AgentSpec[];
      setAgents(specs);
      if (specs.length > 0) {
        setSelectedId((prev) => prev || specs[0].id);
      } else {
        setSelectedId("");
        setDraft(null);
      }
    } catch (error) {
      console.error("Failed to load agents:", error);
      toast.error("Failed to load saved agents");
    } finally {
      setIsLoading(false);
    }
  }, []);

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
        [field]: nextValue,
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
      await tauri.deployAgentSpec(draft);
      toast.success("Agent deployed to Rainy-ATM");
    } catch (error) {
      console.error("Failed to deploy agent:", error);
      toast.error(`Deploy failed: ${error}`);
    } finally {
      setIsDeploying(false);
    }
  };

  return (
    <div className="h-full min-h-0 flex gap-4">
      <Card className="w-[340px] shrink-0 h-full min-h-0 bg-content1/50 backdrop-blur-md border border-white/10">
        <Card.Header className="flex flex-col items-stretch gap-3 p-4 border-b border-divider">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-base font-semibold">Agents Store</p>
              <p className="text-xs text-default-500">
                Review, edit, and deploy saved agents
              </p>
            </div>
            <Chip variant="soft" size="sm">
              {agents.length}
            </Chip>
          </div>
          <Input
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Search by name or description"
          />
          <div className="flex gap-2">
            <Button
              variant="secondary"
              className="flex-1"
              onPress={loadAgents}
              isDisabled={isLoading}
            >
              <RefreshCw className={`size-4 ${isLoading ? "animate-spin" : ""}`} />
              Refresh
            </Button>
            <Button variant="primary" className="flex-1" onPress={onCreateAgent}>
              <Plus className="size-4" />
              New
            </Button>
          </div>
        </Card.Header>
        <Card.Content className="p-2 overflow-auto space-y-1">
          {isLoading ? (
            <div className="py-16 flex items-center justify-center">
              <Spinner size="lg" />
            </div>
          ) : filteredAgents.length === 0 ? (
            <div className="py-12 px-4 text-center text-default-500 text-sm">
              No agents found.
            </div>
          ) : (
            filteredAgents.map((agent) => {
              const isSelected = selectedId === agent.id;
              return (
                <Button
                  key={agent.id}
                  variant={isSelected ? "secondary" : "ghost"}
                  className="w-full h-auto p-3 justify-start"
                  onPress={() => setSelectedId(agent.id)}
                >
                  <div className="w-full text-left">
                    <div className="flex items-center gap-2">
                      <Bot className="size-4 shrink-0 text-primary" />
                      <p className="text-sm font-medium truncate">
                        {agent.soul.name || "Untitled Agent"}
                      </p>
                    </div>
                    <p className="text-xs text-default-500 truncate mt-1">
                      {agent.soul.description || "No description"}
                    </p>
                  </div>
                </Button>
              );
            })
          )}
        </Card.Content>
      </Card>

      <Card className="flex-1 h-full min-h-0 bg-content1/40 backdrop-blur-md border border-white/10">
        {!draft ? (
          <div className="h-full flex items-center justify-center p-8 text-default-500 text-sm">
            Select an agent from the store to review or edit it.
          </div>
        ) : (
          <>
            <Card.Header className="p-5 border-b border-divider flex flex-col gap-4">
              <div className="flex items-start justify-between gap-4">
                <div className="space-y-1">
                  <h2 className="text-xl font-semibold">
                    {draft.soul.name || "Untitled Agent"}
                  </h2>
                  <p className="text-sm text-default-500">
                    {draft.soul.description || "No description"}
                  </p>
                  <div className="flex gap-2 pt-1">
                    <Chip size="sm" variant="soft">
                      v{draft.version}
                    </Chip>
                    <Chip size="sm" variant="soft">
                      {draft.memory_config.strategy}
                    </Chip>
                    <Chip size="sm" variant="soft">
                      caps: {draft.skills.capabilities.length}
                    </Chip>
                  </div>
                </div>
                <div className="flex gap-2">
                  <Button
                    variant="secondary"
                    onPress={() => onEditInBuilder(draft)}
                    isDisabled={isSaving || isDeploying}
                  >
                    <Pencil className="size-4 mr-2" />
                    Builder
                  </Button>
                  <Button
                    variant="secondary"
                    onPress={handleDeploy}
                    isDisabled={isSaving || isDeploying}
                  >
                    <Rocket className="size-4 mr-2" />
                    {isDeploying ? "Deploying..." : "Deploy"}
                  </Button>
                  <Button
                    variant="primary"
                    onPress={handleSave}
                    isDisabled={!isDirty || isSaving || isDeploying}
                  >
                    <Save className="size-4 mr-2" />
                    {isSaving ? "Saving..." : "Save"}
                  </Button>
                </div>
              </div>
              <div className="flex gap-2">
                <Button
                  variant={activeTab === "review" ? "secondary" : "ghost"}
                  onPress={() => setActiveTab("review")}
                >
                  <Eye className="size-4 mr-2" />
                  Review
                </Button>
                <Button
                  variant={activeTab === "edit" ? "secondary" : "ghost"}
                  onPress={() => setActiveTab("edit")}
                >
                  <Pencil className="size-4 mr-2" />
                  Edit
                </Button>
                <Button
                  variant={activeTab === "json" ? "secondary" : "ghost"}
                  onPress={() => setActiveTab("json")}
                >
                  <FileJson className="size-4 mr-2" />
                  JSON
                </Button>
              </div>
            </Card.Header>
            <Card.Content className="p-5 overflow-auto">
              {activeTab === "review" && (
                <div className="space-y-4">
                  <Card className="p-4">
                    <p className="text-xs uppercase tracking-wide text-default-500 mb-2">
                      Personality
                    </p>
                    <p className="text-sm whitespace-pre-wrap">
                      {draft.soul.personality || "Not defined"}
                    </p>
                  </Card>

                  <Card className="p-4">
                    <p className="text-xs uppercase tracking-wide text-default-500 mb-2">
                      Soul Content
                    </p>
                    <p className="text-sm whitespace-pre-wrap">
                      {draft.soul.soul_content || "No soul content"}
                    </p>
                  </Card>

                  <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
                    <Card className="p-4">
                      <p className="text-xs uppercase tracking-wide text-default-500 mb-2">
                        Memory
                      </p>
                      <p className="text-sm">
                        {draft.memory_config.strategy} •{" "}
                        {draft.memory_config.retention_days} days
                      </p>
                      <p className="text-sm text-default-500 mt-1">
                        {draft.memory_config.max_tokens} tokens
                      </p>
                    </Card>
                    <Card className="p-4">
                      <p className="text-xs uppercase tracking-wide text-default-500 mb-2">
                        Connectors
                      </p>
                      <p className="text-sm">
                        Telegram:{" "}
                        {draft.connectors.telegram_enabled ? "Enabled" : "Disabled"}
                      </p>
                      <p className="text-sm text-default-500 mt-1">
                        Auto reply: {draft.connectors.auto_reply ? "On" : "Off"}
                      </p>
                    </Card>
                    <Card className="p-4">
                      <p className="text-xs uppercase tracking-wide text-default-500 mb-2">
                        Skills
                      </p>
                      <p className="text-sm">
                        {draft.skills.capabilities.length} capabilities
                      </p>
                      <p className="text-sm text-default-500 mt-1 truncate">
                        {draft.skills.capabilities.map((c) => c.name).join(", ") ||
                          "No capabilities selected"}
                      </p>
                    </Card>
                  </div>
                </div>
              )}

              {activeTab === "edit" && (
                <div className="space-y-4 max-w-4xl">
                  <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                    <div className="space-y-2">
                      <p className="text-xs uppercase tracking-wide text-default-500">
                        Name
                      </p>
                      <Input
                        value={draft.soul.name}
                        onChange={(e) =>
                          setDraft({
                            ...draft,
                            soul: { ...draft.soul, name: e.target.value },
                          })
                        }
                      />
                    </div>
                    <div className="space-y-2">
                      <p className="text-xs uppercase tracking-wide text-default-500">
                        Tone
                      </p>
                      <Input
                        value={draft.soul.tone}
                        onChange={(e) =>
                          setDraft({
                            ...draft,
                            soul: { ...draft.soul, tone: e.target.value },
                          })
                        }
                      />
                    </div>
                  </div>

                  <div className="space-y-2">
                    <p className="text-xs uppercase tracking-wide text-default-500">
                      Description
                    </p>
                    <Input
                      value={draft.soul.description}
                      onChange={(e) =>
                        setDraft({
                          ...draft,
                          soul: { ...draft.soul, description: e.target.value },
                        })
                      }
                    />
                  </div>

                  <div className="space-y-2">
                    <p className="text-xs uppercase tracking-wide text-default-500">
                      Personality
                    </p>
                    <TextArea
                      value={draft.soul.personality}
                      onChange={(e) =>
                        setDraft({
                          ...draft,
                          soul: { ...draft.soul, personality: e.target.value },
                        })
                      }
                      rows={3}
                    />
                  </div>

                  <div className="space-y-2">
                    <p className="text-xs uppercase tracking-wide text-default-500">
                      Soul Content
                    </p>
                    <TextArea
                      value={draft.soul.soul_content}
                      onChange={(e) =>
                        setDraft({
                          ...draft,
                          soul: { ...draft.soul, soul_content: e.target.value },
                        })
                      }
                      rows={6}
                    />
                  </div>

                  <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                    <div className="space-y-2">
                      <p className="text-xs uppercase tracking-wide text-default-500">
                        Memory Strategy
                      </p>
                      <Input
                        value={draft.memory_config.strategy}
                        onChange={(e) =>
                          setDraft({
                            ...draft,
                            memory_config: {
                              ...draft.memory_config,
                              strategy: e.target.value as
                                | "hybrid"
                                | "simple_buffer"
                                | "vector",
                            },
                          })
                        }
                      />
                    </div>
                    <div className="space-y-2">
                      <p className="text-xs uppercase tracking-wide text-default-500">
                        Retention Days
                      </p>
                      <Input
                        type="number"
                        value={String(draft.memory_config.retention_days)}
                        onChange={(e) =>
                          setMemoryNumber("retention_days", e.target.value)
                        }
                      />
                    </div>
                    <div className="space-y-2">
                      <p className="text-xs uppercase tracking-wide text-default-500">
                        Max Tokens
                      </p>
                      <Input
                        type="number"
                        value={String(draft.memory_config.max_tokens)}
                        onChange={(e) => setMemoryNumber("max_tokens", e.target.value)}
                      />
                    </div>
                  </div>

                  <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                    <div className="space-y-2">
                      <p className="text-xs uppercase tracking-wide text-default-500">
                        Telegram Channel ID
                      </p>
                      <Input
                        value={draft.connectors.telegram_channel_id || ""}
                        onChange={(e) =>
                          setDraft({
                            ...draft,
                            connectors: {
                              ...draft.connectors,
                              telegram_channel_id: e.target.value || undefined,
                            },
                          })
                        }
                      />
                    </div>
                    <div className="flex items-end gap-2">
                      <Button
                        className="flex-1"
                        variant={
                          draft.connectors.telegram_enabled ? "secondary" : "ghost"
                        }
                        onPress={() =>
                          setDraft({
                            ...draft,
                            connectors: {
                              ...draft.connectors,
                              telegram_enabled: !draft.connectors.telegram_enabled,
                            },
                          })
                        }
                      >
                        Telegram{" "}
                        {draft.connectors.telegram_enabled ? "Enabled" : "Disabled"}
                      </Button>
                      <Button
                        className="flex-1"
                        variant={draft.connectors.auto_reply ? "secondary" : "ghost"}
                        onPress={() =>
                          setDraft({
                            ...draft,
                            connectors: {
                              ...draft.connectors,
                              auto_reply: !draft.connectors.auto_reply,
                            },
                          })
                        }
                      >
                        Auto Reply {draft.connectors.auto_reply ? "On" : "Off"}
                      </Button>
                    </div>
                  </div>
                </div>
              )}

              {activeTab === "json" && (
                <TextArea
                  value={JSON.stringify(draft, null, 2)}
                  readOnly
                  rows={24}
                  className="font-mono text-xs"
                />
              )}
            </Card.Content>
          </>
        )}
      </Card>
    </div>
  );
}
