import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { RefObject } from 'react';

export interface ProgressiveGridOptions {
  initialCount?: number;
  incrementCount?: number;
  preloadDistancePx?: number;
}

export interface ProgressiveGridResult<T> {
  containerRef: RefObject<HTMLDivElement | null>;
  sentinelRef: RefObject<HTMLDivElement | null>;
  visibleItems: T[];
  visibleCount: number;
  totalCount: number;
  remainingCount: number;
  hasMore: boolean;
  loadMore(): void;
}

const DEFAULT_INITIAL_COUNT = 180;
const DEFAULT_INCREMENT_COUNT = 120;
const DEFAULT_PRELOAD_DISTANCE_PX = 900;

/**
 * Progressive rendering for large explorer folders.
 *
 * The explorer still keeps the full sorted item list for selection, Shift selection,
 * Ctrl+A, and search state. Only the visible card DOM is rendered in batches, so a
 * folder with thousands of images does not mount thousands of thumbnail components
 * at once.
 */
export function useProgressiveGridItems<T extends { id: string }>(
  items: T[],
  options: ProgressiveGridOptions = {},
): ProgressiveGridResult<T> {
  const initialCount = options.initialCount ?? DEFAULT_INITIAL_COUNT;
  const incrementCount = options.incrementCount ?? DEFAULT_INCREMENT_COUNT;
  const preloadDistancePx = options.preloadDistancePx ?? DEFAULT_PRELOAD_DISTANCE_PX;
  const [visibleCount, setVisibleCount] = useState(() => Math.min(initialCount, items.length));
  const containerRef = useRef<HTMLDivElement | null>(null);
  const sentinelRef = useRef<HTMLDivElement | null>(null);

  const listIdentity = useMemo(() => {
    const first = items[0]?.id ?? '';
    const last = items[items.length - 1]?.id ?? '';
    return `${items.length}:${first}:${last}`;
  }, [items]);

  useEffect(() => {
    setVisibleCount(Math.min(initialCount, items.length));
    if (containerRef.current) {
      containerRef.current.scrollTop = 0;
    }
  }, [initialCount, items.length, listIdentity]);

  const loadMore = useCallback(() => {
    setVisibleCount((current) => Math.min(items.length, current + incrementCount));
  }, [incrementCount, items.length]);

  useEffect(() => {
    const container = containerRef.current;
    const sentinel = sentinelRef.current;
    if (!container || !sentinel || visibleCount >= items.length) return;

    if ('IntersectionObserver' in window) {
      const observer = new IntersectionObserver(
        (entries) => {
          if (entries.some((entry) => entry.isIntersecting)) {
            loadMore();
          }
        },
        {
          root: container,
          rootMargin: `${preloadDistancePx}px 0px`,
          threshold: 0.01,
        },
      );

      observer.observe(sentinel);
      return () => observer.disconnect();
    }

    const handleScroll = () => {
      const distanceToBottom = container.scrollHeight - container.scrollTop - container.clientHeight;
      if (distanceToBottom <= preloadDistancePx) {
        loadMore();
      }
    };

    container.addEventListener('scroll', handleScroll, { passive: true });
    handleScroll();
    return () => container.removeEventListener('scroll', handleScroll);
  }, [items.length, loadMore, preloadDistancePx, visibleCount]);

  const visibleItems = useMemo(() => items.slice(0, visibleCount), [items, visibleCount]);
  const remainingCount = Math.max(0, items.length - visibleItems.length);

  return {
    containerRef,
    sentinelRef,
    visibleItems,
    visibleCount: visibleItems.length,
    totalCount: items.length,
    remainingCount,
    hasMore: remainingCount > 0,
    loadMore,
  };
}
