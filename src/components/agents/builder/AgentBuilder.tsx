import { useEffect, useState } from "react";
// @ts-ignore
import { Button, Select, ListBox, Label } from "@heroui/react";
import { toast } from "sonner";
import {
  Save,
  Bot,
  Shield,
  Network,
  Cpu,
  Rocket,
  ArrowLeft,
} from "lucide-react";
import { AgentSpec } from "../../../types/agent-spec";
import { SoulEditor } from "./SoulEditor";
import { SkillsSelector } from "./SkillsSelector";
import { SecurityPanel } from "./SecurityPanel";
import { createDefaultAgentSpec } from "./specDefaults";
import * as tauri from "../../../services/tauri";
import { useTheme } from "../../../hooks/useTheme";

interface AgentBuilderProps {
  onBack: () => void;
  initialSpec?: AgentSpec;
}

export function AgentBuilder({ onBack, initialSpec }: AgentBuilderProps) {
  const [spec, setSpec] = useState<AgentSpec>(() =>
    initialSpec ? structuredClone(initialSpec) : createDefaultAgentSpec(),
  );
  const [isSaving, setIsSaving] = useState(false);
  const [isDeploying, setIsDeploying] = useState(false);
  const [activeTab, setActiveTab] = useState<string>("soul");
  const { mode } = useTheme();
  const isDark = mode === "dark";

  useEffect(() => {
    setSpec(
      initialSpec ? structuredClone(initialSpec) : createDefaultAgentSpec(),
    );
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

  // Dynamic Opacity Rule: 60% Light, 20% Dark
  // Using Tailwind's opacity modifiers on bg-background/card or custom colors
  // panelClass = "bg-background/60 dark:bg-background/20 backdrop-blur-2xl border border-border/50"

  const NavItem = ({
    id,
    icon: Icon,
    label,
    description,
  }: {
    id: string;
    icon: any;
    label: string;
    description: string;
  }) => {
    const isActive = activeTab === id;
    return (
      <button
        onClick={() => setActiveTab(id)}
        className={`w-full text-left px-4 py-3 rounded-2xl transition-all duration-300 group relative overflow-hidden ${
          isActive
            ? "bg-primary text-primary-foreground shadow-md shadow-primary/10"
            : "hover:bg-foreground/5 text-muted-foreground hover:text-foreground"
        }`}
      >
        <div className="flex items-center gap-3 relative z-10">
          <div
            className={`p-1.5 rounded-full ${
              isActive ? "bg-black/10" : "bg-white/5 group-hover:bg-white/10"
            }`}
          >
            <Icon className="size-4" />
          </div>
          <div>
            <span
              className={`block text-sm font-bold ${isActive ? "text-primary-foreground" : "text-foreground"}`}
            >
              {label}
            </span>
            <span
              className={`text-[10px] uppercase tracking-wider ${isActive ? "text-primary-foreground/70" : "text-muted-foreground"}`}
            >
              {description}
            </span>
          </div>
        </div>
      </button>
    );
  };

  return (
    <div className="h-full w-full bg-background p-3 flex gap-3 overflow-hidden font-sans selection:bg-primary selection:text-primary-foreground relative">
      {/* Draggable Background Layer */}
      <div
        className="absolute inset-0 w-full h-full z-0"
        data-tauri-drag-region
      />

      {/* LEFT PANEL: Navigation */}
      <aside
        className={`w-[260px] shrink-0 rounded-[1.5rem] border border-border/40 flex flex-col shadow-xl overflow-hidden relative z-10 ${isDark ? "bg-card/20" : "bg-card/60"} backdrop-blur-2xl`}
      >
        {/* Header - Explicitly Draggable */}
        <div className="p-6 pb-2" data-tauri-drag-region>
          <button
            onClick={onBack}
            className="flex items-center gap-2 text-muted-foreground hover:text-primary transition-colors mb-4 group relative z-50 window-no-drag"
          >
            <ArrowLeft className="size-3 group-hover:-translate-x-1 transition-transform" />
            <span className="text-xs font-medium tracking-wide uppercase">
              Back
            </span>
          </button>
          <h1 className="text-xl font-bold text-foreground tracking-tight leading-tight pointer-events-none">
            Agent
            <br />
            Builder
          </h1>
        </div>

        {/* Nav Links */}
        <div className="flex-1 px-3 space-y-1 overflow-y-auto relative z-20">
          <NavItem
            id="soul"
            icon={Bot}
            label="Identity"
            description="Persona & core"
          />
          <NavItem
            id="skills"
            icon={Cpu}
            label="Skills"
            description="Capabilities"
          />
          <NavItem
            id="memory"
            icon={Network}
            label="Memory"
            description="Knowledge"
          />
          <NavItem
            id="security"
            icon={Shield}
            label="Security"
            description="Permissions"
          />
        </div>

        {/* Footer */}
        <div className="p-4 pt-2">
          <div className="text-[10px] text-muted-foreground font-mono text-center opacity-50 pointer-events-none">
            Rainy Cowork
          </div>
        </div>
      </aside>

      {/* RIGHT PANEL: Content Editor */}
      <main
        className={`flex-1 rounded-[1.5rem] border border-border/40 shadow-xl flex flex-col overflow-hidden relative z-10 ${isDark ? "bg-card/20" : "bg-card/60"} backdrop-blur-2xl`}
      >
        {/* Background Gradients - Dynamic Primary Color */}
        <div className="absolute top-0 right-0 w-[400px] h-[400px] bg-primary/[0.03] blur-[100px] rounded-full pointer-events-none z-0" />

        {/* Header - Explicitly Draggable */}
        <header
          className="h-16 shrink-0 flex items-center justify-between px-8 border-b border-border/10 bg-background/20 backdrop-blur-xl z-20 relative"
          data-tauri-drag-region
        >
          <div className="flex items-center gap-3">
            <h2 className="text-lg font-bold text-foreground tracking-tight">
              {spec.soul.name || "Untitled Agent"}
            </h2>
            <div className="flex items-center gap-1.5 px-2 py-0.5 rounded-full bg-foreground/5 border border-foreground/5">
              <span className="w-1.5 h-1.5 rounded-full bg-primary" />
              <span className="text-xs text-muted-foreground font-mono">
                v{spec.version}
              </span>
            </div>
          </div>

          <div className="flex items-center gap-2">
            <Button
              onPress={handleSave}
              isDisabled={isSaving || isDeploying}
              variant="ghost"
              size="sm"
              className="text-muted-foreground hover:text-primary font-medium"
            >
              <Save className="size-3.5 mr-1.5" />
              Save Draft
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

        {/* Content Area */}
        <div className="flex-1 overflow-y-auto p-8 z-10 scrollbar-hide">
          <div className="max-w-3xl mx-auto pb-16">
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
              <div className="space-y-8">
                <div className="flex flex-col gap-1">
                  <h3 className="text-xl font-bold text-foreground">
                    Memory Matrix
                  </h3>
                  <p className="text-muted-foreground text-sm">
                    Configure retention and retrieval.
                  </p>
                </div>

                <div className="grid grid-cols-1 md:grid-cols-2 gap-8">
                  {/* Strategy */}
                  <div className="space-y-3">
                    <Label className="text-muted-foreground text-[10px] font-bold uppercase tracking-widest">
                      Retrieval Strategy
                    </Label>
                    <Select
                      selectedKey={spec.memory_config.strategy}
                      onSelectionChange={(key) => {
                        if (key) {
                          updateSpec({
                            memory_config: {
                              ...spec.memory_config,
                              strategy: key as
                                | "vector"
                                | "simple_buffer"
                                | "hybrid",
                            },
                          });
                        }
                      }}
                      className="w-full"
                    >
                      <Label>Select Strategy</Label>
                      <Select.Trigger className="bg-card/40 hover:bg-card/60 border border-border/20 rounded-xl h-12 px-3 text-foreground transition-all data-[open=true]:border-primary/50 text-sm">
                        <Select.Value />
                        <Select.Indicator />
                      </Select.Trigger>
                      <Select.Popover className="bg-popover border border-border dark rounded-xl shadow-xl">
                        <ListBox>
                          <ListBox.Item
                            key="hybrid"
                            textValue="Hybrid Search"
                            className="data-[hover=true]:bg-foreground/5 py-2 rounded-lg"
                          >
                            <div className="flex flex-col gap-0.5">
                              <span className="text-sm font-bold text-foreground">
                                Hybrid Search
                              </span>
                              <span className="text-[10px] text-muted-foreground">
                                Vector + Short-term buffer (Recommended)
                              </span>
                            </div>
                          </ListBox.Item>
                          <ListBox.Item
                            key="vector"
                            textValue="Vector Only"
                            className="data-[hover=true]:bg-foreground/5 py-2 rounded-lg"
                          >
                            <div className="flex flex-col gap-0.5">
                              <span className="text-sm font-bold text-foreground">
                                Vector Only
                              </span>
                              <span className="text-[10px] text-muted-foreground">
                                Long-term semantic search
                              </span>
                            </div>
                          </ListBox.Item>
                          <ListBox.Item
                            key="simple_buffer"
                            textValue="Simple Buffer"
                            className="data-[hover=true]:bg-foreground/5 py-2 rounded-lg"
                          >
                            <div className="flex flex-col gap-0.5">
                              <span className="text-sm font-bold text-foreground">
                                Simple Buffer
                              </span>
                              <span className="text-[10px] text-muted-foreground">
                                FIFO context window only
                              </span>
                            </div>
                          </ListBox.Item>
                        </ListBox>
                      </Select.Popover>
                    </Select>
                  </div>

                  {/* Configs */}
                  <div className="space-y-6">
                    <div className="space-y-3">
                      <div className="flex justify-between items-end">
                        <Label className="text-muted-foreground text-[10px] font-bold uppercase tracking-widest">
                          Retention
                        </Label>
                        <span className="font-mono text-foreground text-xs">
                          {spec.memory_config.retention_days} days
                        </span>
                      </div>
                      <input
                        type="range"
                        min={1}
                        max={90}
                        className="w-full accent-primary h-1 bg-foreground/10 rounded-lg appearance-none cursor-pointer"
                        value={spec.memory_config.retention_days}
                        onChange={(e) =>
                          updateSpec({
                            memory_config: {
                              ...spec.memory_config,
                              retention_days: Math.max(
                                1,
                                parseInt(e.target.value || "1", 10),
                              ),
                            },
                          })
                        }
                      />
                    </div>

                    <div className="space-y-3">
                      <div className="flex justify-between items-end">
                        <Label className="text-muted-foreground text-[10px] font-bold uppercase tracking-widest">
                          Context Window
                        </Label>
                        <span className="font-mono text-foreground text-xs">
                          {spec.memory_config.max_tokens} tokens
                        </span>
                      </div>
                      <input
                        type="range"
                        min={512}
                        max={32000}
                        step={512}
                        className="w-full accent-primary h-1 bg-foreground/10 rounded-lg appearance-none cursor-pointer"
                        value={spec.memory_config.max_tokens}
                        onChange={(e) =>
                          updateSpec({
                            memory_config: {
                              ...spec.memory_config,
                              max_tokens: Math.max(
                                512,
                                parseInt(e.target.value || "512", 10),
                              ),
                            },
                          })
                        }
                      />
                    </div>
                  </div>
                </div>
              </div>
            )}
          </div>
        </div>
      </main>
    </div>
  );
}
