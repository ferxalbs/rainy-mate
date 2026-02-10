import { Modal, Button } from "@heroui/react";
import { useAirlock } from "../../hooks";
import { ShieldCheck, ShieldAlert, Terminal } from "lucide-react";
import { AirlockLevel } from "../../types";

export function AirlockEvents() {
  const { pendingRequests, respond } = useAirlock();

  if (pendingRequests.length === 0) return null;

  // Process the first request in the queue
  const request = pendingRequests[0];

  // Format info
  const isDangerous = request.airlockLevel === AirlockLevel.Dangerous;

  // Try to format JSON payload if possible
  let formattedPayload = request.payloadSummary;
  try {
    const parsed = JSON.parse(request.payloadSummary);
    formattedPayload = JSON.stringify(parsed, null, 2);
  } catch (e) {
    // Keep as is
  }

  return (
    <Modal.Backdrop
      isOpen={true}
      onOpenChange={() => {}}
      className="backdrop-blur-3xl bg-black/60 z-50"
    >
      <Modal.Container className="scale-100 opacity-100">
        <Modal.Dialog
          className={`border relative overflow-hidden transition-all duration-300 rounded-[28px] shadow-2xl max-w-lg w-full ${
            isDangerous
              ? "border-red-500/30 shadow-[0_0_80px_-20px_rgba(239,68,68,0.2)] bg-zinc-950/80 backdrop-blur-3xl"
              : "border-white/10 shadow-[0_0_80px_-20px_rgba(255,255,255,0.05)] bg-zinc-950/80 backdrop-blur-3xl"
          }`}
        >
          {/* Subtle Gradient Glow */}
          <div
            className={`absolute top-0 inset-x-0 h-32 bg-gradient-to-b opacity-20 pointer-events-none ${
              isDangerous ? "from-red-500/30" : "from-blue-500/20"
            }`}
          />

          <Modal.Header className="relative z-10 p-6 pb-2">
            <div className="flex items-center gap-3">
              <div
                className={`flex items-center justify-center size-10 rounded-full border ${
                  isDangerous
                    ? "bg-red-500/10 border-red-500/20 text-red-500"
                    : "bg-blue-500/10 border-blue-500/20 text-blue-500"
                }`}
              >
                {isDangerous ? (
                  <ShieldAlert className="size-5" />
                ) : (
                  <ShieldCheck className="size-5" />
                )}
              </div>
              <div>
                <Modal.Heading className="text-xl font-semibold tracking-tight text-white">
                  {isDangerous
                    ? "Critical Operation"
                    : "Authentication Required"}
                </Modal.Heading>
                <p className="text-xs text-white/50 font-medium">
                  {request.commandId}
                </p>
              </div>
            </div>
          </Modal.Header>

          <Modal.Body className="relative z-10 px-6 py-4 space-y-5">
            <p className="text-sm text-white/70 leading-relaxed">
              An agent is requesting permission to execute the following
              <span
                className={`font-semibold ml-1 ${
                  isDangerous ? "text-red-400" : "text-blue-400"
                }`}
              >
                {request.intent}
              </span>{" "}
              action.
            </p>

            <div className="rounded-2xl border border-white/5 bg-black/40 overflow-hidden">
              <div className="flex items-center gap-2 px-4 py-2 border-b border-white/5 bg-white/5">
                <Terminal className="size-3 text-white/30" />
                <span className="text-[10px] font-medium text-white/40 uppercase tracking-wider">
                  Payload Preview
                </span>
              </div>
              <div className="max-h-64 overflow-y-auto scrollbar-thin scrollbar-thumb-white/10 scrollbar-track-transparent">
                <pre className="p-4 text-[11px] leading-relaxed font-mono text-white/80 whitespace-pre-wrap font-variant-ligatures-none">
                  {formattedPayload}
                </pre>
              </div>
            </div>

            {isDangerous && (
              <div className="flex gap-3 items-start p-3 rounded-xl border border-red-500/20 bg-red-500/5">
                <ShieldAlert className="size-4 text-red-500 shrink-0 mt-0.5" />
                <p className="text-xs text-red-200/80 leading-relaxed">
                  This action can modify system state or files. Only approve if
                  you are confident in the agent's intent.
                </p>
              </div>
            )}
          </Modal.Body>

          <Modal.Footer className="relative z-10 p-6 pt-2">
            <div className="flex gap-3 justify-end w-full">
              <Button
                variant="ghost"
                onPress={() => respond(request.commandId, false)}
                className="rounded-full px-6 h-10 text-sm font-medium text-white/60 hover:text-white hover:bg-white/5"
              >
                Reject
              </Button>
              <Button
                variant="primary"
                onPress={() => respond(request.commandId, true)}
                className={`rounded-full px-6 h-10 text-sm font-medium shadow-lg transition-all active:scale-95 ${
                  isDangerous
                    ? "bg-red-600 hover:bg-red-500 text-white shadow-red-500/20"
                    : "bg-white text-black hover:bg-white/90 shadow-white/10"
                }`}
              >
                {isDangerous ? "Authorize Execution" : "Approve Action"}
              </Button>
            </div>
          </Modal.Footer>
        </Modal.Dialog>
      </Modal.Container>
    </Modal.Backdrop>
  );
}
