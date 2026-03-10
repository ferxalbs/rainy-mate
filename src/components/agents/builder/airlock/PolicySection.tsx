import { memo, useMemo } from "react";
import { Button, Input, ListBox, Select, Switch } from "@heroui/react";
import type { AirlockConfig, AirlockLevel } from "../../../../types/airlock";
import { getToolSkill } from "../../../../constants/toolPolicy";
import { inputClass, LEVELS, sectionTitleClass } from "./constants";
const softButtonClass =
  "bg-background/85 dark:bg-background/35 border border-default-300/70 dark:border-white/15 text-foreground data-[hover=true]:bg-background/90 dark:data-[hover=true]:bg-background/45";
const selectTriggerClass =
  "h-11 bg-background/85 dark:bg-background/20 border border-default-300/70 dark:border-white/15 rounded-xl text-foreground";

interface PolicySectionProps {
  airlock: AirlockConfig;
  allTools: string[];
  activeTools: string[];
  customToolInput: string;
  getToolLevel: (toolName: string) => AirlockLevel;
  onModeChange: (mode: "all" | "allowlist") => void;
  onAllowAllTools: () => void;
  onAllowSafeOnly: () => void;
  onAllowSafeSensitive: () => void;
  onClearAllowlist: () => void;
  onSetToolLevel: (toolName: string, level: AirlockLevel) => void;
  onSetAllowed: (toolName: string, enabled: boolean) => void;
  onSetDenied: (toolName: string, enabled: boolean) => void;
  onCustomToolInputChange: (value: string) => void;
  onAddCustomTool: () => void;
}

interface ToolPermissionRowProps {
  toolName: string;
  mode: "all" | "allowlist";
  level: AirlockLevel;
  isAllowed: boolean;
  isDenied: boolean;
  onSetToolLevel: (toolName: string, level: AirlockLevel) => void;
  onSetAllowed: (toolName: string, enabled: boolean) => void;
  onSetDenied: (toolName: string, enabled: boolean) => void;
}

const selectionToValue = (selection: unknown): string | null => {
  if (typeof selection === "string") return selection;
  if (selection instanceof Set) {
    const first = selection.values().next().value;
    return typeof first === "string" ? first : null;
  }
  return null;
};

const ToolPermissionRow = memo(function ToolPermissionRow({
  toolName,
  mode,
  level,
  isAllowed,
  isDenied,
  onSetToolLevel,
  onSetAllowed,
  onSetDenied,
}: ToolPermissionRowProps) {
  const levelMeta = LEVELS.find((item) => item.level === level)!;
  const skill = getToolSkill(toolName);

  return (
    <div className="grid grid-cols-1 md:grid-cols-7 gap-3 rounded-2xl border border-border/20 bg-card/30 backdrop-blur-md p-3 items-center">
      <div className="md:col-span-2 min-w-0">
        <code className="text-xs text-foreground/90">{toolName}</code>
        <div className="mt-1 text-[10px] uppercase tracking-wider text-muted-foreground">
          {skill}
        </div>
      </div>

      <div className="md:col-span-2">
        <Select
          className={inputClass}
          selectedKey={String(level)}
          onSelectionChange={(selection) => {
            const value = selectionToValue(selection);
            if (!value) return;
            onSetToolLevel(toolName, Number(value) as AirlockLevel);
          }}
        >
          <Select.Trigger className={selectTriggerClass}>
            <Select.Value className="text-foreground" />
            <Select.Indicator />
          </Select.Trigger>
          <Select.Popover className="bg-background/95 dark:bg-background/35 border border-default-200/70 dark:border-white/15 backdrop-blur-xl">
            <ListBox className="bg-transparent">
              {LEVELS.map((item) => (
                <ListBox.Item
                  key={String(item.level)}
                  id={String(item.level)}
                  textValue={`L${item.level} ${item.title}`}
                >
                  L{item.level} {item.title}
                  <ListBox.ItemIndicator />
                </ListBox.Item>
              ))}
            </ListBox>
          </Select.Popover>
        </Select>
        <p className={`text-[11px] mt-1 ${levelMeta.tone}`}>{levelMeta.modalBehavior}</p>
      </div>

      {mode === "allowlist" ? (
        <div className="md:col-span-3 flex justify-start md:justify-end">
          <Switch isSelected={isAllowed} onChange={(enabled) => onSetAllowed(toolName, enabled)}>
            <Switch.Control>
              <Switch.Thumb />
            </Switch.Control>
            Allow this tool
          </Switch>
        </div>
      ) : (
        <div className="md:col-span-3 flex justify-start md:justify-end">
          <Switch isSelected={isDenied} onChange={(enabled) => onSetDenied(toolName, enabled)}>
            <Switch.Control>
              <Switch.Thumb />
            </Switch.Control>
            Block this tool (deny)
          </Switch>
        </div>
      )}
    </div>
  );
});

export function PolicySection({
  airlock,
  allTools,
  activeTools,
  customToolInput,
  getToolLevel,
  onModeChange,
  onAllowAllTools,
  onAllowSafeOnly,
  onAllowSafeSensitive,
  onClearAllowlist,
  onSetToolLevel,
  onSetAllowed,
  onSetDenied,
  onCustomToolInputChange,
  onAddCustomTool,
}: PolicySectionProps) {
  const allowSet = useMemo(
    () => new Set(airlock.tool_policy.allow),
    [airlock.tool_policy.allow],
  );
  const denySet = useMemo(
    () => new Set(airlock.tool_policy.deny),
    [airlock.tool_policy.deny],
  );

  const activeLevelCounts = useMemo(() => {
    return LEVELS.reduce<Record<number, number>>((acc, item) => {
      acc[item.level] = activeTools.filter((tool) => getToolLevel(tool) === item.level).length;
      return acc;
    }, {});
  }, [activeTools, getToolLevel]);

  return (
    <section className="space-y-4">
      <div className="flex flex-wrap items-center gap-3">
        <h4 className={sectionTitleClass}>Policy Mode</h4>
        <Select
          className={`${inputClass} max-w-[260px]`}
          selectedKey={airlock.tool_policy.mode}
          onSelectionChange={(selection) => {
            const value = selectionToValue(selection);
            if (!value) return;
            onModeChange(value as "all" | "allowlist");
          }}
        >
          <Select.Trigger className={selectTriggerClass}>
            <Select.Value className="text-foreground" />
            <Select.Indicator />
          </Select.Trigger>
          <Select.Popover className="bg-background/95 dark:bg-background/35 border border-default-200/70 dark:border-white/15 backdrop-blur-xl">
            <ListBox className="bg-transparent">
              <ListBox.Item id="all" textValue="Allow all unless denied">
                Allow all unless denied
                <ListBox.ItemIndicator />
              </ListBox.Item>
              <ListBox.Item id="allowlist" textValue="Allowlist only (manual)">
                Allowlist only (manual)
                <ListBox.ItemIndicator />
              </ListBox.Item>
            </ListBox>
          </Select.Popover>
        </Select>
        <span className="text-xs text-muted-foreground">
          Active tools: <strong>{activeTools.length}</strong>
        </span>
      </div>

      {airlock.tool_policy.mode === "allowlist" && (
        <div className="rounded-2xl border border-primary/20 bg-primary/[0.05] p-4 space-y-3">
          <p className="text-sm text-foreground">
            Manual allowlist is enabled. Select exactly which tools this agent can execute.
          </p>
          <div className="flex flex-wrap gap-2">
            <Button size="sm" variant="secondary" className={softButtonClass} onPress={onAllowAllTools}>
              Allow all known tools
            </Button>
            <Button size="sm" variant="secondary" className={softButtonClass} onPress={onAllowSafeOnly}>
              Allow L0 only
            </Button>
            <Button size="sm" variant="secondary" className={softButtonClass} onPress={onAllowSafeSensitive}>
              Allow L0 + L1
            </Button>
            <Button size="sm" variant="secondary" className={softButtonClass} onPress={onClearAllowlist}>
              Clear allowlist
            </Button>
          </div>
        </div>
      )}

      <div className="space-y-2">
        <h4 className={sectionTitleClass}>Tool Permissions</h4>
        <p className="text-xs text-muted-foreground">
          Levels control Airlock prompts: <span className="text-emerald-500">L0 auto</span>,{" "}
          <span className="text-amber-500">L1 asks approval</span>,{" "}
          <span className="text-red-500">L2 requires explicit approval</span>.
        </p>
        {allTools.map((toolName) => (
          <ToolPermissionRow
            key={toolName}
            toolName={toolName}
            mode={airlock.tool_policy.mode}
            level={getToolLevel(toolName)}
            isAllowed={allowSet.has(toolName)}
            isDenied={denySet.has(toolName)}
            onSetToolLevel={onSetToolLevel}
            onSetAllowed={onSetAllowed}
            onSetDenied={onSetDenied}
          />
        ))}
      </div>

      <div className="rounded-2xl border border-border/20 bg-card/20 p-4 space-y-3">
        <h4 className={sectionTitleClass}>Custom Tool</h4>
        <div className="flex gap-2">
          <Input
            value={customToolInput}
            onChange={(e) => onCustomToolInputChange(e.target.value)}
            placeholder="tool_name"
            className={inputClass}
          />
          <Button variant="secondary" className={softButtonClass} onPress={onAddCustomTool}>
            Add
          </Button>
        </div>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        {LEVELS.map((item) => (
          <div key={item.level} className="rounded-xl border border-border/20 bg-card/20 p-3">
            <p className={`text-sm font-semibold ${item.tone}`}>
              LEVEL {item.level} - {item.title}
            </p>
            <p className="text-xs text-muted-foreground mt-1">
              {activeLevelCounts[item.level] ?? 0} active tool(s)
            </p>
          </div>
        ))}
      </div>
    </section>
  );
}
