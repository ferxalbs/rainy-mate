import type { AirlockConfig } from "./airlock";
import type { MemoryConfig } from "./memory";

export type RuntimeMode =
  | "single"
  | "parallel_supervisor"
  | "supervisor"
  | "hierarchical_supervisor";

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
export type PromptSkillKind = "prompt_skill" | "workspace_instruction";

export interface PromptSkillBinding {
  id: string;
  name: string;
  description: string;
  content: string;
  source_path: string;
  scope: PromptSkillScope;
  kind: PromptSkillKind;
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
  mode?: RuntimeMode;
  max_specialists?: number;
  verification_required?: boolean;
  delegation?: DelegationConfig;
  language_policy?: LanguagePolicyConfig;

  // @deprecated Legacy flat runtime fields (kept for backwards compatibility)
  delegation_policy?: string;
  max_depth?: number;
  max_threads?: number;
  max_parallel_subagents?: number;
  job_max_runtime_seconds?: number;
  final_synthesis_required?: boolean;
  internal_coordination_language?: string;
  final_response_language_mode?: string;
}

export interface DelegationConfig {
  policy?: "explicit_only" | "hybrid_intent_gated" | "auto_heuristic";
  max_depth?: number;
  max_threads?: number;
  max_parallel_subagents?: number;
  job_max_runtime_seconds?: number;
  final_synthesis_required?: boolean;
}

export interface LanguagePolicyConfig {
  internal_coordination_language?: "english";
  final_response_language_mode?: "user" | "english";
}
