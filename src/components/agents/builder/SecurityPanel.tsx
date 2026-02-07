import { Card, Chip, Button, Separator } from "@heroui/react";
import { AgentSpec } from "../../../types/agent-spec";
import { Shield, Lock, AlertTriangle } from "lucide-react";

interface SecurityPanelProps {
  spec: AgentSpec;
  onUpdate: (updates: Partial<AgentSpec>) => void;
}

export function SecurityPanel({ spec, onUpdate }: SecurityPanelProps) {
  const isSigned = !!spec.signature;

  const handleSign = () => {
    // Logic to sign would go here
    console.log("Signing agent...");
  };

  return (
    <Card className="w-full">
      <Card.Header className="flex gap-3">
        <div className="flex flex-col">
          <p className="text-md font-bold">Security & Trust</p>
          <p className="text-small text-default-500">
            Sign this agent to verify its capabilities and origin.
          </p>
        </div>
      </Card.Header>
      <Separator />

      <Card.Content className="p-4 flex flex-col gap-4">
        {isSigned ? (
          <div className="bg-success-50 dark:bg-success-900/10 border border-success-200 dark:border-success-800 rounded-lg p-4 flex items-center gap-4">
            <div className="p-2 bg-success-100 dark:bg-success-900/30 rounded-full">
              <Shield className="size-6 text-success-600 dark:text-success-400" />
            </div>
            <div>
              <h4 className="font-semibold text-success-800 dark:text-success-200">
                Verifiably Signed
              </h4>
              <p className="text-sm text-success-600 dark:text-success-400">
                This agent is cryptographically signed and its capabilities are
                locked.
              </p>
            </div>
          </div>
        ) : (
          <div className="bg-warning-50 dark:bg-warning-900/10 border border-warning-200 dark:border-warning-800 rounded-lg p-4 flex items-center gap-4">
            <div className="p-2 bg-warning-100 dark:bg-warning-900/30 rounded-full">
              <AlertTriangle className="size-6 text-warning-600 dark:text-warning-400" />
            </div>
            <div>
              <h4 className="font-semibold text-warning-800 dark:text-warning-200">
                Unsigned Agent
              </h4>
              <p className="text-sm text-warning-600 dark:text-warning-400">
                This agent is running in development mode. Sign it to deploy
                safely.
              </p>
            </div>
          </div>
        )}

        <div className="flex justify-end gap-2 mt-2">
          {isSigned ? (
            <Button
              variant="danger-soft"
              onPress={() => onUpdate({ signature: undefined })}
            >
              <Lock className="size-4 mr-2" />
              Unsign (Edit Mode)
            </Button>
          ) : (
            <Button variant="primary" onPress={handleSign}>
              <Shield className="size-4 mr-2" />
              Sign Agent
            </Button>
          )}
        </div>

        {isSigned && spec.signature && (
          <>
            <Separator className="my-2" />
            <div className="grid grid-cols-2 gap-4 text-xs font-mono">
              <div>
                <span className="text-default-400 block mb-1">Signer ID</span>
                <Chip size="sm" variant="soft" color="success">
                  {spec.signature.signer_id.substring(0, 12)}...
                </Chip>
              </div>
              <div>
                <span className="text-default-400 block mb-1">Timestamp</span>
                <span className="text-default-600">
                  {new Date(spec.signature.signed_at * 1000).toLocaleString()}
                </span>
              </div>
            </div>
          </>
        )}
      </Card.Content>
    </Card>
  );
}
