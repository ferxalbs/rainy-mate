import { ReactNode } from "react";

// Custom Chip to avoid HeroUI type issues and ensure consistent design
export function NeuralChip({
  children,
  variant = "flat",
  color = "default",
  className = "",
}: {
  children: ReactNode;
  variant?: "flat" | "solid" | "outline";
  color?: "default" | "primary" | "success" | "warning" | "danger" | "info";
  className?: string;
}) {
  const baseClass =
    "inline-flex items-center px-2 py-0.5 rounded text-xs font-medium transition-colors";

  const colorStyles = {
    default: {
      flat: "bg-foreground/10 text-foreground",
      solid: "bg-foreground text-background",
      outline: "border border-foreground/20 text-foreground",
    },
    primary: {
      flat: "bg-primary/10 text-primary",
      solid: "bg-primary text-primary-foreground",
      outline: "border border-primary/20 text-primary",
    },
    success: {
      flat: "bg-emerald-500/10 text-emerald-500",
      solid: "bg-emerald-500 text-white",
      outline: "border border-emerald-500/20 text-emerald-500",
    },
    warning: {
      flat: "bg-amber-500/10 text-amber-500",
      solid: "bg-amber-500 text-white",
      outline: "border border-amber-500/20 text-amber-500",
    },
    danger: {
      flat: "bg-red-500/10 text-red-500",
      solid: "bg-red-500 text-white",
      outline: "border border-red-500/20 text-red-500",
    },
    info: {
      flat: "bg-blue-500/10 text-blue-400",
      solid: "bg-blue-500 text-white",
      outline: "border border-blue-500/20 text-blue-400",
    },
  };

  const style = colorStyles[color][variant];

  return (
    <span className={`${baseClass} ${style} ${className}`}>{children}</span>
  );
}

// Custom Switch/Toggle
export function NeuralSwitch({
  checked,
  onChange,
  className = "",
}: {
  checked: boolean;
  onChange: (checked: boolean) => void;
  className?: string;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      onClick={() => onChange(!checked)}
      className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-primary/50 focus:ring-offset-2 focus:ring-offset-background ${
        checked ? "bg-primary" : "bg-muted"
      } ${className}`}
    >
      <span
        className={`${
          checked ? "translate-x-6" : "translate-x-1"
        } inline-block h-4 w-4 transform rounded-full bg-white transition-transform`}
      />
    </button>
  );
}

// Re-export Button with correct props or wrapper if needed?
// HeroUI Button is fine if we use className for styling instead of 'color' props which seem to bug out.
