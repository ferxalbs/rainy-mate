import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button, Input, Separator } from "@heroui/react";

export function AtmBootstrap() {
  const [masterKey, setMasterKey] = useState("");
  const [workspaceName, setWorkspaceName] = useState("");
  const [status, setStatus] = useState<
    "idle" | "loading" | "success" | "error"
  >("idle");
  const [result, setResult] = useState<any>(null);
  const [error, setError] = useState("");
  const [agentResult, setAgentResult] = useState<any>(null);
  const [isDeploying, setIsDeploying] = useState(false);

  const handleBootstrap = async () => {
    setStatus("loading");
    setError("");
    try {
      const res = await invoke("bootstrap_atm", {
        masterKey,
        name: workspaceName,
      });
      setResult(res);
      // Credentials are set in the backend automatically
      setStatus("success");
    } catch (err: any) {
      console.error(err);
      setError(typeof err === "string" ? err : JSON.stringify(err));
      setStatus("error");
    }
  };

  const handleDeployAgent = async () => {
    setIsDeploying(true);
    try {
      // Default "Echo" agent for testing
      const config = {
        model: "gpt-4-turbo",
        systemPrompt: "You are a helpful assistant deployed via Rainy ATM.",
      };
      const res = await invoke("create_atm_agent", {
        name: "First Agent",
        type_: "default",
        config: config,
      });
      setAgentResult(res);
    } catch (err: any) {
      setError(typeof err === "string" ? err : JSON.stringify(err));
    } finally {
      setIsDeploying(false);
    }
  };

  return (
    <div className="p-8 max-w-2xl mx-auto">
      <div className="w-full bg-content1 rounded-xl shadow-sm border border-default-200">
        <div className="flex flex-col items-start px-6 pt-6 mb-4">
          <h2 className="text-2xl font-bold">🌧️ Rainy ATM Bootstrap</h2>
          <p className="text-default-500">
            Connect your desktop app to the Rainy Cloud Runtime
          </p>
        </div>

        <Separator />

        <div className="gap-4 px-6 py-6">
          {status === "success" ? (
            <div className="space-y-4">
              <div className="bg-success-50 text-success-600 p-4 rounded-lg">
                <h3 className="font-bold">✅ Workspace Created!</h3>
                <div className="mt-2 text-sm font-mono whitespace-pre-wrap">
                  {JSON.stringify(result, null, 2)}
                </div>
                <p className="mt-4 text-xs text-default-500">
                  The API Key has been automatically saved to your session.
                </p>
              </div>

              <Separator />

              <div className="flex flex-col gap-2 pt-2">
                <h3 className="font-bold text-lg">🚀 Deploy First Agent</h3>
                <p className="text-small text-default-500">
                  Deploy a test agent to verify the cloud runtime.
                </p>
                {agentResult ? (
                  <div className="bg-primary-50 text-primary-600 p-4 rounded-lg">
                    <h4 className="font-bold">Agent Deployed!</h4>
                    <div className="text-xs font-mono">
                      {JSON.stringify(agentResult, null, 2)}
                    </div>
                  </div>
                ) : (
                  <div>
                    <Button
                      className="bg-secondary text-white"
                      onPress={handleDeployAgent}
                      isDisabled={isDeploying}
                    >
                      {isDeploying ? "Deploying..." : 'Deploy "First Agent"'}
                    </Button>
                  </div>
                )}
              </div>
            </div>
          ) : (
            <div className="flex flex-col gap-4">
              <div className="flex flex-col gap-1">
                <label className="text-sm font-medium">
                  Master Key (Encryption Key)
                </label>
                <Input
                  placeholder="Enter the 32-byte hex key..."
                  value={masterKey}
                  onChange={(e) => setMasterKey(e.target.value)}
                  type="password"
                />
              </div>
              <div className="flex flex-col gap-1">
                <label className="text-sm font-medium">Workspace Name</label>
                <Input
                  placeholder="e.g. My Agency"
                  value={workspaceName}
                  onChange={(e) => setWorkspaceName(e.target.value)}
                />
              </div>
              {status === "error" && (
                <div className="bg-danger-50 text-danger-600 p-3 rounded-lg text-sm">
                  🚨 Error: {error}
                </div>
              )}
            </div>
          )}
        </div>

        <Separator />

        <div className="px-6 pb-6 pt-4 flex justify-end">
          {status !== "success" && (
            <Button
              className="bg-primary text-white"
              isDisabled={!masterKey || status === "loading"}
              onPress={handleBootstrap}
            >
              {status === "loading"
                ? "Initializing..."
                : "Initialize Workspace"}
            </Button>
          )}
        </div>
      </div>
    </div>
  );
}
