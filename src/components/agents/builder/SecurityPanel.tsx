import { Card, CardBody, CardHeader, Chip, Button } from "@heroui/react";
import { AgentSignature } from "../../../types/agent-spec";
import { ShieldCheck, ShieldAlert, Fingerprint } from "lucide-react";

interface SecurityPanelProps {
  signature?: AgentSignature;
  onSign: () => void;
  isSigning: boolean;
}

export function SecurityPanel({
  signature,
  onSign,
  isSigning,
}: SecurityPanelProps) {
  const isSigned = !!signature;

  return (
    <Card
      className={`w-full border-l-4 ${isSigned ? "border-l-success" : "border-l-warning"}`}
    >
      <CardHeader className="flex justify-between items-center">
        <div className="flex gap-2 items-center">
          {isSigned ? (
            <ShieldCheck className="text-success size-5" />
          ) : (
            <ShieldAlert className="text-warning size-5" />
          )}
          <div className="flex flex-col">
            <span className="font-bold text-sm">Security Verification</span>
            <span className="text-xs text-default-500">
              {isSigned
                ? "Agent is cryptographically signed."
                : "Unsigned agent - restricted capabilities."}
            </span>
          </div>
        </div>
        <Button
          size="sm"
          color={isSigned ? "success" : "warning"}
          variant={isSigned ? "flat" : "solid"}
          onPress={onSign}
          isLoading={isSigning}
          isDisabled={isSigned} // For now, only sign once
        >
          {isSigned ? "Verified" : "Sign Package"}
        </Button>
      </CardHeader>

      {isSigned && (
        <CardBody className="pt-0 pb-4">
          <div className="bg-default-50 p-3 rounded-lg flex flex-col gap-2 text-xs">
            <div className="flex justify-between">
              <span className="text-default-400">Signer ID:</span>
              <span
                className="font-mono text-foreground truncate max-w-[200px]"
                title={signature.signer_id}
              >
                {signature.signer_id.substring(0, 12)}...
              </span>
            </div>
            <div className="flex justify-between">
              <span className="text-default-400">Origin Device:</span>
              <span className="font-mono text-foreground">
                {signature.origin_device_id}
              </span>
            </div>
            <div className="flex justify-between items-center">
              <span className="text-default-400">Capabilities Hash:</span>
              <Chip size="sm" variant="dot" color="success">
                Verified
              </Chip>
            </div>
            <div className="mt-2 text-[10px] text-default-300 flex items-center gap-1">
              <Fingerprint className="size-3" />
              {signature.signature.substring(0, 32)}...
            </div>
          </div>
        </CardBody>
      )}
    </Card>
  );
}
