import { Card, Chip, Button, Spinner } from "@heroui/react";
import { Bot, RefreshCw, Plus, Sparkles } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import {
  listAtmAgents,
  ensureAtmCredentialsLoaded,
} from "../../services/tauri";
import { toast } from "sonner";

interface Agent {
  id: string;
  name: string;
  type: string;
  status: string;
  createdAt?: number;
  updatedAt?: number;
  logicalSpecId?: string | null;
  isDuplicateLogicalSpec?: boolean;
  config?: {
    model?: string;
    temperature?: number;
    maxTokens?: number;
  };
}

interface AgentListProps {
  onCreateClick: () => void;
}

export function AgentList({ onCreateClick }: AgentListProps) {
  const [agents, setAgents] = useState<Agent[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [authStatus, setAuthStatus] = useState<"unknown" | "ready" | "missing">(
    "unknown",
  );
  const inFlightRef = useRef(false);

  const fetchAgents = async () => {
    if (inFlightRef.current) return;
    inFlightRef.current = true;
    setIsLoading(true);
    try {
      const hasCreds = await ensureAtmCredentialsLoaded();
      if (!hasCreds) {
        setAuthStatus("missing");
        setAgents([]);
        return;
      }

      setAuthStatus("ready");
      const result = await listAtmAgents();
      const agentList = Array.isArray(result) ? result : result.agents || [];
      setAgents(agentList);
    } catch (error) {
      console.error("Failed to fetch agents:", error);
      const message =
        (error as any)?.message || (error as any)?.toString?.() || "";
      if (
        message.includes("Not authenticated") ||
        message.includes("Unauthorized")
      ) {
        setAuthStatus("missing");
        setAgents([]);
        toast.error("ATM admin key missing. Reconnect Neural Link.");
      } else {
        toast.error("Failed to load agents");
      }
    } finally {
      setIsLoading(false);
      inFlightRef.current = false;
    }
  };

  useEffect(() => {
    void fetchAgents();
  }, []);

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center justify-between">
        <h3 className="text-lg font-semibold flex items-center gap-2">
          <Bot className="size-5 text-purple-500" />
          Active Agents
        </h3>
        <div className="flex gap-2">
          <Button
            size="sm"
            variant="ghost"
            onPress={fetchAgents}
            isDisabled={isLoading}
            isIconOnly
          >
            <RefreshCw
              className={`size-4 ${isLoading ? "animate-spin" : ""}`}
            />
          </Button>
          <Button size="sm" variant="primary" onPress={onCreateClick}>
            <Plus className="size-4 mr-1" />
            Create Agent
          </Button>
        </div>
      </div>

      {authStatus === "missing" ? (
        <Card className="p-8 text-center text-muted-foreground border-dashed">
          <Bot className="size-10 mx-auto mb-3 opacity-20" />
          <p>ATM admin key not available.</p>
          <p className="text-sm mt-1">
            Reconnect in Neural Link to restore access.
          </p>
        </Card>
      ) : isLoading && agents.length === 0 ? (
        <div className="flex justify-center p-8">
          <Spinner size="lg" />
        </div>
      ) : agents.length === 0 ? (
        <Card className="p-8 text-center text-muted-foreground border-dashed">
          <Bot className="size-10 mx-auto mb-3 opacity-20" />
          <p>No agents deployed yet.</p>
          <p className="text-sm mt-1">
            Create your first agent to start using the Cloud Cortex.
          </p>
        </Card>
      ) : (
        <div className="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-5">
          {agents.map((agent) => (
            <Card
              key={agent.id}
              className="group flex flex-col p-5 border border-border/10 bg-card/40 backdrop-blur-xl hover:bg-card/60 hover:border-primary/20 hover:shadow-lg hover:shadow-primary/5 transition-all duration-300 rounded-[1.25rem]"
            >
              <div className="flex justify-between items-start gap-4 mb-4">
                <div className="flex gap-3 min-w-0 flex-1">
                  <div className="p-3 bg-primary/10 rounded-2xl group-hover:bg-primary/20 transition-colors shrink-0 flex items-center justify-center">
                    <Bot className="size-6 text-primary" />
                  </div>
                  <div className="flex flex-col min-w-0 justify-center">
                    <h4 className="font-bold text-foreground text-base leading-tight truncate">
                      {agent.name}
                    </h4>
                    <span className="text-[11px] font-semibold tracking-wider uppercase text-primary/80 mt-1 truncate">
                      {agent.type.replace(/_/g, " ")}
                    </span>
                  </div>
                </div>
                <div className="shrink-0 pt-0.5">
                  <Chip
                    size="sm"
                    variant="soft"
                    color={agent.status === "active" ? "success" : "default"}
                    className="font-bold text-[10px] uppercase tracking-wider px-1 shadow-sm border-none"
                  >
                    {agent.status}
                  </Chip>
                </div>
              </div>

              <div className="space-y-2 mb-4 flex-1">
                <div className="flex items-center justify-between text-[11px] font-mono px-3 py-1.5 rounded-lg bg-black/5 dark:bg-black/20 border border-white/5">
                  <span className="text-muted-foreground/70 font-sans tracking-wide">ID</span>
                  <span className="text-muted-foreground select-all">{agent.id.slice(0, 12)}</span>
                </div>
                {agent.logicalSpecId && (
                  <div className="flex items-center justify-between text-[11px] font-mono px-3 py-1.5 rounded-lg bg-black/5 dark:bg-black/20 border border-white/5">
                    <span className="text-muted-foreground/70 font-sans tracking-wide">SPEC</span>
                    <span className="text-muted-foreground select-all">{agent.logicalSpecId.slice(0, 12)}</span>
                  </div>
                )}
              </div>

              {agent.config?.model && (
                <div className="mt-auto pt-4 border-t border-border/10 flex items-center justify-between">
                  <div className="flex items-center gap-1.5">
                    <Sparkles className="size-3.5 text-amber-500/80" />
                    <span className="text-xs font-medium text-muted-foreground/80 truncate">
                      {agent.config.model.replace(/-preview$/, "").replace(/^models\//, "")}
                    </span>
                  </div>
                </div>
              )}
              {agent.isDuplicateLogicalSpec ? (
                <div className="mt-3 text-[10px] font-semibold uppercase tracking-wide text-warning">
                  Duplicate logical spec detected
                </div>
              ) : null}
            </Card>
          ))}
        </div>
      )}
    </div>
  );
}
