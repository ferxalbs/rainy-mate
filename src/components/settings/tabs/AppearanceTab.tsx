import { useContext } from "react";
import { Label, Switch } from "@heroui/react";
import { Sparkles } from "lucide-react";
import { ThemeSelector } from "../ThemeSelector";
import { ThemeContext } from "../../../providers/ThemeProvider";

export function AppearanceTab() {
  const themeContext = useContext(ThemeContext);

  return (
    <div className="space-y-4">
      <ThemeSelector />

      <div className="space-y-6">
        <div className="flex items-center justify-between">
          <div className="flex flex-col gap-1">
            <span className="text-sm font-medium flex items-center gap-2">
              <Sparkles className="size-4 text-primary" />
              Premium Animations
            </span>
            <span className="text-xs text-muted-foreground">
              Enable dynamic background effects (may impact battery)
            </span>
          </div>
          <Switch
            isSelected={themeContext?.enableAnimations}
            onChange={(e) => themeContext?.setEnableAnimations(e.valueOf())}
          >
            <Switch.Control>
              <Switch.Thumb />
            </Switch.Control>
          </Switch>
        </div>

        <div className="flex items-center justify-between">
          <div>
            <Label className="font-medium">Compact Mode</Label>
            <p className="text-sm text-muted-foreground">
              Reduce spacing in UI
            </p>
          </div>
          <Switch>
            <Switch.Control>
              <Switch.Thumb />
            </Switch.Control>
          </Switch>
        </div>
      </div>
    </div>
  );
}
