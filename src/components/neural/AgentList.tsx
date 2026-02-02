import { Card, Chip, Button, Spinner } from "@heroui/react";
import { Bot, RefreshCw, Plus } from "lucide-react";
import { useEffect, useState } from "react";
import { listAtmAgents } from "../../services/tauri";
import { toast } from "@heroui/react";

interface Agent {
  id: string;
  name: string;
  type: string;
  status: string;
  created_at: string;
}

export function AgentList({ onCreateClick }: { onCreateClick: () => void }) {
  const [agents, setAgents] = useState<Agent[]>([]);
  const [isLoading, setIsLoading] = useState(false);

  const fetchAgents = async () => {
    setIsLoading(true);
    try {
      const result = await listAtmAgents();
      // Adjust based on actual API response structure (checking if it matches array directly or nested)
      const agentList = Array.isArray(result) ? result : result.agents || [];
      setAgents(agentList);
    } catch (error) {
      console.error("Failed to fetch agents:", error);
      toast.danger("Failed to load agents");
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    fetchAgents();
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

      {isLoading && agents.length === 0 ? (
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
