import type { AirlockConfig } from "./airlock";
import type { MemoryConfig } from "./memory";

export interface AgentSpec {
  id: string;
  version: string;
  soul: AgentSoul;
  skills: AgentSkills;
  airlock: AirlockConfig;
  memory_config: MemoryConfig;
  runtime?: RuntimeConfig;
  model?: string;
  temperature?: number;
  maxTokens?: number;
  provider?: string;
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
  prompt_skills: PromptSkillBinding[];

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

export type PromptSkillScope = "project" | "global" | "mate_managed";

export interface PromptSkillBinding {
  id: string;
  name: string;
  description: string;
  content: string;
  source_path: string;
  scope: PromptSkillScope;
  source_hash: string;
  enabled: boolean;
  last_synced_at: number;
}

export interface Capability {
  name: string;
  description: string;
  scopes: string[];
  permissions: Permission[];
}

export const Permission = {
  Read: "Read",
  Write: "Write",
  Execute: "Execute",
  Network: "Network",
} as const;
export type Permission = typeof Permission[keyof typeof Permission];

export interface RuntimeConfig {
  mode?: "single" | "supervisor";
  max_specialists?: number;
  verification_required?: boolean;
}
