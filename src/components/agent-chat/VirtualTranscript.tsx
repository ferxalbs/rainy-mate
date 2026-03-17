import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { Button } from "../ui/button";
import { cn } from "../../lib/utils";
import { useVirtualTranscript } from "../../hooks/agent-chat/useVirtualTranscript";

interface VirtualTranscriptItem {
  id: string;
}

interface VirtualTranscriptProps<T extends VirtualTranscriptItem> {
  items: T[];
  renderItem: (item: T, index: number) => React.ReactNode;
  header?: React.ReactNode;
  className?: string;
  contentClassName?: string;
  estimateSize?: (item: T, index: number) => number;
  hasMoreHistory?: boolean;
  isHydratingHistory?: boolean;
  onLoadOlderHistory?: () => Promise<void> | void;
}

export function VirtualTranscript<T extends VirtualTranscriptItem>({
  items,
  renderItem,
  header,
  className,
  contentClassName,
  estimateSize,
  hasMoreHistory = false,
  isHydratingHistory = false,
  onLoadOlderHistory,
}: VirtualTranscriptProps<T>) {
  const rowGapPx = 32;
  const viewportRef = useRef<HTMLDivElement>(null);
  const headerRef = useRef<HTMLDivElement>(null);
  const prevLengthRef = useRef(items.length);
  const prevTotalHeightRef = useRef(0);
  const [headerHeight, setHeaderHeight] = useState(0);

  const {
    isNearBottom,
    measureRow,
    paddingBottom,
    paddingTop,
    scrollToBottom,
    totalHeight,
    visibleRows,
  } = useVirtualTranscript({
    items,
    viewportRef,
    headerHeight,
    estimateSize: (item, index) =>
      (estimateSize?.(item, index) ?? 240) + (index === items.length - 1 ? 0 : rowGapPx),
  });

  useEffect(() => {
    const headerNode = headerRef.current;
    if (!headerNode) {
      setHeaderHeight(0);
      return;
    }

    const syncHeaderHeight = () => {
      setHeaderHeight(Math.ceil(headerNode.getBoundingClientRect().height));
    };

    syncHeaderHeight();
    const observer = new ResizeObserver(syncHeaderHeight);
    observer.observe(headerNode);

    return () => observer.disconnect();
  }, [header]);

  useEffect(() => {
    const previousLength = prevLengthRef.current;
    prevLengthRef.current = items.length;
    const previousTotalHeight = prevTotalHeightRef.current;
    prevTotalHeightRef.current = totalHeight;

    if (items.length === 0) return;
    if (totalHeight <= previousTotalHeight && items.length <= previousLength) return;
    if (!isNearBottom && previousTotalHeight > 0) return;

    const frame = window.requestAnimationFrame(() => {
      const behavior =
        items.length > previousLength && previousLength > 0 ? "smooth" : "auto";
      scrollToBottom(behavior);
    });

    return () => window.cancelAnimationFrame(frame);
  }, [isNearBottom, items.length, scrollToBottom, totalHeight]);

  const handleLoadOlder = useCallback(async () => {
    if (!onLoadOlderHistory || !viewportRef.current || isHydratingHistory) return;

    const viewport = viewportRef.current;
    const previousScrollHeight = viewport.scrollHeight;
    const previousScrollTop = viewport.scrollTop;

    await onLoadOlderHistory();

    window.requestAnimationFrame(() => {
      const nextViewport = viewportRef.current;
      if (!nextViewport) return;
      const delta = nextViewport.scrollHeight - previousScrollHeight;
      nextViewport.scrollTop = previousScrollTop + delta;
    });
  }, [isHydratingHistory, onLoadOlderHistory]);

  const messageRegionStyle = useMemo(
    () => ({ minHeight: `${Math.max(totalHeight, 1)}px` }),
    [totalHeight],
  );

  return (
    <div
      ref={viewportRef}
      className={cn("absolute inset-0 z-10 h-full w-full overflow-y-auto", className)}
    >
      <div className={cn("mx-auto flex w-full max-w-6xl flex-col px-4 pb-44 pt-24 md:px-6", contentClassName)}>
        <div ref={headerRef} className="space-y-8">
          {header}

          {hasMoreHistory && (
            <div className="flex justify-center">
              <Button
                size="sm"
                variant="ghost"
                onClick={handleLoadOlder}
                disabled={isHydratingHistory}
                className="rounded-full border border-white/10 bg-background/80 px-4 backdrop-blur-sm backdrop-saturate-150 dark:bg-background/10"
              >
                {isHydratingHistory ? "Loading..." : "Load older messages"}
              </Button>
            </div>
          )}
        </div>

        <div className="mt-8" style={messageRegionStyle}>
          <div style={{ paddingTop, paddingBottom }}>
            {visibleRows.map(({ index, item }) => (
              <div
                key={item.id}
                ref={measureRow(item.id)}
                className={index === items.length - 1 ? undefined : "pb-8"}
                style={{ contain: "layout paint style" }}
              >
                {renderItem(item, index)}
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
