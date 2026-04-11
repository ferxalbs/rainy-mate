import { memo, useMemo } from "react";
import { LoaderCircle } from "lucide-react";

import { deriveMessagesTimelineRows } from "./MessagesTimeline.logic";
import { UserMessageRow } from "./entries/UserMessageRow";
import { AssistantMessageRow } from "./entries/AssistantMessageRow";
import { WorkEntryRow } from "./entries/WorkEntryRow";
import type { AgentMessage } from "../../../types/agent";

interface MessagesTimelineProps {
  messages: AgentMessage[];
  scrollContainer: HTMLDivElement | null;
}

export const MessagesTimeline = memo(function MessagesTimeline({
  messages,
}: MessagesTimelineProps) {
  const rows = useMemo(() => deriveMessagesTimelineRows(messages), [messages]);

  return (
    <div className="relative mx-auto w-full min-w-0 max-w-6xl overflow-x-hidden px-4 pb-8 pt-4 md:px-6">
      <div className="mx-auto mb-5 w-full max-w-[56rem]">
        <div className="inline-flex items-center gap-2 rounded-full border border-border/60 bg-card/70 px-3 py-1.5 text-[11px] text-muted-foreground backdrop-blur-xl">
          <span className="font-medium text-foreground/90">Timeline</span>
          <span>{messages.length} message{messages.length === 1 ? "" : "s"}</span>
        </div>
      </div>

      <div className="space-y-0">
        {rows.map((row) => {
          if (row.kind === "user-message" && row.message) {
            return <UserMessageRow key={row.id} message={row.message} />;
          }

          if (row.kind === "work-group" && row.groupedEntries) {
            return (
              <WorkEntryRow
                key={row.id}
                entries={row.groupedEntries}
                message={row.message}
                defaultExpanded={row.message?.runState === "running"}
              />
            );
          }

          if (row.kind === "assistant-message" && row.message) {
            return <AssistantMessageRow key={row.id} message={row.message} />;
          }

          if (row.kind === "working") {
            return (
              <div key={row.id} className="mb-6 flex w-full justify-start">
                <div className="inline-flex items-center gap-2 rounded-full border border-border/60 bg-card/75 px-3 py-2 text-xs text-muted-foreground backdrop-blur-xl">
                  <LoaderCircle className="size-3.5 animate-spin text-primary" />
                  {row.message?.statusText || "Awaiting first streamed tokens"}
                </div>
              </div>
            );
          }

          return null;
        })}
      </div>
    </div>
  );
});
