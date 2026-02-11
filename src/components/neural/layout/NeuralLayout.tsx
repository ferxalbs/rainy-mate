import { ReactNode } from "react";
import { useTheme } from "../../../hooks/useTheme";

interface NeuralLayoutProps {
  sidebar: ReactNode;
  children: ReactNode;
  headerContent?: ReactNode;
}

export function NeuralLayout({
  sidebar,
  children,
  headerContent,
}: NeuralLayoutProps) {
  const { mode } = useTheme();
  const isDark = mode === "dark";

  return (
    <div className="h-full w-full bg-background p-3 flex gap-3 overflow-hidden font-sans selection:bg-primary selection:text-primary-foreground relative">
      <div
        className="absolute inset-0 w-full h-full z-0 pointer-events-none"
        data-tauri-drag-region
      />

      {sidebar}

      <main
        className={`flex-1 rounded-[1.5rem] border border-border/40 shadow-xl flex flex-col overflow-hidden relative z-10 ${
          isDark ? "bg-card/20" : "bg-card/60"
        } backdrop-blur-2xl`}
      >
        <div className="absolute top-0 right-0 w-[400px] h-[400px] bg-primary/[0.03] blur-[100px] rounded-full pointer-events-none z-0" />

        {headerContent && (
          <header
            className="h-16 shrink-0 flex items-center justify-between px-8 border-b border-border/10 bg-background/20 backdrop-blur-xl z-20 relative"
            data-tauri-drag-region
          >
            {headerContent}
          </header>
        )}

        <div className="flex-1 overflow-y-auto p-8 z-10 scrollbar-hide">
          <div className="max-w-5xl mx-auto pb-16">{children}</div>
        </div>
      </main>
    </div>
  );
}
