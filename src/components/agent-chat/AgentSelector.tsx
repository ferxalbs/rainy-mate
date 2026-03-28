import { useState, useMemo } from "react";
import { cn } from "../../lib/utils";
import { Popover, PopoverContent, PopoverTrigger } from "../ui/popover";
import { Input } from "../ui/input";
import { Check, ChevronDown, Search, Bot, User } from "lucide-react";
import type { AgentSpec } from "../../types/agent-spec";

interface AgentSelectorProps {
  selectedAgentId: string;
  onSelect: (agentId: string) => void;
  agentSpecs: AgentSpec[];
  className?: string;
}

export function AgentSelector({
  selectedAgentId,
  onSelect,
  agentSpecs,
  className,
}: AgentSelectorProps) {
  const [searchQuery, setSearchQuery] = useState("");
  const [isPopoverOpen, setIsPopoverOpen] = useState(false);

  const selectedAgent = useMemo(
    () => agentSpecs.find((a) => a.id === selectedAgentId),
    [agentSpecs, selectedAgentId],
  );

  const filteredAgents = useMemo(() => {
    if (!searchQuery) return agentSpecs;
    return agentSpecs.filter((agent) =>
      (agent.soul.name || "Untitled Agent")
        .toLowerCase()
        .includes(searchQuery.toLowerCase()),
    );
  }, [agentSpecs, searchQuery]);

  return (
    <Popover open={isPopoverOpen} onOpenChange={setIsPopoverOpen}>
      <PopoverTrigger
        render={
          <button
            type="button"
            className={cn(
              "group flex items-center gap-1.5 rounded-md px-1.5 py-1 text-xs font-medium text-muted-foreground transition-colors hover:text-foreground",
              className
            )}
          />
        }
      >
        {selectedAgent ? (
          <span className="max-w-[150px] truncate">
            {selectedAgent.soul.name || "Untitled Agent"}
          </span>
        ) : (
          <span>Default Agent</span>
        )}
        <ChevronDown className="size-3 opacity-50 transition-transform group-data-[state=open]:rotate-180" />
      </PopoverTrigger>

      <PopoverContent align="start" sideOffset={12} className="w-[240px] overflow-hidden rounded-xl border border-white/10 bg-background/20 p-1 shadow-2xl backdrop-blur-md">
        <div className="flex flex-col">
          {/* Search */}
          <div className="border-b border-white/5 p-1.5">
            <div className="relative">
              <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 size-3 text-muted-foreground" />
              <Input
                placeholder="Search agents..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="h-7 w-full rounded-lg border-none bg-white/5 pl-8 text-[11px] focus-visible:ring-1 focus-visible:ring-white/10"
              />
            </div>
          </div>

          {/* Agent List */}
          <div className="max-h-[300px] overflow-y-auto p-1 custom-scrollbar">
            <button
              onClick={() => {
                onSelect("");
                setIsPopoverOpen(false);
              }}
              className={cn(
                "flex w-full items-center justify-between gap-3 rounded-lg px-2.5 py-2 text-left text-xs transition-colors",
                !selectedAgentId ? "bg-white/10 text-foreground" : "text-muted-foreground hover:bg-white/5 hover:text-foreground"
              )}
            >
              <div className="flex items-center gap-2">
                <User className="size-3.5" />
                <span>Default Agent</span>
              </div>
              {!selectedAgentId && <Check className="size-3.5" />}
            </button>

            {filteredAgents.length > 0 && (
              <div className="mt-1 space-y-0.5">
                {filteredAgents.map((agent) => (
                  <button
                    key={agent.id}
                    onClick={() => {
                      onSelect(agent.id);
                      setIsPopoverOpen(false);
                    }}
                    className={cn(
                      "flex w-full items-center justify-between gap-3 rounded-lg px-2.5 py-2 text-left text-xs transition-colors",
                      selectedAgentId === agent.id ? "bg-white/10 text-foreground" : "text-muted-foreground hover:bg-white/5 hover:text-foreground"
                    )}
                  >
                    <div className="flex items-center gap-2 min-w-0">
                      <Bot className="size-3.5 shrink-0" />
                      <span className="truncate">
                        {agent.soul.name || "Untitled Agent"}
                      </span>
                    </div>
                    {selectedAgentId === agent.id && <Check className="size-3.5 shrink-0" />}
                  </button>
                ))}
              </div>
            )}
          </div>
        </div>
      </PopoverContent>
    </Popover>
  );
}
