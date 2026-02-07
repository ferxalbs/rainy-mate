import {
  Card,
  CardBody,
  CardHeader,
  Divider,
  Switch,
  Chip,
  CheckboxGroup,
  Checkbox,
} from "@heroui/react";
import { AgentSkills, Capability, Permission } from "../../../types/agent-spec";

interface SkillsSelectorProps {
  skills: AgentSkills;
  onChange: (skills: AgentSkills) => void;
}

const AVAILABLE_CAPABILITIES = [
  {
    name: "filesystem",
    description: "Access local files and directories",
    defaultScopes: ["$HOME/Documents", "$HOME/Projects"],
    permissions: [Permission.Read, Permission.Write],
  },
  {
    name: "browser",
    description: "Browse the web and capture content",
    defaultScopes: ["https://*"],
    permissions: [Permission.Read],
  },
  {
    name: "shell",
    description: "Execute system commands",
    defaultScopes: ["ls", "grep", "git"],
    permissions: [Permission.Execute],
  },
  {
    name: "telegram",
    description: "Send and receive messages on Telegram",
    defaultScopes: ["chat_id:*"],
    permissions: [Permission.Read, Permission.Write],
  },
];

export function SkillsSelector({ skills, onChange }: SkillsSelectorProps) {
  const hasCapability = (name: string) => {
    return skills.capabilities.some((c) => c.name === name);
  };

  const toggleCapability = (name: string, enabled: boolean) => {
    if (enabled) {
      // Add default capability
      const template = AVAILABLE_CAPABILITIES.find((c) => c.name === name);
      if (template) {
        onChange({
          ...skills,
          capabilities: [
            ...skills.capabilities,
            {
              name: template.name,
              description: template.description,
              scopes: template.defaultScopes,
              permissions: template.permissions,
            },
          ],
        });
      }
    } else {
      // Remove capability
      onChange({
        ...skills,
        capabilities: skills.capabilities.filter((c) => c.name !== name),
      });
    }
  };

  // Helper to update specific capability fields (scopes/permissions)
  const updateCapability = (
    name: string,
    updater: (c: Capability) => Capability,
  ) => {
    onChange({
      ...skills,
      capabilities: skills.capabilities.map((c) =>
        c.name === name ? updater(c) : c,
      ),
    });
  };

  return (
    <Card className="w-full">
      <CardHeader className="flex flex-col items-start gap-1">
        <p className="text-md font-bold">Skills & Capabilities</p>
        <p className="text-small text-default-500">
          What is this agent allowed to do?
        </p>
      </CardHeader>
      <Divider />
      <CardBody className="gap-6">
        {AVAILABLE_CAPABILITIES.map((cap) => {
          const isEnabled = hasCapability(cap.name);
          const currentConfig = skills.capabilities.find(
            (c) => c.name === cap.name,
          );

          return (
            <div
              key={cap.name}
              className="flex flex-col gap-2 p-3 rounded-lg border border-default-100 hover:border-default-300 transition-colors"
            >
              <div className="flex justify-between items-center">
                <div className="flex flex-col">
                  <span className="text-sm font-semibold uppercase tracking-wider text-primary">
                    {cap.name}
                  </span>
                  <span className="text-xs text-default-500">
                    {cap.description}
                  </span>
                </div>
                <Switch
                  isSelected={isEnabled}
                  onValueChange={(v) => toggleCapability(cap.name, v)}
                  size="sm"
                />
              </div>

              {isEnabled && currentConfig && (
                <div className="ml-4 pl-4 border-l-2 border-primary/20 space-y-3 mt-2">
                  {/* Permissions */}
                  <div className="flex gap-2 flex-wrap">
                    {Object.values(Permission).map((p) => {
                      const hasPerm = currentConfig.permissions.includes(p);
                      // Simple toggle logic for permissions
                      const togglePerm = () => {
                        const newPerms = hasPerm
                          ? currentConfig.permissions.filter((x) => x !== p)
                          : [...currentConfig.permissions, p];
                        updateCapability(cap.name, (c) => ({
                          ...c,
                          permissions: newPerms,
                        }));
                      };

                      return (
                        <Chip
                          key={p}
                          variant={hasPerm ? "solid" : "bordered"}
                          color={hasPerm ? "primary" : "default"}
                          size="sm"
                          className="cursor-pointer select-none"
                          onClick={togglePerm}
                        >
                          {p}
                        </Chip>
                      );
                    })}
                  </div>

                  {/* Scopes Display (ReadOnly for now in MVP) */}
                  <div className="flex gap-2 flex-wrap items-center">
                    <span className="text-xs text-default-400">Scopes:</span>
                    {currentConfig.scopes.map((scope, idx) => (
                      <code
                        key={idx}
                        className="text-xs bg-default-100 px-1 py-0.5 rounded"
                      >
                        {scope}
                      </code>
                    ))}
                    {/* 
                         TODO: Add scope editor 
                         For now, users accept defaults or edit JSON manually if we add a raw view.
                        */}
                  </div>
                </div>
              )}
            </div>
          );
        })}
      </CardBody>
    </Card>
  );
}
