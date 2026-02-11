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
const inputClass =
  "w-full bg-card/40 hover:bg-card/60 backdrop-blur-md rounded-xl px-4 py-3 text-sm text-foreground placeholder:text-muted-foreground/50 leading-relaxed border border-border/20 focus:outline-none focus:border-primary/50 focus:ring-1 focus:ring-primary/20 transition-all shadow-sm";

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
      <div className="flex flex-col gap-1 border-b border-border/10 pb-6">
        <h3 className="text-2xl font-bold text-foreground tracking-tight">Skills</h3>
        <p className="text-muted-foreground text-sm">
          Define workflows, tool preferences, and behavior rules.
        </p>
      </div>

      <section className="space-y-4">
        <div className="flex items-center justify-between">
          <h4 className={sectionTitleClass}>Workflows</h4>
          <button
            type="button"
            onClick={() => updateWorkflows([...skills.workflows, createWorkflow()])}
            className="px-3 py-1.5 text-xs rounded-lg border border-border/30 text-foreground hover:border-primary/40 hover:text-primary transition-colors"
          >
            + Add Workflow
          </button>
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
                <label className="text-xs text-muted-foreground flex items-center gap-2">
                  <input
                    type="checkbox"
                    checked={workflow.enabled}
                    onChange={(e) =>
                      updateWorkflows(
                        skills.workflows.map((item) =>
                          item.id === workflow.id
                            ? { ...item, enabled: e.target.checked }
                            : item,
                        ),
                      )
                    }
                    className="accent-primary"
                  />
                  Enabled
                </label>
                <button
                  type="button"
                  onClick={() =>
                    updateWorkflows(
                      skills.workflows.filter((item) => item.id !== workflow.id),
                    )
                  }
                  className="px-2 py-1 text-xs rounded-md border border-border/30 text-muted-foreground hover:text-destructive hover:border-destructive/40 transition-colors"
                >
                  Remove
                </button>
              </div>
            </div>

            <input
              type="text"
              placeholder="Name"
              value={workflow.name}
              onChange={(e) =>
                updateWorkflows(
                  skills.workflows.map((item) =>
                    item.id === workflow.id
                      ? { ...item, name: e.target.value }
                      : item,
                  ),
                )
              }
              className={inputClass}
            />

            <input
              type="text"
              placeholder="Trigger (e.g. user asks for code review)"
              value={workflow.trigger}
              onChange={(e) =>
                updateWorkflows(
                  skills.workflows.map((item) =>
                    item.id === workflow.id
                      ? { ...item, trigger: e.target.value }
                      : item,
                  ),
                )
              }
              className={inputClass}
            />

            <textarea
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
              className={`${inputClass} resize-none`}
              rows={2}
            />

            <textarea
              placeholder="Workflow steps (markdown)"
              value={workflow.steps}
              onChange={(e) =>
                updateWorkflows(
                  skills.workflows.map((item) =>
                    item.id === workflow.id
                      ? { ...item, steps: e.target.value }
                      : item,
                  ),
                )
              }
              className={`${inputClass} font-mono text-xs resize-none`}
              rows={5}
            />
          </div>
        ))}
      </section>

      <section className="space-y-4">
        <div className="flex items-center justify-between">
          <h4 className={sectionTitleClass}>Tool Preferences</h4>
          <button
            type="button"
            onClick={() =>
              updateToolPreferences([...skills.tool_preferences, createToolPreference()])
            }
            className="px-3 py-1.5 text-xs rounded-lg border border-border/30 text-foreground hover:border-primary/40 hover:text-primary transition-colors"
          >
            + Add Tool Rule
          </button>
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
            <input
              type="text"
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
              className={`${inputClass} md:col-span-1`}
            />

            <select
              value={preference.priority}
              onChange={(e) =>
                updateToolPreferences(
                  skills.tool_preferences.map((item, itemIndex) =>
                    itemIndex === index
                      ? {
                          ...item,
                          priority: e.target.value as
                            | "prefer"
                            | "avoid"
                            | "never",
                        }
                      : item,
                  ),
                )
              }
              className={`${inputClass} md:col-span-1`}
            >
              <option value="prefer">Prefer</option>
              <option value="avoid">Avoid</option>
              <option value="never">Never</option>
            </select>

            <input
              type="text"
              placeholder="Context for this rule"
              value={preference.context}
              onChange={(e) =>
                updateToolPreferences(
                  skills.tool_preferences.map((item, itemIndex) =>
                    itemIndex === index ? { ...item, context: e.target.value } : item,
                  ),
                )
              }
              className={`${inputClass} md:col-span-2`}
            />
          </div>
        ))}
      </section>

      <section className="space-y-4">
        <div className="flex items-center justify-between">
          <h4 className={sectionTitleClass}>Behaviors</h4>
          <button
            type="button"
            onClick={() => updateBehaviors([...skills.behaviors, createBehavior()])}
            className="px-3 py-1.5 text-xs rounded-lg border border-border/30 text-foreground hover:border-primary/40 hover:text-primary transition-colors"
          >
            + Add Behavior
          </button>
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
              <input
                type="text"
                placeholder="Behavior name"
                value={behavior.name}
                onChange={(e) =>
                  updateBehaviors(
                    skills.behaviors.map((item) =>
                      item.id === behavior.id
                        ? { ...item, name: e.target.value }
                        : item,
                    ),
                  )
                }
                className={inputClass}
              />
              <label className="text-xs text-muted-foreground flex items-center gap-2 shrink-0">
                <input
                  type="checkbox"
                  checked={behavior.enabled}
                  onChange={(e) =>
                    updateBehaviors(
                      skills.behaviors.map((item) =>
                        item.id === behavior.id
                          ? { ...item, enabled: e.target.checked }
                          : item,
                      ),
                    )
                  }
                  className="accent-primary"
                />
                Enabled
              </label>
            </div>

            <textarea
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
              className={`${inputClass} resize-none`}
              rows={3}
            />

            <button
              type="button"
              onClick={() =>
                updateBehaviors(
                  skills.behaviors.filter((item) => item.id !== behavior.id),
                )
              }
              className="px-2 py-1 text-xs rounded-md border border-border/30 text-muted-foreground hover:text-destructive hover:border-destructive/40 transition-colors"
            >
              Remove behavior
            </button>
          </div>
        ))}
      </section>
    </div>
  );
}
