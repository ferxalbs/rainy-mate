import { Moon, Sun } from "lucide-react";

interface MacOSToggleProps {
  isDark: boolean;
  onToggle: (checked: boolean) => void;
}

export function MacOSToggle({ isDark, onToggle }: MacOSToggleProps) {
  return (
    <div
      className="relative flex items-center bg-muted/40 p-1 rounded-full border border-border/50 cursor-pointer w-14 h-7 transition-all duration-300 hover:bg-muted/60"
      onClick={() => onToggle(!isDark)}
    >
      {/* Sliding Tab */}
      <div
        className={`absolute top-1 bottom-1 w-6 rounded-full shadow-sm bg-background border border-border flex items-center justify-center transition-all duration-300 ease-[cubic-bezier(0.25,1,0.5,1)] ${
          isDark ? "translate-x-7" : "translate-x-0.5"
        }`}
      >
        {isDark ? (
          <Moon className="size-3.5 text-foreground animate-in fade-in zoom-in duration-300" />
        ) : (
          <Sun className="size-3.5 text-foreground animate-in fade-in zoom-in duration-300" />
        )}
      </div>

      {/* Background Icons (inactive state) */}
      <div
        className={`flex w-full justify-between items-center px-1.5 pointer-events-none`}
      >
        <Sun
          className={`size-3.5 transition-opacity duration-300 ${!isDark ? "opacity-0" : "opacity-40"}`}
        />
        <Moon
          className={`size-3.5 transition-opacity duration-300 ${isDark ? "opacity-0" : "opacity-40"}`}
        />
      </div>
    </div>
  );
}
