import {
  Input,
  Textarea,
  Card,
  CardBody,
  CardHeader,
  Divider,
} from "@heroui/react";
import { AgentSoul } from "../../../types/agent-spec";
import { useMemo } from "react";

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
      <CardHeader className="flex gap-3">
        <div className="flex flex-col">
          <p className="text-md font-bold">Identity & Soul</p>
          <p className="text-small text-default-500">
            Define who this agent is.
          </p>
        </div>
      </CardHeader>
      <Divider />
      <CardBody className="gap-4">
        <div className="flex gap-4">
          <Input
            label="Name"
            placeholder="Agent Name"
            value={soul.name}
            onValueChange={(v) => handleChange("name", v)}
            className="flex-1"
          />
          <Input
            label="Version"
            placeholder="1.0.0"
            value={soul.version}
            onValueChange={(v) => handleChange("version", v)}
            className="w-32"
          />
        </div>

        <Textarea
          label="Description"
          placeholder="Brief description of what this agent does..."
          value={soul.description}
          onValueChange={(v) => handleChange("description", v)}
        />

        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <Textarea
            label="Personality"
            placeholder="e.g. Helpful, concise, technical..."
            value={soul.personality}
            onValueChange={(v) => handleChange("personality", v)}
            minRows={3}
          />
          <Input
            label="Tone"
            placeholder="e.g. Professional, Casual, Pirate"
            value={soul.tone}
            onValueChange={(v) => handleChange("tone", v)}
          />
        </div>

        <Divider className="my-2" />

        <div className="space-y-2">
          <h3 className="text-sm font-medium">Soul Content (Markdown)</h3>
          <p className="text-xs text-default-400">
            Detailed instructions, behavioral guidelines, and core beliefs.
          </p>
          <Textarea
            aria-label="Soul Content"
            placeholder="# My Core Directives..."
            value={soul.soul_content}
            onValueChange={(v) => handleChange("soul_content", v)}
            minRows={10}
            classNames={{
              input: "font-mono text-sm",
            }}
          />
        </div>
      </CardBody>
    </Card>
  );
}
