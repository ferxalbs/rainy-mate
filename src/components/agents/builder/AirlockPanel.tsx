import { useMemo, useState } from "react";
import type { AirlockConfig, AirlockLevel } from "../../../types/airlock";
import {
  KNOWN_TOOL_NAMES,
  getToolAirlockLevel,
  getToolSkill,
} from "../../../constants/toolPolicy";

interface AirlockPanelProps {
  airlock: AirlockConfig;
  onChange: (airlock: AirlockConfig) => void;
}

const sectionTitleClass =
  "text-[10px] font-bold uppercase tracking-widest text-muted-foreground";
const inputClass =
  "w-full bg-card/40 hover:bg-card/60 backdrop-blur-md rounded-xl px-4 py-3 text-sm text-foreground placeholder:text-muted-foreground/50 leading-relaxed border border-border/20 focus:outline-none focus:border-primary/50 focus:ring-1 focus:ring-primary/20 transition-all shadow-sm";

const LEVELS: Array<{
  level: AirlockLevel;
  title: string;
  tone: string;
  modalBehavior: string;
}> = [
  {
    level: 0,
    title: "Safe",
    tone: "text-emerald-500",
    modalBehavior: "Auto-approved (no modal)",
  },
  {
    level: 1,
    title: "Sensitive",
    tone: "text-amber-500",
    modalBehavior: "Approval modal (notification gate)",
  },
  {
    level: 2,
    title: "Dangerous",
    tone: "text-red-500",
    modalBehavior: "Explicit approval modal required",
  },
];

function parseList(value: string): string[] {
  return value
    .split(/\r?\n|,/)
    .map((item) => item.trim())
    .filter(Boolean);
}

function joinList(items: string[]) {
  return items.join("\n");
}

export function AirlockPanel({ airlock, onChange }: AirlockPanelProps) {
  const [customToolInput, setCustomToolInput] = useState("");

  const allTools = useMemo(
    () =>
      Array.from(
        new Set([
          ...KNOWN_TOOL_NAMES,
          ...Object.keys(airlock.tool_levels),
          ...airlock.tool_policy.allow,
          ...airlock.tool_policy.deny,
        ]),
      ).sort((a, b) => a.localeCompare(b)),
    [airlock.tool_levels, airlock.tool_policy.allow, airlock.tool_policy.deny],
  );

  const ensureLevel = (toolName: string, currentLevel?: AirlockLevel) =>
    currentLevel ?? getToolAirlockLevel(toolName);

  const setToolLevel = (toolName: string, level: AirlockLevel) => {
    onChange({
      ...airlock,
      tool_levels: {
        ...airlock.tool_levels,
        [toolName]: level,
      },
    });
  };

  const setAllowed = (toolName: string, enabled: boolean) => {
    const allow = enabled
      ? Array.from(new Set([...airlock.tool_policy.allow, toolName]))
      : airlock.tool_policy.allow.filter((item) => item !== toolName);

    const deny = airlock.tool_policy.deny.filter((item) => item !== toolName);

    onChange({
      ...airlock,
      tool_policy: {
        ...airlock.tool_policy,
        allow,
        deny,
      },
    });
  };

  const setDenied = (toolName: string, enabled: boolean) => {
    const deny = enabled
      ? Array.from(new Set([...airlock.tool_policy.deny, toolName]))
      : airlock.tool_policy.deny.filter((item) => item !== toolName);

    const allow = enabled
      ? airlock.tool_policy.allow.filter((item) => item !== toolName)
      : airlock.tool_policy.allow;

    onChange({
      ...airlock,
      tool_policy: {
        ...airlock.tool_policy,
        deny,
        allow,
      },
    });
  };

  const addCustomTool = () => {
    const toolName = customToolInput.trim();
    if (!toolName) return;

    const withLevel = {
      ...airlock.tool_levels,
      [toolName]: ensureLevel(toolName, airlock.tool_levels[toolName]),
    };
    const withAllow = Array.from(new Set([...airlock.tool_policy.allow, toolName]));

    onChange({
      ...airlock,
      tool_levels: withLevel,
      tool_policy: {
        ...airlock.tool_policy,
        allow: withAllow,
      },
    });

    setCustomToolInput("");
  };

  const applyAllowPreset = (predicate: (level: AirlockLevel) => boolean) => {
    const selected = allTools.filter((toolName) =>
      predicate(ensureLevel(toolName, airlock.tool_levels[toolName])),
    );

    onChange({
      ...airlock,
      tool_policy: {
        ...airlock.tool_policy,
        allow: selected,
        deny: airlock.tool_policy.deny.filter((tool) => !selected.includes(tool)),
      },
    });
  };

  const activeTools =
    airlock.tool_policy.mode === "allowlist"
      ? airlock.tool_policy.allow
      : allTools.filter((toolName) => !airlock.tool_policy.deny.includes(toolName));

  return (
    <div className="space-y-8 animate-appear">
      <div className="flex flex-col gap-1 border-b border-border/10 pb-6">
        <h3 className="text-2xl font-bold text-foreground tracking-tight">
          Airlock
        </h3>
        <p className="text-muted-foreground text-sm">
          Define exactly which tools run, at which risk level, and how approval
          modals are triggered.
        </p>
      </div>

      <section className="space-y-4">
        <div className="flex flex-wrap items-center gap-3">
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
            className={`${inputClass} max-w-[260px]`}
          >
            <option value="all">Allow all unless denied</option>
            <option value="allowlist">Allowlist only (manual)</option>
          </select>
          <span className="text-xs text-muted-foreground">
            Active tools: <strong>{activeTools.length}</strong>
          </span>
        </div>

        {airlock.tool_policy.mode === "allowlist" && (
          <div className="rounded-2xl border border-primary/20 bg-primary/[0.05] p-4 space-y-3">
            <p className="text-sm text-foreground">
              Manual allowlist is enabled. Select exactly which tools this agent
              can execute.
            </p>
            <div className="flex flex-wrap gap-2">
              <button
                onClick={() => applyAllowPreset(() => true)}
                className="px-3 py-1.5 rounded-full text-xs border border-border/30 hover:border-primary/50 hover:text-primary transition-colors"
              >
                Allow all known tools
              </button>
              <button
                onClick={() => applyAllowPreset((level) => level === 0)}
                className="px-3 py-1.5 rounded-full text-xs border border-border/30 hover:border-emerald-500/50 hover:text-emerald-500 transition-colors"
              >
                Allow L0 only
              </button>
              <button
                onClick={() => applyAllowPreset((level) => level <= 1)}
                className="px-3 py-1.5 rounded-full text-xs border border-border/30 hover:border-amber-500/50 hover:text-amber-500 transition-colors"
              >
                Allow L0 + L1
              </button>
              <button
                onClick={() => applyAllowPreset(() => false)}
                className="px-3 py-1.5 rounded-full text-xs border border-border/30 hover:border-red-500/50 hover:text-red-500 transition-colors"
              >
                Clear allowlist
              </button>
            </div>
          </div>
        )}

        <div className="space-y-2">
          <h4 className={sectionTitleClass}>Tool Permissions</h4>
          <p className="text-xs text-muted-foreground">
            Levels control Airlock prompts:
            {" "}
            <span className="text-emerald-500">L0 auto</span>,{" "}
            <span className="text-amber-500">L1 asks approval</span>,{" "}
            <span className="text-red-500">L2 requires explicit approval</span>.
          </p>
          {allTools.map((toolName) => {
            const level = ensureLevel(toolName, airlock.tool_levels[toolName]);
            const levelMeta = LEVELS.find((item) => item.level === level)!;
            const skill = getToolSkill(toolName);
            const isAllowed = airlock.tool_policy.allow.includes(toolName);
            const isDenied = airlock.tool_policy.deny.includes(toolName);

            return (
              <div
                key={toolName}
                className="grid grid-cols-1 md:grid-cols-7 gap-3 rounded-2xl border border-border/20 bg-card/30 backdrop-blur-md p-3 items-center"
              >
                <div className="md:col-span-2 min-w-0">
                  <code className="text-xs text-foreground/90">{toolName}</code>
                  <div className="mt-1 text-[10px] uppercase tracking-wider text-muted-foreground">
                    {skill}
                  </div>
                </div>

                <div className="md:col-span-2">
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
                  <p className={`text-[11px] mt-1 ${levelMeta.tone}`}>
                    {levelMeta.modalBehavior}
                  </p>
                </div>

                {airlock.tool_policy.mode === "allowlist" ? (
                  <label className="md:col-span-3 text-xs text-muted-foreground flex items-center gap-2 justify-start md:justify-end">
                    <input
                      type="checkbox"
                      checked={isAllowed}
                      onChange={(e) => setAllowed(toolName, e.target.checked)}
                      className="accent-primary"
                    />
                    Allow this tool
                  </label>
                ) : (
                  <label className="md:col-span-3 text-xs text-muted-foreground flex items-center gap-2 justify-start md:justify-end">
                    <input
                      type="checkbox"
                      checked={isDenied}
                      onChange={(e) => setDenied(toolName, e.target.checked)}
                      className="accent-primary"
                    />
                    Block this tool (deny)
                  </label>
                )}
              </div>
            );
          })}
        </div>

        <div className="rounded-2xl border border-border/20 bg-card/20 p-4 space-y-3">
          <h4 className={sectionTitleClass}>Custom Tool</h4>
          <div className="flex gap-2">
            <input
              value={customToolInput}
              onChange={(e) => setCustomToolInput(e.target.value)}
              placeholder="tool_name"
              className={inputClass}
            />
            <button
              onClick={addCustomTool}
              className="px-4 rounded-xl border border-border/30 text-sm hover:border-primary/50 hover:text-primary transition-colors"
            >
              Add
            </button>
          </div>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          {LEVELS.map((item) => (
            <div
              key={item.level}
              className="rounded-xl border border-border/20 bg-card/20 p-3"
            >
              <p className={`text-sm font-semibold ${item.tone}`}>
                LEVEL {item.level} - {item.title}
              </p>
              <p className="text-xs text-muted-foreground mt-1">
                {
                  activeTools.filter(
                    (tool) => ensureLevel(tool, airlock.tool_levels[tool]) === item.level,
                  ).length
                }{" "}
                active tool(s)
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
