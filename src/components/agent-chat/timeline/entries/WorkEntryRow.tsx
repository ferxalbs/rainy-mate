import { useState } from "react";
import { ChevronDown, ChevronRight, Zap, CheckCircle2, AlertCircle, Bot } from "lucide-react";
import type { TimelineWorkEntry } from "../MessagesTimeline.logic";
import { cn } from "../../../../lib/utils";

interface WorkEntryRowProps {
  entries: TimelineWorkEntry[];
  defaultExpanded?: boolean;
}

export function WorkEntryRow({ entries, defaultExpanded = false }: WorkEntryRowProps) {
  const [isExpanded, setIsExpanded] = useState(defaultExpanded);
  
  if (!entries || entries.length === 0) return null;

  const onlyTools = entries.every(e => e.tone === "tool");
  const groupingTitle = onlyTools ? "Tool executions" : "Work log";
  
  return (
    <div className="w-full flex justify-start mb-6">
      <div className="flex flex-col w-full max-w-[85%] rounded-2xl border border-white/10 bg-background/40 py-2.5 px-3">
        <div className="flex items-center justify-between px-1 mb-2">
           <span className="text-[10px] uppercase tracking-wider text-muted-foreground/70 font-semibold flex items-center gap-1.5">
              {groupingTitle}
              <span className="bg-white/10 text-foreground/80 px-1.5 py-0.5 rounded-full text-[9px]">
                {entries.length}
              </span>
           </span>
           <button 
             onClick={() => setIsExpanded(!isExpanded)}
             className="text-[10px] hover:text-foreground text-muted-foreground transition-colors flex items-center gap-1 uppercase tracking-wider"
           >
             {isExpanded ? "Hide" : "Show"}
             {isExpanded ? <ChevronDown className="size-3" /> : <ChevronRight className="size-3" />}
           </button>
        </div>
        
        {isExpanded && (
           <div className="flex flex-col gap-1 mt-1">
             {entries.map((entry) => (
                <div key={entry.id} className="flex items-start gap-2.5 rounded-lg border border-white/5 bg-black/20 p-2.5 hover:bg-black/30 transition-colors">
                   <div className="mt-0.5 text-muted-foreground shrink-0">
                      {entry.tone === "thinking" && <Bot className="size-4 text-emerald-400" />}
                      {entry.tone === "tool" && <Zap className="size-4 text-amber-400" />}
                      {entry.tone === "error" && <AlertCircle className="size-4 text-rose-400" />}
                      {entry.tone === "info" && <CheckCircle2 className="size-4 text-sky-400" />}
                   </div>
                   <div className="flex flex-col min-w-0 flex-1">
                      {entry.command && (
                         <div className="font-mono text-xs font-semibold text-foreground truncate mb-1 bg-white/5 py-0.5 px-1.5 rounded self-start">
                           {entry.command}
                         </div>
                      )}
                      {entry.detail && (
                         <div className={cn(
                            "text-sm whitespace-pre-wrap break-words leading-relaxed",
                            entry.tone === "thinking" ? "text-foreground/70 italic" : "text-muted-foreground"
                         )}>
                            {entry.detail}
                         </div>
                      )}
                      {entry.rawCommand && (
                         <div className="mt-1.5 text-xs font-mono text-muted-foreground/60 p-2 rounded bg-background/50 overflow-x-auto whitespace-pre border border-white/5">
                            {entry.rawCommand}
                         </div>
                      )}
                   </div>
                </div>
             ))}
           </div>
        )}
      </div>
    </div>
  );
}
