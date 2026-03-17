import { useCallback, useEffect, useMemo, useRef, useState, type RefObject } from "react";

interface VirtualTranscriptItem {
  id: string;
}

interface UseVirtualTranscriptOptions<T extends VirtualTranscriptItem> {
  items: T[];
  viewportRef: RefObject<HTMLDivElement | null>;
  headerHeight?: number;
  overscan?: number;
  estimateSize?: (item: T, index: number) => number;
}

interface VirtualRow<T> {
  index: number;
  item: T;
}

const DEFAULT_OVERSCAN = 4;
const NEAR_BOTTOM_THRESHOLD_PX = 96;

function findNearestIndex(offsets: number[], target: number): number {
  let low = 0;
  let high = offsets.length - 1;

  while (low <= high) {
    const mid = Math.floor((low + high) / 2);
    if (offsets[mid] <= target) {
      low = mid + 1;
    } else {
      high = mid - 1;
    }
  }

  return Math.max(0, low - 1);
}

export function useVirtualTranscript<T extends VirtualTranscriptItem>({
  items,
  viewportRef,
  headerHeight = 0,
  overscan = DEFAULT_OVERSCAN,
  estimateSize,
}: UseVirtualTranscriptOptions<T>) {
  const sizeCacheRef = useRef(new Map<string, number>());
  const observersRef = useRef(new Map<string, ResizeObserver>());
  const refCallbacksRef = useRef(
    new Map<string, (node: HTMLDivElement | null) => void>(),
  );
  const rafRef = useRef<number | null>(null);
  const [measurementVersion, setMeasurementVersion] = useState(0);
  const [scrollState, setScrollState] = useState({
    scrollTop: 0,
    viewportHeight: 0,
    isNearBottom: true,
  });

  const estimate = useCallback(
    (item: T, index: number) => estimateSize?.(item, index) ?? 240,
    [estimateSize],
  );

  useEffect(() => {
    const viewport = viewportRef.current;
    if (!viewport) return;

    const syncScrollState = () => {
      setScrollState({
        scrollTop: viewport.scrollTop,
        viewportHeight: viewport.clientHeight,
        isNearBottom:
          viewport.scrollHeight - viewport.clientHeight - viewport.scrollTop <=
          NEAR_BOTTOM_THRESHOLD_PX,
      });
    };

    const handleScroll = () => {
      if (rafRef.current != null) return;
      rafRef.current = window.requestAnimationFrame(() => {
        rafRef.current = null;
        syncScrollState();
      });
    };

    syncScrollState();
    viewport.addEventListener("scroll", handleScroll, { passive: true });

    const viewportResizeObserver = new ResizeObserver(() => syncScrollState());
    viewportResizeObserver.observe(viewport);

    return () => {
      viewport.removeEventListener("scroll", handleScroll);
      viewportResizeObserver.disconnect();
      if (rafRef.current != null) {
        window.cancelAnimationFrame(rafRef.current);
        rafRef.current = null;
      }
    };
  }, [viewportRef]);

  useEffect(() => {
    const activeIds = new Set(items.map((item) => item.id));

    for (const [id, observer] of observersRef.current.entries()) {
      if (activeIds.has(id)) continue;
      observer.disconnect();
      observersRef.current.delete(id);
      sizeCacheRef.current.delete(id);
      refCallbacksRef.current.delete(id);
    }
  }, [items]);

  const measureRow = useCallback((id: string) => {
    const existingCallback = refCallbacksRef.current.get(id);
    if (existingCallback) return existingCallback;

    const callback = (node: HTMLDivElement | null) => {
      const existingObserver = observersRef.current.get(id);
      if (existingObserver) {
        existingObserver.disconnect();
        observersRef.current.delete(id);
      }

      if (!node) return;

      const syncHeight = () => {
        const nextHeight = Math.ceil(node.getBoundingClientRect().height);
        if (!Number.isFinite(nextHeight) || nextHeight <= 0) return;
        const prevHeight = sizeCacheRef.current.get(id);
        if (prevHeight === nextHeight) return;
        sizeCacheRef.current.set(id, nextHeight);
        setMeasurementVersion((version) => version + 1);
      };

      syncHeight();
      const observer = new ResizeObserver(syncHeight);
      observer.observe(node);
      observersRef.current.set(id, observer);
    };

    refCallbacksRef.current.set(id, callback);
    return callback;
  }, []);

  const metrics = useMemo(() => {
    if (items.length === 0) {
      return {
        paddingTop: 0,
        paddingBottom: 0,
        totalHeight: 0,
        visibleRows: [] as VirtualRow<T>[],
      };
    }

    const sizes = items.map((item, index) => sizeCacheRef.current.get(item.id) ?? estimate(item, index));
    const offsets = new Array<number>(items.length);
    let totalHeight = 0;

    for (let index = 0; index < sizes.length; index += 1) {
      offsets[index] = totalHeight;
      totalHeight += sizes[index];
    }

    const effectiveScrollTop = Math.max(0, scrollState.scrollTop - headerHeight);
    const effectiveViewportHeight = Math.max(
      0,
      scrollState.viewportHeight - headerHeight,
    );

    const firstVisibleIndex = findNearestIndex(offsets, effectiveScrollTop);
    const lastVisibleIndex = findNearestIndex(
      offsets,
      effectiveScrollTop + effectiveViewportHeight,
    );

    const startIndex = Math.max(0, firstVisibleIndex - overscan);
    const endIndex = Math.min(items.length - 1, lastVisibleIndex + overscan);

    const visibleRows: VirtualRow<T>[] = [];
    for (let index = startIndex; index <= endIndex; index += 1) {
      visibleRows.push({ index, item: items[index] });
    }

    const paddingTop = offsets[startIndex] ?? 0;
    const endOffset = offsets[endIndex] ?? 0;
    const paddingBottom = Math.max(0, totalHeight - endOffset - sizes[endIndex]);

    return {
      paddingTop,
      paddingBottom,
      totalHeight,
      visibleRows,
    };
  }, [
    estimate,
    headerHeight,
    items,
    measurementVersion,
    overscan,
    scrollState.scrollTop,
    scrollState.viewportHeight,
  ]);

  const scrollToBottom = useCallback(
    (behavior: ScrollBehavior = "auto") => {
      const viewport = viewportRef.current;
      if (!viewport) return;
      viewport.scrollTo({ top: viewport.scrollHeight, behavior });
    },
    [viewportRef],
  );

  return {
    isNearBottom: scrollState.isNearBottom,
    measureRow,
    paddingBottom: metrics.paddingBottom,
    paddingTop: metrics.paddingTop,
    scrollToBottom,
    totalHeight: metrics.totalHeight,
    visibleRows: metrics.visibleRows,
  };
}
