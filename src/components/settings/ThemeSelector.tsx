import { Sun, Moon, Sparkles, Check, Monitor } from "lucide-react";
import { useTheme } from "../../hooks/useTheme";
import type { ThemeName } from "../../types/theme";
import { cn } from "@/lib/utils";

/**
 * Theme Selector Component
 * Beautiful UI for switching between themes and modes
 */
export function ThemeSelector() {
  const {
    theme: currentTheme,
    mode,
    setTheme,
    toggleMode,
    themes,
  } = useTheme();

  return (
    <div className="space-y-10 animate-in fade-in slide-in-from-bottom-2 duration-700">
      {/* Mode Toggle */}
      <section className="space-y-4">
        <div className="flex items-center justify-between">
          <div className="space-y-1">
            <h3 className="text-sm font-bold tracking-tight text-foreground uppercase opacity-70">
              Appearance Mode
            </h3>
            <p className="text-xs text-muted-foreground">
              Select your preferred workspace lighting.
            </p>
          </div>
          
          {/* Mode Toggle - iOS Segmented Control Style */}
          <div className="flex bg-muted/30 p-1 rounded-2xl border border-border/10 backdrop-blur-xl shadow-inner group/toggle overflow-hidden">
            <button
              onClick={() => mode !== "light" && toggleMode()}
              className={cn(
                "relative flex items-center justify-center gap-2 h-9 px-5 text-xs font-semibold rounded-xl transition-all duration-300",
                mode === "light" 
                  ? "bg-background shadow-lg text-foreground scale-100" 
                  : "text-muted-foreground/60 hover:text-foreground/80 hover:bg-background/40 active:scale-95"
              )}
            >
              <Sun className={cn("size-3.5 transition-colors", mode === "light" ? "text-amber-500 fill-amber-500" : "")} />
              <span>Light</span>
              {mode === "light" && (
                <div className="absolute inset-0 bg-primary/5 rounded-xl animate-in fade-in" />
              )}
            </button>
            <button
              onClick={() => mode !== "dark" && toggleMode()}
              className={cn(
                "relative flex items-center justify-center gap-2 h-9 px-5 text-xs font-semibold rounded-xl transition-all duration-300",
                mode === "dark" 
                  ? "bg-background shadow-lg text-foreground scale-100" 
                  : "text-muted-foreground/60 hover:text-foreground/80 hover:bg-background/40 active:scale-95"
              )}
            >
              <Moon className={cn("size-3.5 transition-colors", mode === "dark" ? "text-indigo-400 fill-indigo-400" : "")} />
              <span>Dark</span>
              {mode === "dark" && (
                <div className="absolute inset-0 bg-primary/5 rounded-xl animate-in fade-in" />
              )}
            </button>
          </div>
        </div>
      </section>

      {/* Theme Grid */}
      <section className="space-y-6">
        <div className="flex flex-col gap-1">
          <div className="flex items-center gap-2">
            <Sparkles className="size-4 text-primary animate-pulse" />
            <h3 className="text-sm font-bold tracking-tight text-foreground uppercase opacity-70">Theme Palette</h3>
          </div>
          <p className="text-xs text-muted-foreground">
            Switch between curated design languages.
          </p>
        </div>

        <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
          {themes.map((themeOption) => {
            const isActive = currentTheme === themeOption.name;
            const colors = themeOption.colors[mode];

            return (
              <button
                key={themeOption.name}
                onClick={() => setTheme(themeOption.name as ThemeName)}
                className={cn(
                  "relative group flex flex-col items-start gap-4 h-full w-full",
                  "rounded-2xl p-5 text-left border border-border/10",
                  "transition-all duration-300 backdrop-blur-lg",
                  isActive 
                    ? "ring-2 ring-primary ring-offset-2 ring-offset-background bg-background/40 shadow-xl"
                    : "bg-muted/10 hover:bg-muted/20 hover:border-border/20 active:scale-[0.98]"
                )}
                style={{
                  background: isActive ? colors.card : undefined,
                }}
              >
                <div className="flex items-center gap-3 w-full">
                  <span className="text-3xl filter drop-shadow-md">{themeOption.icon}</span>
                  <div className="flex-1 min-w-0">
                    <h4 className="text-sm font-bold truncate tracking-tight">{themeOption.displayName}</h4>
                    <p className="text-[10px] text-muted-foreground truncate opacity-70">{themeOption.description}</p>
                  </div>
                  {isActive && (
                    <div className="size-5 rounded-full bg-primary flex items-center justify-center shadow-lg animate-in zoom-in-95">
                      <Check className="size-3 text-primary-foreground stroke-[3]" />
                    </div>
                  )}
                </div>

                <div className="flex items-center gap-1.5 w-full mt-auto">
                  {[colors.primary, colors.accent, colors.secondary, colors.muted].map((color, idx) => (
                    <div 
                      key={idx}
                      className="h-6 flex-1 rounded-md border border-white/5 shadow-inner"
                      style={{ background: color }}
                    />
                  ))}
                </div>
              </button>
            );
          })}
        </div>
      </section>

      {/* Experimental Preview Card */}
      <div className="relative overflow-hidden p-6 rounded-3xl bg-muted/20 border border-border/10 backdrop-blur-2xl group/preview">
        <div className="absolute -top-12 -right-12 size-40 bg-primary/10 blur-3xl rounded-full transition-all group-hover/preview:scale-150 duration-700" />
        
        <div className="relative space-y-4">
          <div className="flex items-center justify-between">
            <h4 className="text-xs font-bold uppercase tracking-widest text-muted-foreground/60 flex items-center gap-2">
              <Monitor className="size-3" />
              Real-time Preview
            </h4>
          </div>
          <div className="space-y-4">
            <div className="h-2 w-2/3 bg-foreground/10 rounded-full" />
            <div className="space-y-2">
              <div className="h-2 w-full bg-foreground/5 rounded-full" />
              <div className="h-2 w-5/6 bg-foreground/5 rounded-full" />
            </div>
            <div className="flex gap-2 pt-2">
              <div className="h-8 w-20 rounded-xl bg-primary shadow-lg shadow-primary/20" />
              <div className="h-8 w-20 rounded-xl bg-muted/40" />
              <div className="h-8 w-8 rounded-xl bg-accent shadow-lg shadow-accent/20" />
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
