import type { AirlockConfig, AirlockScopes } from "../../../../types/airlock";
import { TextArea } from "@heroui/react";
import { inputClass, sectionTitleClass } from "./constants";
import { joinList, parseList } from "./utils";

interface ScopesSectionProps {
  airlock: AirlockConfig;
  onScopesChange: (scopes: AirlockScopes) => void;
}

interface ScopeFieldProps {
  title: string;
  rows: number;
  value: string[];
  onChange: (value: string[]) => void;
}

function ScopeField({ title, rows, value, onChange }: ScopeFieldProps) {
  return (
    <div className="space-y-2">
      <h4 className={sectionTitleClass}>{title}</h4>
      <TextArea
        className={`${inputClass} resize-none`}
        rows={rows}
        value={joinList(value)}
        onChange={(e) => onChange(parseList(e.target.value))}
      />
    </div>
  );
}

export function ScopesSection({ airlock, onScopesChange }: ScopesSectionProps) {
  return (
    <section className="grid grid-cols-1 md:grid-cols-2 gap-4">
      <ScopeField
        title="Allowed Paths"
        rows={4}
        value={airlock.scopes.allowed_paths}
        onChange={(allowed_paths) =>
          onScopesChange({
            ...airlock.scopes,
            allowed_paths,
          })
        }
      />
      <ScopeField
        title="Blocked Paths"
        rows={4}
        value={airlock.scopes.blocked_paths}
        onChange={(blocked_paths) =>
          onScopesChange({
            ...airlock.scopes,
            blocked_paths,
          })
        }
      />
      <ScopeField
        title="Allowed Domains"
        rows={3}
        value={airlock.scopes.allowed_domains}
        onChange={(allowed_domains) =>
          onScopesChange({
            ...airlock.scopes,
            allowed_domains,
          })
        }
      />
      <ScopeField
        title="Blocked Domains"
        rows={3}
        value={airlock.scopes.blocked_domains}
        onChange={(blocked_domains) =>
          onScopesChange({
            ...airlock.scopes,
            blocked_domains,
          })
        }
      />
    </section>
  );
}
