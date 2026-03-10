import {
  Button,
  Input,
  ListBox,
  Select,
  Switch,
  TextArea,
} from "@heroui/react";
import { Cpu } from "lucide-react";
import type {
  AgentSkills,
  SkillBehavior,
  SkillWorkflow,
  ToolPreference,
} from "../../../types/agent-spec";

interface SkillsEditorProps {
  skills: AgentSkills;
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

export function SkillsEditor({ skills, onChange }: SkillsEditorProps) {
  const updateWorkflows = (workflows: SkillWorkflow[]) => {
    onChange({ ...skills, workflows });
  };

  const updateToolPreferences = (tool_preferences: ToolPreference[]) => {
    onChange({ ...skills, tool_preferences });
  };

  const updateBehaviors = (behaviors: SkillBehavior[]) => {
    onChange({ ...skills, behaviors });
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
            Define workflows, tool preferences, and behavior rules.
          </p>
        </div>
      </div>

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
