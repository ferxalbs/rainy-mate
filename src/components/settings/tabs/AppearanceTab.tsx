import { useContext } from "react";
import { Sparkles, Layers, Zap } from "lucide-react";
import { ThemeSelector } from "../ThemeSelector";
import { ThemeContext } from "../../../providers/ThemeProvider";
import { Switch } from "@heroui/react";

export function AppearanceTab() {
  const themeContext = useContext(ThemeContext);

  return (
    <div className="space-y-12 animate-in fade-in duration-500">
      <ThemeSelector />

      <div className="h-px bg-success/10 w-full opacity-10" />

      <div className="space-y-8">
        <div className="flex items-center justify-between p-4 rounded-2xl bg-muted/10 border border-border/5 hover:bg-muted/20 transition-all group">
          <div className="flex flex-col gap-1">
            <span className="text-sm font-bold flex items-center gap-2 tracking-tight uppercase opacity-80">
              <Sparkles className="size-4 text-primary group-hover:animate-pulse" />
              Premium Animations
            </span>
            <span className="text-xs text-muted-foreground max-w-sm">
              Enable high-fidelity background effects and transitions.
            </span>
          </div>
          <Switch
            isSelected={themeContext?.enableAnimations}
            onChange={(checked) => themeContext?.setEnableAnimations(checked as unknown as boolean)}
          />
        </div>

        <div className="flex items-center justify-between p-4 rounded-2xl bg-muted/10 border border-border/5 hover:bg-muted/20 transition-all group">
          <div className="flex flex-col gap-1">
            <span className="text-sm font-bold flex items-center gap-2 tracking-tight uppercase opacity-80">
              <Layers className="size-4 text-primary" />
              Compact Mode
            </span>
            <span className="text-xs text-muted-foreground max-w-sm">
              Reduce interface density for smaller screens or minimalists.
            </span>
          </div>
          <Switch
            isSelected={themeContext?.enableCompactMode}
            onChange={(checked) => themeContext?.setEnableCompactMode(checked as unknown as boolean)}
          />
        </div>

        <div className="flex items-center justify-between p-4 rounded-2xl bg-muted/10 border border-border/5 hover:bg-muted/20 transition-all group opacity-50 cursor-not-allowed">
          <div className="flex flex-col gap-1">
            <span className="text-sm font-bold flex items-center gap-2 tracking-tight uppercase opacity-80">
              <Zap className="size-4 text-primary" />
              Ultra Low Power
            </span>
            <span className="text-xs text-muted-foreground max-w-sm">
              Disables all blur and transparency for maximum performance.
            </span>
          </div>
          <Switch isDisabled />
        </div>
      </div>
    </div>
  );
}
