import { Modal, Button } from "@heroui/react";
import { ShieldCheck, ShieldAlert, Terminal } from "lucide-react";

import { useAirlock } from "../../hooks";
import { AirlockLevel } from "../../types";

export function AirlockEvents() {
  const { pendingRequests, respond } = useAirlock();

  if (pendingRequests.length === 0) return null;

  const request = pendingRequests[0];
  const isDangerous = request.airlockLevel === AirlockLevel.Dangerous;
  const hasExpiry = typeof request.timeoutSecs === "number" && request.timeoutSecs > 0;

  let formattedPayload = request.payloadSummary;
  try {
    const parsed = JSON.parse(request.payloadSummary);
    formattedPayload = JSON.stringify(parsed, null, 2);
  } catch {
    // Keep the original summary when it is not JSON.
  }

  return (
    <Modal.Backdrop
      isOpen={true}
      onOpenChange={() => {}}
      className="z-50 bg-background/55 dark:bg-background/25 backdrop-blur-3xl"
    >
      <Modal.Container className="scale-100 opacity-100">
        <Modal.Dialog
          className={`relative w-full max-w-lg overflow-hidden rounded-[28px] border shadow-2xl transition-all duration-300 ${
            isDangerous
              ? "border-red-500/30 bg-background/70 dark:bg-background/30 backdrop-blur-3xl"
              : "border-white/10 bg-background/70 dark:bg-background/30 backdrop-blur-3xl"
          }`}
        >
          <div
            className={`pointer-events-none absolute inset-x-0 top-0 h-32 bg-gradient-to-b opacity-20 ${
              isDangerous ? "from-red-500/30" : "from-blue-500/20"
            }`}
          />

          <Modal.Header className="relative z-10 p-6 pb-2">
            <div className="flex items-center gap-3">
              <div
                className={`flex size-10 items-center justify-center rounded-full border ${
                  isDangerous
                    ? "border-red-500/20 bg-red-500/10 text-red-500"
                    : "border-blue-500/20 bg-blue-500/10 text-blue-500"
                }`}
              >
                {isDangerous ? (
                  <ShieldAlert className="size-5" />
                ) : (
                  <ShieldCheck className="size-5" />
                )}
              </div>
              <div>
                <Modal.Heading className="text-xl font-semibold tracking-tight text-foreground">
                  {isDangerous ? "Critical Operation" : "Authentication Required"}
                </Modal.Heading>
                <p className="text-xs font-medium text-muted-foreground">
                  {request.commandId}
                </p>
                <p className="text-[11px] text-muted-foreground/80">
                  {hasExpiry
                    ? `Expires in ${request.timeoutSecs}s if unresolved`
                    : "Awaiting explicit approval"}
                </p>
              </div>
            </div>
          </Modal.Header>

          <Modal.Body className="relative z-10 space-y-5 px-6 py-4">
            <p className="text-sm leading-relaxed text-foreground/80">
              An agent is requesting permission to execute the following
              <span
                className={`ml-1 font-semibold ${
                  isDangerous ? "text-red-400 dark:text-red-300" : "text-blue-500 dark:text-blue-300"
                }`}
              >
                {request.intent}
              </span>{" "}
              action.
            </p>

            <div className="overflow-hidden rounded-2xl border border-border/60 bg-background/45 dark:bg-background/20">
              <div className="flex items-center gap-2 border-b border-border/50 bg-foreground/[0.03] px-4 py-2 dark:bg-white/[0.03]">
                <Terminal className="size-3 text-muted-foreground" />
                <span className="text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
                  Payload Preview
                </span>
              </div>
              <div className="max-h-64 overflow-y-auto scrollbar-thin scrollbar-thumb-white/10 scrollbar-track-transparent">
                <pre className="p-4 font-mono text-[11px] leading-relaxed whitespace-pre-wrap text-foreground/85 [font-variant-ligatures:none]">
                  {formattedPayload}
                </pre>
              </div>
            </div>

            {isDangerous && (
              <div className="flex items-start gap-3 rounded-xl border border-red-500/20 bg-red-500/5 p-3">
                <ShieldAlert className="mt-0.5 size-4 shrink-0 text-red-500" />
                <p className="text-xs leading-relaxed text-red-600 dark:text-red-200/80">
                  This action can modify system state or files. Only approve if
                  you are confident in the agent&apos;s intent.
                </p>
              </div>
            )}
          </Modal.Body>

          <Modal.Footer className="relative z-10 p-6 pt-2">
            <div className="flex w-full justify-end gap-3">
              <Button
                variant="ghost"
                onPress={() => respond(request.commandId, false)}
                className="h-10 rounded-full px-6 text-sm font-medium text-muted-foreground hover:bg-foreground/5 hover:text-foreground"
              >
                Reject
              </Button>
              <Button
                variant="primary"
                onPress={() => respond(request.commandId, true)}
                className={`h-10 rounded-full px-6 text-sm font-medium shadow-lg transition-all active:scale-95 ${
                  isDangerous
                    ? "bg-red-600 text-white shadow-red-500/20 hover:bg-red-500"
                    : "bg-foreground text-background hover:opacity-90"
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
