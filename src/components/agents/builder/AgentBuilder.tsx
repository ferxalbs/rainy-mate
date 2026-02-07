import { useEffect, useState } from "react";
import { Button, Card } from "@heroui/react";
import { toast } from "sonner";
import {
  Save,
  ArrowLeft,
  Bot,
  Shield,
  Network,
  Cpu,
  Rocket,
  Library,
} from "lucide-react";
import { AgentSpec } from "../../../types/agent-spec";
import { SoulEditor } from "./SoulEditor";
import { SkillsSelector } from "./SkillsSelector";
import { SecurityPanel } from "./SecurityPanel";
import { createDefaultAgentSpec } from "./specDefaults";
import * as tauri from "../../../services/tauri";

interface AgentBuilderProps {
  onBack: () => void;
  initialSpec?: AgentSpec;
  onOpenStore?: () => void;
}

export function AgentBuilder({
  onBack,
  initialSpec,
  onOpenStore,
}: AgentBuilderProps) {
  const [spec, setSpec] = useState<AgentSpec>(() =>
    initialSpec ? structuredClone(initialSpec) : createDefaultAgentSpec(),
  );
  const [isSaving, setIsSaving] = useState(false);
  const [isDeploying, setIsDeploying] = useState(false);
  const [activeTab, setActiveTab] = useState<string>("soul");

  useEffect(() => {
    setSpec(initialSpec ? structuredClone(initialSpec) : createDefaultAgentSpec());
  }, [initialSpec]);

  const handleSave = async () => {
    setIsSaving(true);
    try {
      await tauri.saveAgentSpec(spec);
      toast.success("Agent saved successfully!");
    } catch (error) {
      console.error("Failed to save agent:", error);
      toast.error("Failed to save agent");
    } finally {
      setIsSaving(false);
    }
  };

  const updateSpec = (updates: Partial<AgentSpec>) => {
    setSpec((prev: AgentSpec) => ({ ...prev, ...updates }));
  };

  const handleDeploy = async () => {
    setIsDeploying(true);
    try {
      const hasCredentials = await tauri.ensureAtmCredentialsLoaded();
      if (!hasCredentials) {
        throw new Error(
          "Rainy-ATM is not authenticated. Configure ATM credentials first.",
        );
      }

      await tauri.deployAgentSpec(spec);
      toast.success("Agent deployed to Rainy-ATM");
    } catch (error) {
      console.error("Failed to deploy agent:", error);
      toast.error(`Deploy failed: ${error}`);
    } finally {
      setIsDeploying(false);
    }
  };

  return (
    <div className="h-full flex flex-col bg-background">
      {/* Header */}
      <div className="h-14 border-b border-divider flex items-center px-4 justify-between bg-content1/50 backdrop-blur">
        <div className="flex items-center gap-4">
          <Button isIconOnly variant="ghost" onPress={onBack}>
            <ArrowLeft className="size-5" />
          </Button>
          <div>
            <h1 className="text-lg font-bold">
              {initialSpec ? "Edit Agent" : "New Agent"}
            </h1>
            <p className="text-xs text-default-500">
              {spec.soul.name || "Untitled Agent"} • v{spec.version}
            </p>
          </div>
        </div>
        <div className="flex gap-2">
          {onOpenStore && (
            <Button variant="ghost" onPress={onOpenStore} className="font-medium">
              <Library className="size-4 mr-2" /> Agent Store
            </Button>
          )}
          {/* Action buttons */}
          <Button
            variant="secondary"
            onPress={handleDeploy}
            className="font-medium"
            isDisabled={isDeploying || isSaving}
          >
            {isDeploying ? (
              "Deploying..."
            ) : (
              <>
                <Rocket className="size-4 mr-2" /> Deploy
              </>
            )}
          </Button>
          <Button
            variant="primary"
            onPress={handleSave}
            className="font-medium"
            isDisabled={isSaving || isDeploying}
          >
            {isSaving ? (
              "Saving..."
            ) : (
              <>
                <Save className="size-4 mr-2" /> Save Agent
              </>
            )}
          </Button>
        </div>
      </div>

      {/* Main Content */}
      <div className="flex-1 overflow-hidden flex">
        {/* Sidebar/Tabs - keeping simple layout */}
        <div className="w-64 border-r border-default-200 p-4 flex flex-col gap-2 bg-content1/30">
          <Button
            variant={activeTab === "soul" ? "primary" : "ghost"}
            className="justify-start"
            onPress={() => setActiveTab("soul")}
          >
            <Bot className="size-4 mr-2" />
            Identity & Soul
          </Button>
          <Button
            variant={activeTab === "skills" ? "primary" : "ghost"}
            className="justify-start"
            onPress={() => setActiveTab("skills")}
          >
            <Cpu className="size-4 mr-2" />
            Skills & Tools
          </Button>
          <Button
            variant={activeTab === "memory" ? "primary" : "ghost"}
            className="justify-start"
            onPress={() => setActiveTab("memory")}
          >
            <Network className="size-4 mr-2" />
            Memory
          </Button>
          <Button
            variant={activeTab === "security" ? "primary" : "ghost"}
            className="justify-start h-auto py-2"
            onPress={() => setActiveTab("security")}
          >
            <Shield className="size-4 mr-2 flex-shrink-0" />
            Security
          </Button>
        </div>

        {/* Editor Area */}
        <div className="flex-1 overflow-auto p-6">
          <div className="max-w-4xl mx-auto space-y-6">
            {activeTab === "soul" && (
              <SoulEditor
                soul={spec.soul}
                onChange={(s) => updateSpec({ soul: s })}
              />
            )}

            {activeTab === "skills" && (
              <SkillsSelector
                skills={spec.skills}
                onChange={(s) => updateSpec({ skills: s })}
              />
            )}

            {activeTab === "security" && (
              <SecurityPanel
                spec={spec}
                onUpdate={(updates) =>
                  setSpec((prev: AgentSpec) => ({ ...prev, ...updates }))
                }
              />
            )}

            {activeTab === "memory" && (
              <Card className="p-6">
                <h3 className="text-lg font-bold mb-4">Memory Configuration</h3>
                <div className="space-y-4">
                  <div>
                    <label
                      htmlFor="memory-strategy"
                      className="text-sm font-medium mb-2 block"
                    >
                      Strategy
                    </label>
                    <select
                      id="memory-strategy"
                      className="w-full rounded-xl border border-default-200 bg-content1 px-3 py-2 text-sm"
                      value={spec.memory_config.strategy}
                      onChange={(e) =>
                        updateSpec({
                          memory_config: {
                            ...spec.memory_config,
                            strategy: e.target.value as
                              | "vector"
                              | "simple_buffer"
                              | "hybrid",
                          },
                        })
                      }
                    >
                      <option value="hybrid">Hybrid</option>
                      <option value="vector">Vector</option>
                      <option value="simple_buffer">Simple Buffer</option>
                    </select>
                  </div>

                  <div>
                    <label
                      htmlFor="retention-days"
                      className="text-sm font-medium mb-2 block"
                    >
                      Retention (Days)
                    </label>
                    <input
                      id="retention-days"
                      type="number"
                      min={1}
                      max={3650}
                      className="w-full rounded-xl border border-default-200 bg-content1 px-3 py-2 text-sm"
                      value={spec.memory_config.retention_days}
                      onChange={(e) =>
                        updateSpec({
                          memory_config: {
                            ...spec.memory_config,
                            retention_days: Math.max(
                              1,
                              Number.parseInt(e.target.value || "1", 10),
                            ),
                          },
                        })
                      }
                    />
                  </div>

                  <div>
                    <label
                      htmlFor="max-tokens"
                      className="text-sm font-medium mb-2 block"
                    >
                      Max Tokens
                    </label>
                    <input
                      id="max-tokens"
                      type="number"
                      min={512}
                      max={1000000}
                      step={512}
                      className="w-full rounded-xl border border-default-200 bg-content1 px-3 py-2 text-sm"
                      value={spec.memory_config.max_tokens}
                      onChange={(e) =>
                        updateSpec({
                          memory_config: {
                            ...spec.memory_config,
                            max_tokens: Math.max(
                              512,
                              Number.parseInt(e.target.value || "512", 10),
                            ),
                          },
                        })
                      }
                    />
                  </div>
                </div>
              </Card>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
