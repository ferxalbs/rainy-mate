import { memo, useMemo, useRef, useEffect, useState, useLayoutEffect } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { deriveMessagesTimelineRows, estimateTimelineRowHeight } from "./MessagesTimeline.logic";
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
  scrollContainer,
}: MessagesTimelineProps) {
  const timelineRootRef = useRef<HTMLDivElement | null>(null);
  const [timelineWidthPx, setTimelineWidthPx] = useState<number | null>(null);

  useLayoutEffect(() => {
    const timelineRoot = timelineRootRef.current;
    if (!timelineRoot) return;

    const updateWidth = (nextWidth: number) => {
      setTimelineWidthPx((previousWidth) => {
        if (previousWidth !== null && Math.abs(previousWidth - nextWidth) < 0.5) {
          return previousWidth;
        }
        return nextWidth;
      });
    };

    updateWidth(timelineRoot.getBoundingClientRect().width);

    if (typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(() => {
      updateWidth(timelineRoot.getBoundingClientRect().width);
    });
    observer.observe(timelineRoot);
    return () => {
      observer.disconnect();
    };
  }, []);

  const rows = useMemo(() => deriveMessagesTimelineRows(messages), [messages]);

  const virtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => scrollContainer,
    estimateSize: (index: number) => {
      const row = rows[index];
      if (!row) return 80;
      return estimateTimelineRowHeight(row, timelineWidthPx);
    },
    useAnimationFrameWithResizeObserver: true,
    overscan: 10,
    getItemKey: (index: number) => rows[index]?.id || `idx-${index}`,
  });

  useEffect(() => {
    if (timelineWidthPx === null) return;
    virtualizer.measure();
  }, [virtualizer, timelineWidthPx, rows.length]);

  return (
    <div
      ref={timelineRootRef}
      className="mx-auto w-full min-w-0 max-w-4xl overflow-x-hidden pt-4 pb-20 px-4 md:px-6 relative"
    >
      <div
         className="relative w-full"
         style={{ height: `${virtualizer.getTotalSize()}px` }}
      >
        {virtualizer.getVirtualItems().map((virtualRow) => {
          const row = rows[virtualRow.index];
          if (!row) return null;

          return (
            <div
              key={virtualRow.key}
              data-index={virtualRow.index}
              ref={virtualizer.measureElement}
              className="absolute left-0 top-0 w-full mb-6"
              style={{
                transform: `translateY(${virtualRow.start}px)`,
              }}
            >
              {row.kind === "user-message" && row.message && (
                <UserMessageRow message={row.message} />
              )}
              
              {row.kind === "work-group" && row.groupedEntries && (
                <WorkEntryRow entries={row.groupedEntries} defaultExpanded={row.message?.runState === "running"} />
              )}
              
              {row.kind === "assistant-message" && row.message && (
                <AssistantMessageRow message={row.message} />
              )}
              
              {row.kind === "working" && (
                <div className="w-full justify-start flex mb-6 py-2 px-3">
                  <div className="flex items-center gap-2">
                     <span className="inline-flex items-center gap-[3px] text-muted-foreground/60">
                        <span className="h-1.5 w-1.5 rounded-full bg-current animate-pulse" />
                        <span className="h-1.5 w-1.5 rounded-full bg-current animate-pulse [animation-delay:200ms]" />
                        <span className="h-1.5 w-1.5 rounded-full bg-current animate-pulse [animation-delay:400ms]" />
                     </span>
                     <span className="text-[11px] font-medium tracking-wide uppercase text-muted-foreground/50">Working</span>
                  </div>
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
});
