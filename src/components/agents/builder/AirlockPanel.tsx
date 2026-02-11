import type { AirlockConfig, AirlockLevel } from "../../../types/airlock";

interface AirlockPanelProps {
  airlock: AirlockConfig;
  onChange: (airlock: AirlockConfig) => void;
}

const sectionTitleClass =
  "text-[10px] font-bold uppercase tracking-widest text-muted-foreground";
const inputClass =
  "w-full bg-card/40 hover:bg-card/60 backdrop-blur-md rounded-xl px-4 py-3 text-sm text-foreground placeholder:text-muted-foreground/50 leading-relaxed border border-border/20 focus:outline-none focus:border-primary/50 focus:ring-1 focus:ring-primary/20 transition-all shadow-sm";

const LEVELS: Array<{ level: AirlockLevel; title: string; tone: string }> = [
  { level: 0, title: "Safe", tone: "text-emerald-500" },
  { level: 1, title: "Sensitive", tone: "text-amber-500" },
  { level: 2, title: "Dangerous", tone: "text-red-500" },
];

export function AirlockPanel({ airlock, onChange }: AirlockPanelProps) {
  const allTools = Array.from(
    new Set([
      ...Object.keys(airlock.tool_levels),
      ...airlock.tool_policy.allow,
      ...airlock.tool_policy.deny,
    ]),
  ).sort((a, b) => a.localeCompare(b));

  const setToolLevel = (toolName: string, level: AirlockLevel) => {
    onChange({
      ...airlock,
      tool_levels: {
        ...airlock.tool_levels,
        [toolName]: level,
      },
    });
  };

  const toggleAllow = (toolName: string, enabled: boolean) => {
    const allow = enabled
      ? Array.from(new Set([...airlock.tool_policy.allow, toolName]))
      : airlock.tool_policy.allow.filter((item) => item !== toolName);

    // Mutual exclusion: remove from deny when allowing
    const deny = enabled
      ? airlock.tool_policy.deny.filter((item) => item !== toolName)
      : airlock.tool_policy.deny;

    onChange({
      ...airlock,
      tool_policy: {
        ...airlock.tool_policy,
        allow,
        deny,
      },
    });
  };

  const toggleDeny = (toolName: string, enabled: boolean) => {
    const deny = enabled
      ? Array.from(new Set([...airlock.tool_policy.deny, toolName]))
      : airlock.tool_policy.deny.filter((item) => item !== toolName);

    // Mutual exclusion: remove from allow when denying
    const allow = enabled
      ? airlock.tool_policy.allow.filter((item) => item !== toolName)
      : airlock.tool_policy.allow;

    onChange({
      ...airlock,
      tool_policy: {
        ...airlock.tool_policy,
        allow,
        deny,
      },
    });
  };

  const parseList = (value: string): string[] =>
    value
      .split(/\r?\n|,/)
      .map((item) => item.trim())
      .filter(Boolean);

  const joinList = (items: string[]) => items.join("\n");

  return (
    <div className="space-y-8 animate-appear">
      <div className="flex flex-col gap-1 border-b border-border/10 pb-6">
        <h3 className="text-2xl font-bold text-foreground tracking-tight">
          Airlock
        </h3>
        <p className="text-muted-foreground text-sm">
          Configure tool permissions, risk levels, scopes, and rate limits.
        </p>
      </div>

      <section className="space-y-4">
        <div className="flex items-center gap-3">
          <h4 className={sectionTitleClass}>Policy Mode</h4>
          <select
            value={airlock.tool_policy.mode}
            onChange={(e) =>
              onChange({
                ...airlock,
                tool_policy: {
                  ...airlock.tool_policy,
                  mode: e.target.value as "all" | "allowlist",
                },
              })
            }
            className={`${inputClass} max-w-[220px]`}
          >
            <option value="all">Allow all unless denied</option>
            <option value="allowlist">Allowlist only</option>
          </select>
        </div>

        <div className="space-y-2">
          {allTools.map((toolName) => {
            const level = airlock.tool_levels[toolName] ?? 1;
            const isAllowed = airlock.tool_policy.allow.includes(toolName);
            const isDenied = airlock.tool_policy.deny.includes(toolName);

            return (
              <div
                key={toolName}
                className="grid grid-cols-1 md:grid-cols-5 gap-3 rounded-2xl border border-border/20 bg-card/30 backdrop-blur-md p-3"
              >
                <div className="md:col-span-2 flex items-center">
                  <code className="text-xs text-foreground/90">{toolName}</code>
                </div>
                <div className="md:col-span-1">
                  <select
                    value={level}
                    onChange={(e) =>
                      setToolLevel(
                        toolName,
                        Number(e.target.value) as AirlockLevel,
                      )
                    }
                    className={inputClass}
                  >
                    {LEVELS.map((item) => (
                      <option key={item.level} value={item.level}>
                        L{item.level} {item.title}
                      </option>
                    ))}
                  </select>
                </div>
                <label className="md:col-span-1 text-xs text-muted-foreground flex items-center gap-2">
                  <input
                    type="checkbox"
                    checked={isAllowed}
                    onChange={(e) => toggleAllow(toolName, e.target.checked)}
                    className="accent-primary"
                  />
                  Allow
                </label>
                <label className="md:col-span-1 text-xs text-muted-foreground flex items-center gap-2">
                  <input
                    type="checkbox"
                    checked={isDenied}
                    onChange={(e) => toggleDeny(toolName, e.target.checked)}
                    className="accent-primary"
                  />
                  Deny
                </label>
              </div>
            );
          })}
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {LEVELS.map((item) => (
            <div
              key={item.level}
              className="rounded-xl border border-border/20 bg-card/20 p-3"
            >
              <p className={`text-sm font-semibold ${item.tone}`}>
                LEVEL {item.level} - {item.title}
              </p>
              <p className="text-xs text-muted-foreground mt-1">
                {Object.values(airlock.tool_levels).filter(
                  (level) => level === item.level,
                ).length || 0}{" "}
                tool(s)
              </p>
            </div>
          ))}
        </div>
      </section>

      <section className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <div className="space-y-2">
          <h4 className={sectionTitleClass}>Allowed Paths</h4>
          <textarea
            className={`${inputClass} resize-none`}
            rows={4}
            value={joinList(airlock.scopes.allowed_paths)}
            onChange={(e) =>
              onChange({
                ...airlock,
                scopes: {
                  ...airlock.scopes,
                  allowed_paths: parseList(e.target.value),
                },
              })
            }
          />
        </div>

        <div className="space-y-2">
          <h4 className={sectionTitleClass}>Blocked Paths</h4>
          <textarea
            className={`${inputClass} resize-none`}
            rows={4}
            value={joinList(airlock.scopes.blocked_paths)}
            onChange={(e) =>
              onChange({
                ...airlock,
                scopes: {
                  ...airlock.scopes,
                  blocked_paths: parseList(e.target.value),
                },
              })
            }
          />
        </div>

        <div className="space-y-2">
          <h4 className={sectionTitleClass}>Allowed Domains</h4>
          <textarea
            className={`${inputClass} resize-none`}
            rows={3}
            value={joinList(airlock.scopes.allowed_domains)}
            onChange={(e) =>
              onChange({
                ...airlock,
                scopes: {
                  ...airlock.scopes,
                  allowed_domains: parseList(e.target.value),
                },
              })
            }
          />
        </div>

        <div className="space-y-2">
          <h4 className={sectionTitleClass}>Blocked Domains</h4>
          <textarea
            className={`${inputClass} resize-none`}
            rows={3}
            value={joinList(airlock.scopes.blocked_domains)}
            onChange={(e) =>
              onChange({
                ...airlock,
                scopes: {
                  ...airlock.scopes,
                  blocked_domains: parseList(e.target.value),
                },
              })
            }
          />
        </div>
      </section>

      <section className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <div className="space-y-2">
          <h4 className={sectionTitleClass}>Requests / Minute</h4>
          <input
            type="number"
            min={1}
            value={airlock.rate_limits.max_requests_per_minute}
            onChange={(e) =>
              onChange({
                ...airlock,
                rate_limits: {
                  ...airlock.rate_limits,
                  max_requests_per_minute: Math.max(
                    1,
                    Number.parseInt(e.target.value || "1", 10),
                  ),
                },
              })
            }
            className={inputClass}
          />
        </div>

        <div className="space-y-2">
          <h4 className={sectionTitleClass}>Tokens / Day</h4>
          <input
            type="number"
            min={1}
            value={airlock.rate_limits.max_tokens_per_day}
            onChange={(e) =>
              onChange({
                ...airlock,
                rate_limits: {
                  ...airlock.rate_limits,
                  max_tokens_per_day: Math.max(
                    1,
                    Number.parseInt(e.target.value || "1", 10),
                  ),
                },
              })
            }
            className={inputClass}
          />
        </div>
      </section>
    </div>
  );
}
