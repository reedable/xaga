import { useCallback, useState, type RefObject } from "react";
import { usePlatform } from "../platform/Platform";
import type { Feed, Topic } from "../types";

export function useCollections(abortControllerRef: RefObject<AbortController>) {
  const platform = usePlatform();
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<unknown>(null);

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

  return { loading, error, listFeeds, listTopics };
}
