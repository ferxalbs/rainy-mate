import { Moon, Sun } from "lucide-react";
import React from "react";

interface MacOSToggleProps {
  isDark: boolean;
  onToggle: (checked: boolean, event?: React.MouseEvent) => void;
}

export function MacOSToggle({ isDark, onToggle }: MacOSToggleProps) {
  return (
    <div
      className="relative flex items-center bg-foreground/[0.03] p-1 rounded-full border border-foreground/[0.05] cursor-pointer w-12 h-6.5 transition-all duration-500 hover:bg-foreground/[0.06] group"
      onClick={(e) => onToggle(!isDark, e)}
    >
      {/* Liquid Sliding Indicator */}
      <div
        className={`absolute top-0.75 bottom-0.75 w-5 rounded-full shadow-[0_2px_8px_-2px_rgba(0,0,0,0.12)] bg-background border border-foreground/[0.03] flex items-center justify-center transition-all duration-600 ease-[cubic-bezier(0.23,1,0.32,1)] ${
          isDark ? "translate-x-5.25" : "translate-x-0.75"
        } group-hover:scale-105 active:scale-95`}
      >
        <div className="relative size-3">
          <Sun 
            className={`absolute inset-0 size-3 text-amber-500/90 transition-all duration-700 ease-[cubic-bezier(0.34,1.56,0.64,1)] ${
              isDark ? "opacity-0 scale-0 rotate-180 blur-sm" : "opacity-100 scale-100 rotate-0 blur-0"
            }`} 
            strokeWidth={2.5}
          />
          <Moon 
            className={`absolute inset-0 size-3 text-blue-500/80 transition-all duration-700 ease-[cubic-bezier(0.34,1.56,0.64,1)] ${
              isDark ? "opacity-100 scale-100 rotate-0 blur-0" : "opacity-0 scale-0 -rotate-180 blur-sm"
            }`}
            strokeWidth={2.5}
          />
        </div>
      </div>
    </div>
  );
}
