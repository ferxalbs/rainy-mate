import { useEffect, useState } from "react";
import { Button } from "@heroui/react";
import { toast } from "sonner";
import { Save, Bot, Shield, Network, Cpu, Rocket, ArrowLeft } from "lucide-react";
import { AgentSpec } from "../../../types/agent-spec";
import { SoulEditor } from "./SoulEditor";
import { SkillsEditor } from "./SkillsEditor";
import { AirlockPanel } from "./AirlockPanel";
import { MemoryPanel } from "./MemoryPanel";
import { createDefaultAgentSpec, normalizeAgentSpec } from "./specDefaults";
import * as tauri from "../../../services/tauri";
import { useTheme } from "../../../hooks/useTheme";

interface AgentBuilderProps {
  onBack: () => void;
  initialSpec?: AgentSpec;
}

export function AgentBuilder({ onBack, initialSpec }: AgentBuilderProps) {
  const [spec, setSpec] = useState<AgentSpec>(() =>
    initialSpec ? normalizeAgentSpec(initialSpec) : createDefaultAgentSpec(),
  );
  const [isSaving, setIsSaving] = useState(false);
  const [isDeploying, setIsDeploying] = useState(false);
  const [activeTab, setActiveTab] = useState<string>("soul");
  const { mode } = useTheme();
  const isDark = mode === "dark";

  useEffect(() => {
    setSpec(
      initialSpec ? normalizeAgentSpec(initialSpec) : createDefaultAgentSpec(),
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

      const result = await tauri.deployAgentSpec(spec);
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
      <div
        className="absolute inset-0 w-full h-full z-0"
        data-tauri-drag-region
      />

      <aside
        className={`w-[260px] shrink-0 rounded-[1.5rem] border border-border/40 flex flex-col shadow-xl overflow-hidden relative z-10 ${isDark ? "bg-card/20" : "bg-card/60"} backdrop-blur-2xl`}
      >
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

        <div className="flex-1 px-3 space-y-1 overflow-y-auto relative z-20">
          <NavItem
            id="soul"
            icon={Bot}
            label="Soul"
            description="Persona & core"
          />
          <NavItem
            id="skills"
            icon={Cpu}
            label="Skills"
            description="Workflows"
          />
          <NavItem
            id="memory"
            icon={Network}
            label="Memory"
            description="Knowledge"
          />
          <NavItem
            id="airlock"
            icon={Shield}
            label="Airlock"
            description="Permissions"
          />
        </div>

        <div className="p-4 pt-2">
          <div className="text-[10px] text-muted-foreground font-mono text-center opacity-50 pointer-events-none">
            Rainy Cowork
          </div>
        </div>
      </aside>

      <main
        className={`flex-1 rounded-[1.5rem] border border-border/40 shadow-xl flex flex-col overflow-hidden relative z-10 ${isDark ? "bg-card/20" : "bg-card/60"} backdrop-blur-2xl`}
      >
        <div className="absolute top-0 right-0 w-[400px] h-[400px] bg-primary/[0.03] blur-[100px] rounded-full pointer-events-none z-0" />

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

        <div className="flex-1 overflow-y-auto p-8 z-10 scrollbar-hide">
          <div className="max-w-3xl mx-auto pb-16">
            {activeTab === "soul" && (
              <SoulEditor
                soul={spec.soul}
                onChange={(s) => updateSpec({ soul: s })}
              />
            )}

            {activeTab === "skills" && (
              <SkillsEditor
                skills={spec.skills}
                onChange={(s) => updateSpec({ skills: s })}
              />
            )}

            {activeTab === "airlock" && (
              <AirlockPanel
                airlock={spec.airlock}
                onChange={(a) => updateSpec({ airlock: a })}
              />
            )}

            {activeTab === "memory" && (
              <MemoryPanel
                agentId={spec.id}
                memoryConfig={spec.memory_config}
                onChange={(memoryConfig) => updateSpec({ memory_config: memoryConfig })}
              />
            )}
          </div>
        </div>
      </main>
    </div>
  );
}
