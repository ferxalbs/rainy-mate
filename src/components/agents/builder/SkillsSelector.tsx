import { Card, Separator, Switch } from "@heroui/react";
import { AgentSkills } from "../../../types/agent-spec";
import { Folder, Globe, Database, Terminal, LucideIcon } from "lucide-react";

interface SkillsSelectorProps {
  skills: AgentSkills;
  onChange: (skills: AgentSkills) => void;
}

export function SkillsSelector({ skills, onChange }: SkillsSelectorProps) {
  const toggleCapability = (name: string, enabled: boolean) => {
    let newCaps = [...skills.capabilities];
    if (enabled) {
      if (!newCaps.find((c) => c.name === name)) {
        newCaps.push({
          name,
          description: "Added by user",
          scopes: [],
          permissions: ["Read", "Write"],
        } as any);
      }
    } else {
      newCaps = newCaps.filter((c) => c.name !== name);
    }
    onChange({ ...skills, capabilities: newCaps });
  };

  const hasCapability = (name: string) =>
    !!skills.capabilities.find((c) => c.name === name);

  const renderCapToggle = (
    name: string,
    Icon: LucideIcon,
    label: string,
    desc: string,
  ) => {
    const isEnabled = hasCapability(name);
    return (
      <div className="flex items-center justify-between p-3 border border-default-200 rounded-lg hover:bg-content2 transition-colors">
        <div className="flex items-center gap-3">
          <div
            className={`p-2 rounded-lg ${isEnabled ? "bg-primary/10 text-primary" : "bg-default-100 text-default-500"}`}
          >
            {Icon && <Icon className="size-5" />}
          </div>
          <div>
            <h5 className="font-medium">{label}</h5>
            <p className="text-xs text-default-500">{desc}</p>
          </div>
        </div>
        <Switch
          size="sm"
          isSelected={isEnabled}
          onChange={(e) => {
            // HeroUI Switch onChange might return event OR boolean depending on version quirk.
            // If previous error said 'target' does not exist on 'boolean', then 'e' IS boolean.
            // We cast to any to be safe or try to infer.
            // But wait, standard React onChange is Event.
            // If Heroui overrides it to return boolean, types should match.
            // Let's try explicit boolean argument based on error.
            toggleCapability(name, e as any as boolean);
          }}
          // Actually, let's try onValueChange again? No, it didn't exist.
          // Let's rely on the error message: "Property 'target' does not exist on type 'boolean'".
          // This implies the argument IS boolean.
        />
      </div>
    );
  };

  return (
    <Card className="w-full">
      <Card.Header className="flex gap-3">
        <div className="flex flex-col">
          <p className="text-md font-bold">Skills & Permissions</p>
          <p className="text-small text-default-500">
            Grant this agent access to system resources.
          </p>
        </div>
      </Card.Header>
      <Separator />
      <Card.Content className="p-4 flex flex-col gap-4">
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {renderCapToggle(
            "filesystem",
            Folder,
            "Filesystem",
            "Read/Write access to workspace files",
          )}
          {renderCapToggle(
            "browser",
            Globe,
            "Web Browser",
            "Access to external websites",
          )}
          {renderCapToggle(
            "database",
            Database,
            "Knowledge Base",
            "Access to vector stores and memory",
          )}
          {renderCapToggle(
            "terminal",
            Terminal,
            "Shell Execution",
            "Execute sandboxed commands",
          )}
        </div>

        <div className="mt-4">
          <h4 className="text-sm font-medium mb-2">Scope Configuration</h4>
          <p className="text-xs text-default-400 mb-4">
            Refine access to specific paths or domains.
          </p>

          <div className="p-4 bg-content2 rounded-lg text-center text-sm text-default-500 border border-dashed border-default-300">
            Detailed scope configuration coming safely in v2.1
          </div>
        </div>
      </Card.Content>
    </Card>
  );
}
