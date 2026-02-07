import { useState } from "react";
import { Button, Card } from "@heroui/react";
import { Save, ArrowLeft, Bot, Shield, Network, Cpu } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { AgentSpec } from "../../../types/agent-spec";
import { SoulEditor } from "./SoulEditor";
import { SkillsSelector } from "./SkillsSelector";
import { SecurityPanel } from "./SecurityPanel";

interface AgentBuilderProps {
  onBack: () => void;
  initialSpec?: AgentSpec;
}

const DEFAULT_SPEC: AgentSpec = {
  id: crypto.randomUUID(),
  version: "1.0.0",
  soul: {
    name: "",
    description: "",
    personality: "",
    tone: "",
    soul_content: "",
    version: "1.0.0",
  },
  skills: {
    capabilities: [],
    tools: {},
  },
  memory_config: {
    strategy: "hybrid",
    retention_days: 30,
    max_tokens: 32000,
  },
  connectors: {
    telegram_enabled: false,
    telegram_channel_id: undefined,
    auto_reply: true,
  },
};

export function AgentBuilder({ onBack, initialSpec }: AgentBuilderProps) {
  const [spec, setSpec] = useState<AgentSpec>(initialSpec || DEFAULT_SPEC);
  const [isSaving, setIsSaving] = useState(false);
  const [activeTab, setActiveTab] = useState<string>("soul");

  const handleSave = async () => {
    setIsSaving(true);
    try {
      await invoke("save_agent_spec", { spec });
      console.log("Agent saved successfully!");
      onBack();
    } catch (error) {
      console.error("Failed to save agent:", error);
    } finally {
      setIsSaving(false);
    }
  };

  const updateSpec = (updates: Partial<AgentSpec>) => {
    setSpec((prev: AgentSpec) => ({ ...prev, ...updates }));
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
          {/* Action buttons */}
          <Button
            variant="primary"
            onPress={handleSave}
            className="font-medium"
            isDisabled={isSaving}
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
                <p className="text-default-500">
                  Advanced memory settings coming soon.
                </p>
              </Card>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
