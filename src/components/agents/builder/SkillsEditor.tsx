import { useEffect, useMemo, useState } from "react";
import {
  Button,
  Input,
  ListBox,
  Select,
  Switch,
  TextArea,
} from "@heroui/react";
import {
  Cpu,
  RefreshCw,
  Sparkles,
  TriangleAlert,
  Wand2,
} from "lucide-react";
import { toast } from "sonner";
import type {
  AgentSkills,
  PromptSkillBinding,
  SkillBehavior,
  SkillWorkflow,
  ToolPreference,
} from "../../../types/agent-spec";
import {
  listPromptSkills,
  refreshPromptSkillSnapshot,
  setPromptSkillAllAgentsEnabled,
  type DiscoveredPromptSkill,
} from "../../../services/tauri";

interface SkillsEditorProps {
  skills: AgentSkills;
  workspacePath?: string;
  onChange: (skills: AgentSkills) => void;
}

const sectionTitleClass =
  "text-[10px] font-bold uppercase tracking-widest text-muted-foreground";
const controlClass =
  "w-full bg-background/85 dark:bg-background/20 border-default-300/70 dark:border-white/15 data-[hover=true]:bg-background/90 dark:data-[hover=true]:bg-background/35 shadow-sm";
const softButtonClass =
  "bg-background/85 dark:bg-background/35 border border-default-300/70 dark:border-white/15 text-foreground data-[hover=true]:bg-background/90 dark:data-[hover=true]:bg-background/45";
const selectTriggerClass =
  "h-11 bg-background/85 dark:bg-background/20 border border-default-300/70 dark:border-white/15 rounded-xl text-foreground";

const selectionToValue = (selection: unknown): string | null => {
  if (typeof selection === "string") return selection;
  if (selection instanceof Set) {
    const first = selection.values().next().value;
    return typeof first === "string" ? first : null;
  }
  return null;
};

function createWorkflow(): SkillWorkflow {
  return {
    id: crypto.randomUUID(),
    name: "",
    description: "",
    trigger: "",
    steps: "",
    enabled: true,
  };
}

function createToolPreference(): ToolPreference {
  return {
    tool_name: "",
    priority: "prefer",
    context: "",
  };
}

function createBehavior(): SkillBehavior {
  return {
    id: crypto.randomUUID(),
    name: "",
    instruction: "",
    enabled: true,
  };
}

function formatScope(scope: DiscoveredPromptSkill["scope"]) {
  if (scope === "mate_managed") return "MaTE";
  if (scope === "global") return "Global";
  return "Project";
}

function createBinding(skill: DiscoveredPromptSkill): PromptSkillBinding {
  return {
    id: skill.id,
    name: skill.name,
    description: skill.description,
    content: skill.bodyMarkdown,
    source_path: skill.sourcePath,
    scope: skill.scope,
    source_hash: skill.sourceHash,
    enabled: true,
    last_synced_at: Math.floor(Date.now() / 1000),
  };
}

export function SkillsEditor({
  skills,
  workspacePath,
  onChange,
}: SkillsEditorProps) {
  const [promptSkills, setPromptSkills] = useState<DiscoveredPromptSkill[]>([]);
  const [loadingPromptSkills, setLoadingPromptSkills] = useState(false);
  const [busySourcePath, setBusySourcePath] = useState<string | null>(null);

  const updateWorkflows = (workflows: SkillWorkflow[]) => {
    onChange({ ...skills, workflows });
  };

  const updateToolPreferences = (tool_preferences: ToolPreference[]) => {
    onChange({ ...skills, tool_preferences });
  };

  const updateBehaviors = (behaviors: SkillBehavior[]) => {
    onChange({ ...skills, behaviors });
  };

  const updatePromptBindings = (prompt_skills: PromptSkillBinding[]) => {
    onChange({ ...skills, prompt_skills });
  };

  useEffect(() => {
    let cancelled = false;
    setLoadingPromptSkills(true);
    listPromptSkills(workspacePath)
      .then((items) => {
        if (!cancelled) {
          setPromptSkills(items);
        }
      })
      .catch((error) => {
        console.error("Failed to load prompt skills:", error);
        if (!cancelled) {
          toast.error("Failed to load detected prompt skills");
        }
      })
      .finally(() => {
        if (!cancelled) {
          setLoadingPromptSkills(false);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [workspacePath]);

  const promptBindingsByPath = useMemo(
    () =>
      new Map(
        skills.prompt_skills
          .filter((binding) => binding.enabled)
          .map((binding) => [binding.source_path, binding]),
      ),
    [skills.prompt_skills],
  );

  const handleTogglePromptSkill = (
    skill: DiscoveredPromptSkill,
    enabled: boolean,
  ) => {
    if (!skill.valid) return;
    if (enabled) {
      const next = [...skills.prompt_skills.filter((item) => item.source_path !== skill.sourcePath), createBinding(skill)];
      updatePromptBindings(next);
      return;
    }

    updatePromptBindings(
      skills.prompt_skills.filter((item) => item.source_path !== skill.sourcePath),
    );
  };

  const handleToggleAllAgents = async (
    skill: DiscoveredPromptSkill,
    enabled: boolean,
  ) => {
    setBusySourcePath(skill.sourcePath);
    try {
      await setPromptSkillAllAgentsEnabled({
        sourcePath: skill.sourcePath,
        enabled,
        workspacePath,
      });
      setPromptSkills((prev) =>
        prev.map((item) =>
          item.sourcePath === skill.sourcePath
            ? { ...item, allAgentsEnabled: enabled }
            : item,
        ),
      );
    } catch (error) {
      console.error("Failed to update prompt skill visibility:", error);
      toast.error("Failed to update all-agents prompt skill toggle");
    } finally {
      setBusySourcePath(null);
    }
  };

  const handleRefreshPromptSkill = async (skill: DiscoveredPromptSkill) => {
    setBusySourcePath(skill.sourcePath);
    try {
      const refreshed = await refreshPromptSkillSnapshot({
        sourcePath: skill.sourcePath,
        workspacePath,
      });
      updatePromptBindings([
        ...skills.prompt_skills.filter((item) => item.source_path !== skill.sourcePath),
        refreshed,
      ]);
      setPromptSkills((prev) =>
        prev.map((item) =>
          item.sourcePath === skill.sourcePath
            ? {
                ...item,
                bodyMarkdown: refreshed.content,
                sourceHash: refreshed.source_hash,
              }
            : item,
        ),
      );
      toast.success(`Refreshed "${skill.name}" from source`);
    } catch (error) {
      console.error("Failed to refresh prompt skill:", error);
      toast.error("Failed to refresh prompt skill from source");
    } finally {
      setBusySourcePath(null);
    }
  };

  return (
    <div className="space-y-8 animate-appear">
      <div className="relative overflow-hidden rounded-2xl border border-border/20 bg-card/40 backdrop-blur-xl p-5">
        <div className="absolute -top-20 right-[-60px] w-[280px] h-[280px] rounded-full bg-primary/10 blur-[85px] pointer-events-none" />
        <div className="absolute -bottom-24 left-[-80px] w-[260px] h-[260px] rounded-full bg-foreground/[0.04] blur-[90px] pointer-events-none" />
        <div className="relative z-10 flex flex-col gap-1">
          <h3 className="text-2xl font-bold text-foreground tracking-tight flex items-center gap-2">
            <Cpu className="size-5 text-primary" />
            Skills
          </h3>
          <p className="text-muted-foreground text-sm">
            Combine detected prompt skills with workflows, tool preferences, and behavior rules.
          </p>
        </div>
      </div>

      <section className="space-y-4 rounded-2xl border border-border/20 bg-card/35 backdrop-blur-md p-5">
        <div className="flex items-center justify-between gap-4">
          <div>
            <h4 className={sectionTitleClass}>Detected Prompt Skills</h4>
            <p className="text-sm text-muted-foreground mt-1">
              `skills.sh`-compatible `SKILL.md` folders detected across the current project and global paths.
            </p>
          </div>
          <Button
            size="sm"
            variant="secondary"
            className={softButtonClass}
            onPress={() => {
              setLoadingPromptSkills(true);
              listPromptSkills(workspacePath)
                .then(setPromptSkills)
                .catch((error) => {
                  console.error("Failed to reload prompt skills:", error);
                  toast.error("Failed to refresh detected prompt skills");
                })
                .finally(() => setLoadingPromptSkills(false));
            }}
          >
            <RefreshCw className={`size-3.5 ${loadingPromptSkills ? "animate-spin" : ""}`} />
            Refresh
          </Button>
        </div>

        {!workspacePath && (
          <div className="rounded-xl border border-dashed border-border/30 bg-card/20 px-4 py-3 text-sm text-muted-foreground">
            Select a workspace to detect project prompt skills. Global prompt skills still load.
          </div>
        )}

        {loadingPromptSkills ? (
          <div className="rounded-xl border border-dashed border-border/30 bg-card/20 px-4 py-6 text-sm text-muted-foreground">
            Detecting prompt skills…
          </div>
        ) : promptSkills.length === 0 ? (
          <div className="rounded-xl border border-dashed border-border/30 bg-card/20 px-4 py-6 text-sm text-muted-foreground">
            No `SKILL.md` prompt skills detected yet.
          </div>
        ) : (
          <div className="space-y-3">
            {promptSkills.map((skill) => {
              const binding = promptBindingsByPath.get(skill.sourcePath);
              const isOutOfSync =
                !!binding && binding.source_hash !== skill.sourceHash;
              const isBusy = busySourcePath === skill.sourcePath;

              return (
                <div
                  key={skill.sourcePath}
                  className="rounded-2xl border border-border/20 bg-card/30 backdrop-blur-md p-4"
                >
                  <div className="flex flex-col gap-3 md:flex-row md:items-start md:justify-between">
                    <div className="space-y-2 min-w-0">
                      <div className="flex flex-wrap items-center gap-2">
                        <span className="text-sm font-semibold text-foreground">
                          {skill.name}
                        </span>
                        <span className="rounded-full border border-border/30 bg-background/50 px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wide text-muted-foreground">
                          {formatScope(skill.scope)}
                        </span>
                        {skill.sourceKind === "plugin_manifest" && (
                          <span className="rounded-full border border-primary/20 bg-primary/10 px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wide text-primary">
                            Plugin
                          </span>
                        )}
                        {skill.allAgentsEnabled && (
                          <span className="rounded-full border border-emerald-500/20 bg-emerald-500/10 px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wide text-emerald-400">
                            All Agents
                          </span>
                        )}
                        {isOutOfSync && (
                          <span className="rounded-full border border-amber-500/20 bg-amber-500/10 px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wide text-amber-400">
                            Out of Sync
                          </span>
                        )}
                        {!skill.valid && (
                          <span className="rounded-full border border-red-500/20 bg-red-500/10 px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wide text-red-400">
                            Invalid
                          </span>
                        )}
                      </div>
                      <p className="text-sm text-muted-foreground">
                        {skill.description || "No description"}
                      </p>
                      <div className="space-y-1 text-xs text-muted-foreground/80">
                        <div className="font-mono break-all">{skill.sourcePath}</div>
                        {(skill.scripts.length > 0 || skill.references.length > 0) && (
                          <div className="flex flex-wrap gap-3">
                            {skill.scripts.length > 0 && (
                              <span>{skill.scripts.length} scripts</span>
                            )}
                            {skill.references.length > 0 && (
                              <span>{skill.references.length} references</span>
                            )}
                          </div>
                        )}
                        {!skill.valid && skill.parseError && (
                          <div className="flex items-center gap-2 text-red-400">
                            <TriangleAlert className="size-3.5" />
                            <span>{skill.parseError}</span>
                          </div>
                        )}
                      </div>
                    </div>

                    <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 min-w-[260px]">
                      <div className="rounded-xl border border-border/20 bg-background/40 px-3 py-2">
                        <div className="flex items-center justify-between gap-3">
                          <div className="flex items-center gap-2">
                            <Sparkles className="size-3.5 text-primary" />
                            <span className="text-xs font-medium text-foreground">
                              This Agent
                            </span>
                          </div>
                          <Switch
                            size="sm"
                            isDisabled={!skill.valid}
                            isSelected={!!binding}
                            onChange={(enabled) =>
                              handleTogglePromptSkill(skill, enabled)
                            }
                          />
                        </div>
                      </div>

                      <div className="rounded-xl border border-border/20 bg-background/40 px-3 py-2">
                        <div className="flex items-center justify-between gap-3">
                          <div className="flex items-center gap-2">
                            <Wand2 className="size-3.5 text-primary" />
                            <span className="text-xs font-medium text-foreground">
                              All Agents
                            </span>
                          </div>
                          <Switch
                            size="sm"
                            isDisabled={!skill.valid || isBusy}
                            isSelected={skill.allAgentsEnabled}
                            onChange={(enabled) =>
                              void handleToggleAllAgents(skill, enabled)
                            }
                          />
                        </div>
                      </div>

                      {isOutOfSync && (
                        <Button
                          size="sm"
                          variant="secondary"
                          className={`sm:col-span-2 ${softButtonClass}`}
                          isDisabled={isBusy}
                          onPress={() => void handleRefreshPromptSkill(skill)}
                        >
                          <RefreshCw className={`size-3.5 ${isBusy ? "animate-spin" : ""}`} />
                          Refresh Snapshot
                        </Button>
                      )}
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </section>

      <section className="space-y-4 rounded-2xl border border-border/20 bg-card/35 backdrop-blur-md p-5">
        <div className="flex items-center justify-between">
          <h4 className={sectionTitleClass}>Workflows</h4>
          <Button
            size="sm"
            variant="secondary"
            className={softButtonClass}
            onPress={() => updateWorkflows([...skills.workflows, createWorkflow()])}
          >
            + Add Workflow
          </Button>
        </div>

        {skills.workflows.length === 0 && (
          <div className="p-4 rounded-xl border border-dashed border-border/30 text-sm text-muted-foreground bg-card/20">
            No workflows yet.
          </div>
        )}

        {skills.workflows.map((workflow, index) => (
          <div
            key={workflow.id}
            className="space-y-3 rounded-2xl border border-border/20 bg-card/30 backdrop-blur-md p-4"
          >
            <div className="flex items-center justify-between gap-3">
              <span className="text-xs font-mono text-muted-foreground">
                Workflow #{index + 1}
              </span>
              <div className="flex items-center gap-2">
                <Switch
                  size="sm"
                  isSelected={workflow.enabled}
                  onChange={(enabled) =>
                    updateWorkflows(
                      skills.workflows.map((item) =>
                        item.id === workflow.id ? { ...item, enabled } : item,
                      ),
                    )
                  }
                >
                  <Switch.Control>
                    <Switch.Thumb />
                  </Switch.Control>
                  Enabled
                </Switch>
                <Button
                  size="sm"
                  variant="ghost"
                  className={softButtonClass}
                  onPress={() =>
                    updateWorkflows(
                      skills.workflows.filter((item) => item.id !== workflow.id),
                    )
                  }
                >
                  Remove
                </Button>
              </div>
            </div>

            <Input
              placeholder="Name"
              value={workflow.name}
              onChange={(e) =>
                updateWorkflows(
                  skills.workflows.map((item) =>
                    item.id === workflow.id ? { ...item, name: e.target.value } : item,
                  ),
                )
              }
              className={controlClass}
            />

            <Input
              placeholder="Trigger (e.g. user asks for code review)"
              value={workflow.trigger}
              onChange={(e) =>
                updateWorkflows(
                  skills.workflows.map((item) =>
                    item.id === workflow.id ? { ...item, trigger: e.target.value } : item,
                  ),
                )
              }
              className={controlClass}
            />

            <TextArea
              placeholder="Description"
              value={workflow.description}
              onChange={(e) =>
                updateWorkflows(
                  skills.workflows.map((item) =>
                    item.id === workflow.id
                      ? { ...item, description: e.target.value }
                      : item,
                  ),
                )
              }
              className={controlClass}
              rows={2}
            />

            <TextArea
              placeholder="Workflow steps (markdown)"
              value={workflow.steps}
              onChange={(e) =>
                updateWorkflows(
                  skills.workflows.map((item) =>
                    item.id === workflow.id ? { ...item, steps: e.target.value } : item,
                  ),
                )
              }
              className={`${controlClass} font-mono text-xs`}
              rows={5}
            />
          </div>
        ))}
      </section>

      <section className="space-y-4 rounded-2xl border border-border/20 bg-card/35 backdrop-blur-md p-5">
        <div className="flex items-center justify-between">
          <h4 className={sectionTitleClass}>Tool Preferences</h4>
          <Button
            size="sm"
            variant="secondary"
            className={softButtonClass}
            onPress={() =>
              updateToolPreferences([...skills.tool_preferences, createToolPreference()])
            }
          >
            + Add Tool Rule
          </Button>
        </div>

        {skills.tool_preferences.length === 0 && (
          <div className="p-4 rounded-xl border border-dashed border-border/30 text-sm text-muted-foreground bg-card/20">
            No tool preferences yet.
          </div>
        )}

        {skills.tool_preferences.map((preference, index) => (
          <div
            key={`${preference.tool_name}-${index}`}
            className="grid grid-cols-1 md:grid-cols-4 gap-3 rounded-2xl border border-border/20 bg-card/30 backdrop-blur-md p-4"
          >
            <Input
              placeholder="Tool name"
              value={preference.tool_name}
              onChange={(e) =>
                updateToolPreferences(
                  skills.tool_preferences.map((item, itemIndex) =>
                    itemIndex === index
                      ? { ...item, tool_name: e.target.value }
                      : item,
                  ),
                )
              }
              className={`${controlClass} md:col-span-1`}
            />

            <Select
              className={`${controlClass} md:col-span-1`}
              selectedKey={preference.priority}
              onSelectionChange={(selection) => {
                const value = selectionToValue(selection);
                if (!value) return;
                updateToolPreferences(
                  skills.tool_preferences.map((item, itemIndex) =>
                    itemIndex === index
                      ? {
                          ...item,
                          priority: value as "prefer" | "avoid" | "never",
                        }
                      : item,
                  ),
                );
              }}
            >
              <Select.Trigger className={selectTriggerClass}>
                <Select.Value className="text-foreground" />
                <Select.Indicator />
              </Select.Trigger>
              <Select.Popover className="bg-background/95 dark:bg-background/35 border border-default-200/70 dark:border-white/15 backdrop-blur-xl">
                <ListBox className="bg-transparent">
                  <ListBox.Item id="prefer" textValue="Prefer">
                    Prefer
                    <ListBox.ItemIndicator />
                  </ListBox.Item>
                  <ListBox.Item id="avoid" textValue="Avoid">
                    Avoid
                    <ListBox.ItemIndicator />
                  </ListBox.Item>
                  <ListBox.Item id="never" textValue="Never">
                    Never
                    <ListBox.ItemIndicator />
                  </ListBox.Item>
                </ListBox>
              </Select.Popover>
            </Select>

            <Input
              placeholder="Context for this rule"
              value={preference.context}
              onChange={(e) =>
                updateToolPreferences(
                  skills.tool_preferences.map((item, itemIndex) =>
                    itemIndex === index ? { ...item, context: e.target.value } : item,
                  ),
                )
              }
              className={`${controlClass} md:col-span-2`}
            />
          </div>
        ))}
      </section>

      <section className="space-y-4 rounded-2xl border border-border/20 bg-card/35 backdrop-blur-md p-5">
        <div className="flex items-center justify-between">
          <h4 className={sectionTitleClass}>Behaviors</h4>
          <Button
            size="sm"
            variant="secondary"
            className={softButtonClass}
            onPress={() => updateBehaviors([...skills.behaviors, createBehavior()])}
          >
            + Add Behavior
          </Button>
        </div>

        {skills.behaviors.length === 0 && (
          <div className="p-4 rounded-xl border border-dashed border-border/30 text-sm text-muted-foreground bg-card/20">
            No behaviors yet.
          </div>
        )}

        {skills.behaviors.map((behavior) => (
          <div
            key={behavior.id}
            className="space-y-3 rounded-2xl border border-border/20 bg-card/30 backdrop-blur-md p-4"
          >
            <div className="flex items-center justify-between gap-3">
              <Input
                placeholder="Behavior name"
                value={behavior.name}
                onChange={(e) =>
                  updateBehaviors(
                    skills.behaviors.map((item) =>
                      item.id === behavior.id ? { ...item, name: e.target.value } : item,
                    ),
                  )
                }
                className={controlClass}
              />
              <Switch
                className="shrink-0"
                size="sm"
                isSelected={behavior.enabled}
                onChange={(enabled) =>
                  updateBehaviors(
                    skills.behaviors.map((item) =>
                      item.id === behavior.id ? { ...item, enabled } : item,
                    ),
                  )
                }
              >
                <Switch.Control>
                  <Switch.Thumb />
                </Switch.Control>
                Enabled
              </Switch>
            </div>

            <TextArea
              placeholder="Instruction"
              value={behavior.instruction}
              onChange={(e) =>
                updateBehaviors(
                  skills.behaviors.map((item) =>
                    item.id === behavior.id
                      ? { ...item, instruction: e.target.value }
                      : item,
                  ),
                )
              }
              className={controlClass}
              rows={3}
            />

            <Button
              size="sm"
              variant="ghost"
              className={softButtonClass}
              onPress={() =>
                updateBehaviors(skills.behaviors.filter((item) => item.id !== behavior.id))
              }
            >
              Remove behavior
            </Button>
          </div>
        ))}
      </section>
    </div>
  );
}
