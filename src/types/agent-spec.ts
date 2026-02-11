import type { AirlockConfig } from "./airlock";
import type { MemoryConfig } from "./memory";

export interface AgentSpec {
  id: string;
  version: string;
  soul: AgentSoul;
  skills: AgentSkills;
  airlock: AirlockConfig;
  memory_config: MemoryConfig;
}

export interface AgentSoul {
  name: string;
  description: string;
  version: string;
  personality: string;
  tone: string;
  soul_content: string;
  embedding?: number[];
}

export interface AgentSkills {
  workflows: SkillWorkflow[];
  tool_preferences: ToolPreference[];
  behaviors: SkillBehavior[];

  // @deprecated Compatibility bridge for current Rust AgentSpec v2.
  capabilities?: Capability[];
  // @deprecated Compatibility bridge for current Rust AgentSpec v2.
  tools?: Record<string, unknown>;
}

export interface SkillWorkflow {
  id: string;
  name: string;
  description: string;
  trigger: string;
  steps: string;
  enabled: boolean;
}

export interface ToolPreference {
  tool_name: string;
  priority: "prefer" | "avoid" | "never";
  context: string;
}

export interface SkillBehavior {
  id: string;
  name: string;
  instruction: string;
  enabled: boolean;
}

export interface Capability {
  name: string;
  description: string;
  scopes: string[];
  permissions: Permission[];
}

export enum Permission {
  Read = "Read",
  Write = "Write",
  Execute = "Execute",
  Network = "Network",
}
