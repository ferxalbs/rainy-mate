export type AirlockLevel = 0 | 1 | 2;

export interface AirlockToolPolicy {
  mode: "all" | "allowlist";
  allow: string[];
  deny: string[];
}

export interface AirlockScopes {
  allowed_paths: string[];
  blocked_paths: string[];
  allowed_domains: string[];
  blocked_domains: string[];
}

export interface AirlockRateLimits {
  max_requests_per_minute: number;
  max_tokens_per_day: number;
}

export interface AirlockConfig {
  tool_policy: AirlockToolPolicy;
  tool_levels: Record<string, AirlockLevel>;
  scopes: AirlockScopes;
  rate_limits: AirlockRateLimits;
}
