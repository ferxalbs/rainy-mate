import { AgentSpec } from "../../../types/agent-spec";

export function createDefaultAgentSpec(): AgentSpec {
  return {
    id: crypto.randomUUID(),
    version: "3.0.0",
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

      // Compatibility with current Rust v2 parser.
      capabilities: [],
      tools: {},
    },
    airlock: {
      tool_policy: {
        mode: "allowlist",
        allow: [
          "read_file",
          "list_files",
          "search_files",
          "get_page_content",
          "web_search",
          "read_web_page",
          "write_file",
          "append_file",
          "mkdir",
        ],
        deny: ["delete_file", "execute_command"],
      },
      tool_levels: {
        read_file: 0,
        list_files: 0,
        search_files: 0,
        get_page_content: 0,
        web_search: 0,
        read_web_page: 0,
        write_file: 1,
        append_file: 1,
        mkdir: 1,
        move_file: 1,
        delete_file: 2,
        execute_command: 2,
        browse_url: 2,
      },
      scopes: {
        allowed_paths: ["/Users/*/Projects"],
        blocked_paths: ["/etc", "/usr", "~/.ssh", "~/.env"],
        allowed_domains: ["*"],
        blocked_domains: [],
      },
      rate_limits: {
        max_requests_per_minute: 30,
        max_tokens_per_day: 1_000_000,
      },
    },
    memory_config: {
      strategy: "hybrid",
      retrieval: {
        retention_days: 30,
        max_tokens: 32000,
      },
      persistence: {
        cross_session: true,
        per_connector_isolation: true,
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
  const defaults = createDefaultAgentSpec();
  const source = raw ?? {};
  const sourceMemory = source.memory_config ?? {};
  const sourceRetrieval = sourceMemory.retrieval ?? {};
  const sourcePersistence = sourceMemory.persistence ?? {};
  const sourceKnowledge = sourceMemory.knowledge ?? {};

  return {
    ...defaults,
    ...source,
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
      capabilities: Array.isArray(source.skills?.capabilities)
        ? source.skills.capabilities
        : defaults.skills.capabilities,
      tools:
        source.skills && typeof source.skills.tools === "object"
          ? source.skills.tools
          : defaults.skills.tools,
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
  };
}
