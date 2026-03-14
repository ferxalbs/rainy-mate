import { Moon, Sun } from "lucide-react";
import React from "react";

interface MacOSToggleProps {
  isDark: boolean;
  onToggle: (checked: boolean, event?: React.MouseEvent) => void;
}

export function MacOSToggle({ isDark, onToggle }: MacOSToggleProps) {
  return (
    <div
      className="relative flex items-center bg-muted/40 p-1 rounded-full border border-border/50 cursor-pointer w-14 h-7 transition-all duration-300 hover:bg-muted/60 group"
      onClick={(e) => onToggle(!isDark, e)}
    >
      {/* Sliding Tab */}
      <div
        className={`absolute top-1 bottom-1 w-6 rounded-full shadow-md bg-background border border-border/10 flex items-center justify-center transition-all duration-500 ease-[cubic-bezier(0.34,1.56,0.64,1)] ${
          isDark ? "translate-x-7" : "translate-x-0.5"
        } group-hover:scale-105 active:scale-95`}
      >
        <div className="relative size-3.5">
          <Sun className={`absolute inset-0 size-3.5 text-amber-500 transition-all duration-500 ${isDark ? "opacity-0 scale-50 rotate-90" : "opacity-100 scale-100 rotate-0"}`} />
          <Moon className={`absolute inset-0 size-3.5 text-blue-400 transition-all duration-500 ${isDark ? "opacity-100 scale-100 rotate-0" : "opacity-0 scale-50 -rotate-90"}`} />
        </div>
      </div>

      {/* Background Icons (inactive state) */}
      <div
        className="flex w-full justify-between items-center px-2 pointer-events-none"
      >
        <Sun
          className={`size-3 transition-all duration-500 ${!isDark ? "opacity-0 scale-0" : "opacity-40 scale-100"}`}
        />
        <Moon
          className={`size-3 transition-all duration-500 ${isDark ? "opacity-0 scale-0" : "opacity-40 scale-100"}`}
        />
      </div>
    </div>
  );
}
