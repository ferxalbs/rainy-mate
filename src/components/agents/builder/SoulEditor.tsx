import { Input, TextArea, Card, Separator } from "@heroui/react";
import { AgentSoul } from "../../../types/agent-spec";

interface SoulEditorProps {
  soul: AgentSoul;
  onChange: (soul: AgentSoul) => void;
}

export function SoulEditor({ soul, onChange }: SoulEditorProps) {
  const handleChange = (field: keyof AgentSoul, value: string) => {
    onChange({
      ...soul,
      [field]: value,
    });
  };

  return (
    <Card className="w-full">
      <Card.Header className="flex gap-3">
        <div className="flex flex-col">
          <p className="text-md font-bold">Identity & Soul</p>
          <p className="text-small text-default-500">
            Define who this agent is.
          </p>
        </div>
      </Card.Header>
      <Separator />
      <Card.Content className="gap-4 p-4 flex flex-col">
        <div className="flex gap-4">
          <div className="flex-1 flex flex-col gap-1">
            <span className="text-small font-medium">Name</span>
            <Input
              placeholder="Agent Name"
              value={soul.name}
              onChange={(e) => handleChange("name", e.target.value)}
            />
          </div>
          <div className="w-32 flex flex-col gap-1">
            <span className="text-small font-medium">Version</span>
            <Input
              placeholder="1.0.0"
              value={soul.version}
              onChange={(e) => handleChange("version", e.target.value)}
            />
          </div>
        </div>

        <div className="flex flex-col gap-1">
          <span className="text-small font-medium">Description</span>
          <TextArea
            placeholder="Brief description of what this agent does..."
            value={soul.description}
            onChange={(e) => handleChange("description", e.target.value)}
          />
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div className="flex flex-col gap-1">
            <span className="text-small font-medium">Personality</span>
            <TextArea
              placeholder="e.g. Helpful, concise, technical..."
              value={soul.personality}
              onChange={(e) => handleChange("personality", e.target.value)}
              className="min-h-[100px]"
            />
          </div>
          <div className="flex flex-col gap-1">
            <span className="text-small font-medium">Tone</span>
            <Input
              placeholder="e.g. Professional, Casual, Pirate"
              value={soul.tone}
              onChange={(e) => handleChange("tone", e.target.value)}
            />
          </div>
        </div>

        <Separator className="my-2" />

        <div className="space-y-2">
          <h3 className="text-sm font-medium">Soul Content (Markdown)</h3>
          <p className="text-xs text-default-400">
            Detailed instructions, behavioral guidelines, and core beliefs.
          </p>
          <TextArea
            aria-label="Soul Content"
            placeholder="# My Core Directives..."
            value={soul.soul_content}
            onChange={(e) => handleChange("soul_content", e.target.value)}
            className="min-h-[300px] font-mono text-sm"
          />
        </div>
      </Card.Content>
    </Card>
  );
}
