import { useState } from "react";
import { Button, Card, Tabs, Tab, Spacer, Toast } from "@heroui/react";
import {
  AgentSpec,
  AgentSoul,
  AgentSkills,
  AgentSignature,
  Permission,
} from "../../types/agent-spec";
import { SoulEditor } from "./SoulEditor";
import { SkillsSelector } from "./SkillsSelector";
import { SecurityPanel } from "./SecurityPanel";
import { invoke } from "@tauri-apps/api/core";
import { UploadCloud, Save, ArrowLeft } from "lucide-react";

interface AgentBuilderProps {
  initialSpec?: AgentSpec;
  onBack?: () => void;
}

const DEFAULT_SPEC: AgentSpec = {
  id: crypto.randomUUID(),
  version: "1.0.0",
  soul: {
    name: "New Agent",
    description: "",
    version: "1.0.0",
    personality: "",
    tone: "Helpful",
    soul_content: "",
  },
  skills: {
    capabilities: [],
    tools: {},
  },
  memory_config: {
    strategy: "simple_buffer",
    retention_days: 7,
    max_tokens: 4096,
  },
  connectors: {
    telegram_enabled: false,
    auto_reply: true,
  },
};

export function AgentBuilder({ initialSpec, onBack }: AgentBuilderProps) {
  const [spec, setSpec] = useState<AgentSpec>(initialSpec || DEFAULT_SPEC);
  const [isDeploying, setIsDeploying] = useState(false);
  const [isSigning, setIsSigning] = useState(false);

  const handleSoulChange = (soul: AgentSoul) => {
    setSpec((prev) => ({ ...prev, soul }));
  };

  const handleSkillsChange = (skills: AgentSkills) => {
    setSpec((prev) => ({ ...prev, skills }));
    // Invalidate signature if skills change
    if (spec.signature) {
      setSpec((prev) => ({ ...prev, signature: undefined }));
    }
  };

  const handleSignAgent = async () => {
    setIsSigning(true);
    // In a real flow, this would call a backend command to sign 'spec' without deploying
    // For now, deployment handles signing, but we want visual feedback.
    // We'll simulate signing or call a specific sign command if we had one.
    // Let's assume we proceed to deploy directly for MVP.
    setTimeout(() => {
      setIsSigning(false);
      // Mock signature for UI feedback
      setSpec((prev) => ({
        ...prev,
        signature: {
          signature: "mock_sig",
          signer_id: "mock_key",
          capabilities_hash: "mock_hash",
          origin_device_id: "local",
          signed_at: Date.now(),
        },
      }));
      Toast.push({
        type: "success",
        message: "Agent Signed Locally",
      });
    }, 1000);
  };

  const handleDeploy = async () => {
    setIsDeploying(true);
    try {
      await invoke("deploy_agent", { spec });
      Toast.push({
        type: "success",
        message: "Agent Deployed to Rainy Cloud!",
      });
    } catch (error) {
      console.error("Deployment failed:", error);
      Toast.push({
        type: "error",
        message: `Deployment Failed: ${error}`,
      });
    } finally {
      setIsDeploying(false);
    }
  };

  return (
    <div className="flex flex-col h-full bg-background p-6 gap-6">
      {/* Header */}
      <div className="flex justify-between items-center">
        <div className="flex items-center gap-4">
          {onBack && (
            <Button isIconOnly variant="light" onPress={onBack}>
              <ArrowLeft className="size-5" />
            </Button>
          )}
          <div>
            <h1 className="text-2xl font-bold">Agent Builder</h1>
            <p className="text-default-500 text-sm">
              Design, Sign, and Deploy.
            </p>
          </div>
        </div>
        <div className="flex gap-2">
          <Button variant="flat" startContent={<Save className="size-4" />}>
            Save Local
          </Button>
          <Button
            color="primary"
            startContent={<UploadCloud className="size-4" />}
            isLoading={isDeploying}
            onPress={handleDeploy}
          >
            Deploy to Cloud
          </Button>
        </div>
      </div>

      {/* Main Content */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6 flex-1 min-h-0">
        {/* Left Col: Editor */}
        <div className="lg:col-span-2 overflow-y-auto pr-2 space-y-6">
          <Tabs aria-label="Agent Configuration">
            <Tab key="soul" title="Soul & Identity">
              <Spacer y={4} />
              <SoulEditor soul={spec.soul} onChange={handleSoulChange} />
            </Tab>
            <Tab key="skills" title="Skills & Permissions">
              <Spacer y={4} />
              <SkillsSelector
                skills={spec.skills}
                onChange={handleSkillsChange}
              />
            </Tab>
            <Tab key="memory" title="Memory">
              <Card>
                <div className="p-4">Memory Config Placeholder</div>
              </Card>
            </Tab>
          </Tabs>
        </div>

        {/* Right Col: Preview & Security */}
        <div className="flex flex-col gap-6">
          <SecurityPanel
            signature={spec.signature}
            onSign={handleSignAgent}
            isSigning={isSigning}
          />

          <Card className="flex-1 overflow-hidden">
            <div className="p-4 bg-default-50 h-full font-mono text-xs overflow-auto">
              <pre>{JSON.stringify(spec, null, 2)}</pre>
            </div>
          </Card>
        </div>
      </div>
    </div>
  );
}
