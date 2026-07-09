import { useCallback, useContext, useState, type RefObject } from "react";
import { usePlatform } from "../platform/Platform";
import ExpandedListContext from "../providers/ExpandedListContext";
import type { Feed, Topic } from "../types";

export type { ListType } from "../providers/ExpandedListContext";

export function useSidebar(abortControllerRef: RefObject<AbortController>) {
  const platform = usePlatform();
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<unknown>(null);

  // Which sidebar section is expanded is shared view state (persisted client-side via
  // `ExpandedListProvider`), so every consumer reads one value rather than each fetching it.
  const expandedContext = useContext(ExpandedListContext);
  if (!expandedContext) {
    throw new Error("useSidebar must be used inside ExpandedListProvider");
  }
  const { expandedList, setExpandedList } = expandedContext;

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
