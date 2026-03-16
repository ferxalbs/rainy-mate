import type { AirlockConfig, AirlockRateLimits } from "../../../../types/airlock";
import { Input } from "@heroui/react";
import { inputClass, sectionTitleClass } from "./constants";

interface RateLimitsSectionProps {
  airlock: AirlockConfig;
  onRateLimitsChange: (rateLimits: AirlockRateLimits) => void;
}

interface RateLimitFieldProps {
  title: string;
  value: number;
  onChange: (value: number) => void;
}

function RateLimitField({ title, value, onChange }: RateLimitFieldProps) {
  return (
    <div className="space-y-2">
      <h4 className={sectionTitleClass}>{title}</h4>
      <Input
        type="number"
        min={1}
        value={value}
        onChange={(e) => onChange(Math.max(1, Number.parseInt(e.target.value || "1", 10)))}
        className={inputClass}
      />
    </div>
  );
}

export function RateLimitsSection({ airlock, onRateLimitsChange }: RateLimitsSectionProps) {
  return (
    <section className="grid grid-cols-1 md:grid-cols-2 gap-4">
      <RateLimitField
        title="Requests / Minute"
        value={airlock.rate_limits.max_requests_per_minute}
        onChange={(max_requests_per_minute) =>
          onRateLimitsChange({
            ...airlock.rate_limits,
            max_requests_per_minute,
          })
        }
      />
    </section>
  );
}
