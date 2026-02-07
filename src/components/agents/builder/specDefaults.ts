import { AgentSpec } from "../../../types/agent-spec";

export function createDefaultAgentSpec(): AgentSpec {
  return {
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
}
