import React, { useEffect, useRef } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  Zap,
  Brain,
  Terminal,
  Activity,
  Cpu,
  CheckCircle2,
} from "lucide-react";
import { ScrollShadow } from "@heroui/react";

interface NeuralStreamProps {
  isVisible: boolean;
  title?: string;
  isThinking: boolean;
  currentThought?: string;
  logs: string[];
  status: "idle" | "thinking" | "executing" | "complete" | "error";
}

export const NeuralStream: React.FC<NeuralStreamProps> = ({
  isVisible,
  isThinking,
  currentThought,
  logs,
  status,
}) => {
  const scrollRef = useRef<HTMLDivElement>(null);

  // Auto-scroll logic
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [logs, currentThought]);

  if (!isVisible) return null;

  return (
    <AnimatePresence>
      <motion.div
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        exit={{ opacity: 0, y: 20 }}
        className="fixed bottom-24 right-6 w-96 max-h-[400px] z-50 pointer-events-none"
      >
        {/* Holographic Container */}
        <div className="relative overflow-hidden rounded-xl border border-primary/20 bg-background/80 backdrop-blur-xl shadow-2xl pointer-events-auto">
          {/* Animated Glow Effect */}
          <div className="absolute inset-0 bg-gradient-to-tr from-primary/5 via-transparent to-secondary/5 pointer-events-none" />

          {/* Header Status Bar */}
          <div className="flex items-center justify-between px-4 py-2 border-b border-primary/10 bg-primary/5">
            <div className="flex items-center gap-2">
              <StatusIcon status={status} />
              <span className="text-xs font-mono font-medium uppercase tracking-wider text-primary">
                {status === "thinking"
                  ? "NEURAL ENGINE ACTIVE"
                  : status === "executing"
                    ? "EXECUTING PROTOCOLS"
                    : "SYSTEM READY"}
              </span>
            </div>
            {isThinking && (
              <div className="flex gap-1">
                {[1, 2, 3].map((i) => (
                  <motion.div
                    key={i}
                    animate={{ opacity: [0.3, 1, 0.3] }}
                    transition={{
                      duration: 1,
                      repeat: Infinity,
                      delay: i * 0.2,
                    }}
                    className="w-1.5 h-1.5 rounded-full bg-primary"
                  />
                ))}
              </div>
            )}
          </div>

          {/* Terminal/Stream Content */}
          <div className="h-64 font-mono text-xs relative">
            <ScrollShadow className="h-full w-full">
              <div ref={scrollRef} className="p-4 space-y-2">
                {/* Historical Logs */}
                {logs.map((log, i) => (
                  <div key={i} className="text-muted-foreground/80 break-words">
                    <span className="text-primary/50 mr-2">{">"}</span>
                    {log}
                  </div>
                ))}

                {/* Active Thought Stream */}
                {currentThought && (
                  <div className="text-primary animate-pulse break-words">
                    <span className="mr-2 text-secondary">⚡</span>
                    {currentThought}
                    <span className="inline-block w-2 h-4 ml-1 bg-primary align-middle animate-blink" />
                  </div>
                )}
              </div>
            </ScrollShadow>
          </div>

          {/* Footer Metrics */}
          <div className="flex items-center justify-between px-4 py-1.5 border-t border-primary/10 bg-background/50 text-[10px] text-muted-foreground font-mono">
            <div className="flex gap-4">
              <span className="flex items-center gap-1">
                <Cpu className="size-3" /> THREADS: 8
              </span>
              <span className="flex items-center gap-1">
                <Activity className="size-3" /> MEMORY: OPTIMAL
              </span>
            </div>
            <div className="text-primary/70">V3.0.0-NEURAL</div>
          </div>
        </div>
      </motion.div>
    </AnimatePresence>
  );
};

// Helper for status icon
const StatusIcon = ({ status }: { status: string }) => {
  switch (status) {
    case "thinking":
      return <Brain className="size-4 text-purple-400 animate-pulse" />;
    case "executing":
      return <Terminal className="size-4 text-cyan-400" />;
    case "complete":
      return <CheckCircle2 className="size-4 text-green-400" />;
    case "error":
      return <Zap className="size-4 text-red-400" />;
    default:
      return <Activity className="size-4 text-muted-foreground" />;
  }
};
