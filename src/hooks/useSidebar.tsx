import { useCallback, useEffect, useState, type RefObject } from "react";
import { usePlatform } from "../platform/Platform";
import type { Feed, Topic } from "../types";

export type ListType = "SHORTCUTS" | "TOPICS" | "FEEDS";

export function useSidebar(abortControllerRef: RefObject<AbortController>) {
  const platform = usePlatform();
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<unknown>(null);
  const [expandedList, setExpandedListState] = useState<ListType | null>(
    "SHORTCUTS"
  );

  useEffect(() => {
    platform
      .getExpandedSidebarList()
      .then((v) => setExpandedListState((v as ListType | null) ?? "SHORTCUTS"))
      .catch(console.error);
  }, [platform]);

  const setExpandedList = useCallback(
    (
      updater: ((prev: ListType | null) => ListType | null) | (ListType | null)
    ) => {
      setExpandedListState((prev) => {
        const next = typeof updater === "function" ? updater(prev) : updater;
        platform.setExpandedSidebarList(next).catch(console.error);
        return next;
      });
    },
    [platform]
  );

  const listFeeds = useCallback(
    async (compare?: (a: Feed, b: Feed) => number) => {
      const signal = abortControllerRef.current.signal;
      setLoading(true);

      try {
        const results = await platform.listFeeds(signal);
        return compare ? results?.sort(compare) : results;
      } catch (e) {
        setError(e);
      } finally {
        setLoading(false);
      }
    },
    [abortControllerRef, platform]
  );

  const listTopics = useCallback(
    async (compare?: (a: Topic, b: Topic) => number) => {
      const signal = abortControllerRef.current.signal;
      setLoading(true);

      try {
        const results = await platform.listTopics(signal);
        return compare ? results?.sort(compare) : results;
      } catch (e) {
        setError(e);
      } finally {
        setLoading(false);
      }
    },
    [abortControllerRef, platform]
  );

  return {
    loading,
    error,
    listFeeds,
    listTopics,
    expandedList,
    setExpandedList,
  };
}
