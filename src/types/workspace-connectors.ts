export interface WorkspaceConnectors {
  channels: ChannelConfig[];
  agent_routing: AgentChannelRoute[];
}

export interface ChannelConfig {
  type: "telegram" | "discord" | "whatsapp" | "custom_api";
  status: "connected" | "disconnected" | "pending";
  pairing_code?: string;
  auto_reply: boolean;
  rate_limit: {
    max_messages_per_minute: number;
  };
}

export interface AgentChannelRoute {
  agent_id: string;
  channel_types: string[];
  priority: number;
}
