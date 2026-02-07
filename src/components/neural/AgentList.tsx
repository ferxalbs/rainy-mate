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
  created_at: string;
  config?: {
    model?: string;
    temperature?: number;
    maxTokens?: number;
  };
}

interface AgentListProps {
  onCreateClick: () => void;
  refreshToken: number;
}

export function AgentList({ onCreateClick, refreshToken }: AgentListProps) {
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

  useEffect(() => {
    if (refreshToken <= 0) return;
    void fetchAgents();
  }, [refreshToken]);

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
        <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
          {agents.map((agent) => (
            <Card
              key={agent.id}
              className="p-4 hover:bg-content2 transition-colors"
            >
              <div className="flex justify-between items-start">
                <div className="flex items-center gap-3">
                  <div className="p-2 bg-primary/10 rounded-lg">
                    <Bot className="size-5 text-primary" />
                  </div>
                  <div>
                    <h4 className="font-semibold">{agent.name}</h4>
                    <span className="text-xs text-muted-foreground uppercase">
                      {agent.type}
                    </span>
                    {agent.config?.model && (
                      <div className="flex items-center gap-1 mt-1">
                        <Sparkles className="size-3 text-amber-500" />
                        <span className="text-xs text-muted-foreground">
                          {agent.config.model.replace(/-preview$/, "")}
                        </span>
                      </div>
                    )}
                  </div>
                </div>
                <Chip
                  size="sm"
                  color={agent.status === "active" ? "success" : "default"}
                  variant="soft"
                >
                  {agent.status}
                </Chip>
              </div>
            </Card>
          ))}
        </div>
      )}
    </div>
  );
}
