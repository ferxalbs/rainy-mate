import type { AgentSpec } from "../../../types/agent-spec";

export function createDefaultAgentSpec(id: string = crypto.randomUUID()): AgentSpec {
  return {
    id,
    version: "3.0.0",
    model: "openai/gpt-5-nano",
    temperature: 0.4,
    maxTokens: 4096,
    provider: "rainy",
    soul: {
      name: "",
      description: "",
      personality: "",
      tone: "",
      soul_content: "",
      version: "1.0.0",
    },
    skills: {
      workflows: [],
      tool_preferences: [],
      behaviors: [],
    },
    airlock: {
      tool_policy: {
        mode: "all",
        allow: [],
        deny: [],
      },
      tool_levels: {},
      scopes: {
        allowed_paths: [],
        blocked_paths: [],
        allowed_domains: [],
        blocked_domains: [],
      },
      rate_limits: {
        max_requests_per_minute: 0,
      },
    },
    memory_config: {
      strategy: "hybrid",
      retrieval: {
        retention_days: 30,
        max_tokens: 32000,
      },
      persistence: {
        cross_session: false,
        per_connector_isolation: false,
        session_scope: "per_user",
      },
      knowledge: {
        enabled: false,
        indexed_files: [],
      },
    },
  };
}

export function normalizeAgentSpec(raw: any): AgentSpec {
  const defaults = createDefaultAgentSpec("");
  const source = raw ?? {};
  const sourceMemory = source.memory_config ?? {};
  const sourceRetrieval = sourceMemory.retrieval ?? {};
  const sourcePersistence = sourceMemory.persistence ?? {};
  const sourceKnowledge = sourceMemory.knowledge ?? {};
  const sourceId =
    typeof source.id === "string" && source.id.trim().length > 0
      ? source.id.trim()
      : "";

  return {
    ...defaults,
    ...source,
    id: sourceId,
    soul: {
      ...defaults.soul,
      ...(source.soul ?? {}),
    },
    skills: {
      ...defaults.skills,
      ...(source.skills ?? {}),
      workflows: Array.isArray(source.skills?.workflows)
        ? source.skills.workflows
        : [],
      tool_preferences: Array.isArray(source.skills?.tool_preferences)
        ? source.skills.tool_preferences
        : [],
      behaviors: Array.isArray(source.skills?.behaviors)
        ? source.skills.behaviors
        : [],
    },
    airlock: {
      ...defaults.airlock,
      ...(source.airlock ?? {}),
      tool_policy: {
        ...defaults.airlock.tool_policy,
        ...(source.airlock?.tool_policy ?? {}),
        allow: Array.isArray(source.airlock?.tool_policy?.allow)
          ? source.airlock.tool_policy.allow
          : defaults.airlock.tool_policy.allow,
        deny: Array.isArray(source.airlock?.tool_policy?.deny)
          ? source.airlock.tool_policy.deny
          : defaults.airlock.tool_policy.deny,
      },
      tool_levels:
        source.airlock && typeof source.airlock.tool_levels === "object"
          ? source.airlock.tool_levels
          : defaults.airlock.tool_levels,
      scopes: {
        ...defaults.airlock.scopes,
        ...(source.airlock?.scopes ?? {}),
        allowed_paths: Array.isArray(source.airlock?.scopes?.allowed_paths)
          ? source.airlock.scopes.allowed_paths
          : defaults.airlock.scopes.allowed_paths,
        blocked_paths: Array.isArray(source.airlock?.scopes?.blocked_paths)
          ? source.airlock.scopes.blocked_paths
          : defaults.airlock.scopes.blocked_paths,
        allowed_domains: Array.isArray(source.airlock?.scopes?.allowed_domains)
          ? source.airlock.scopes.allowed_domains
          : defaults.airlock.scopes.allowed_domains,
        blocked_domains: Array.isArray(source.airlock?.scopes?.blocked_domains)
          ? source.airlock.scopes.blocked_domains
          : defaults.airlock.scopes.blocked_domains,
      },
      rate_limits: {
        ...defaults.airlock.rate_limits,
        ...(source.airlock?.rate_limits ?? {}),
      },
    },
    memory_config: {
      ...defaults.memory_config,
      ...sourceMemory,
      retrieval: {
        ...defaults.memory_config.retrieval,
        ...(sourceMemory.retention_days || sourceMemory.max_tokens
          ? {
              retention_days:
                sourceMemory.retention_days ??
                defaults.memory_config.retrieval.retention_days,
              max_tokens:
                sourceMemory.max_tokens ??
                defaults.memory_config.retrieval.max_tokens,
            }
          : {}),
        ...sourceRetrieval,
      },
      persistence: {
        ...defaults.memory_config.persistence,
        ...sourcePersistence,
      },
      knowledge: {
        ...defaults.memory_config.knowledge,
        ...sourceKnowledge,
        indexed_files: Array.isArray(sourceKnowledge.indexed_files)
          ? sourceKnowledge.indexed_files
          : defaults.memory_config.knowledge.indexed_files,
      },
    },
    model:
      typeof source.model === "string" && source.model.trim().length > 0
        ? source.model.trim()
        : defaults.model,
    temperature:
      typeof source.temperature === "number"
        ? source.temperature
        : defaults.temperature,
    maxTokens:
      typeof source.maxTokens === "number"
        ? source.maxTokens
        : defaults.maxTokens,
    provider:
      typeof source.provider === "string" && source.provider.trim().length > 0
        ? source.provider.trim()
        : defaults.provider,
  };
}
