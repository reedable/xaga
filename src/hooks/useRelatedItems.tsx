import { useCallback, useState, type RefObject } from "react";
import { usePlatform } from "../platform/Platform";
import type { Filter } from "../types";

export default function useRelatedItems(
  abortControllerRef: RefObject<AbortController>
) {
  const platform = usePlatform();
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<unknown>(null);

  const queryByItem = useCallback(
    async (
      itemId: string,
      filter: Filter = "all",
      minSimilarity: number = 0.5,
      maxSimilarity?: number | null,
      max?: number
    ) => {
      const signal = abortControllerRef.current.signal;
      setError(null);
      setLoading(true);

      try {
        return await platform.queryByItem(
          signal,
          itemId,
          filter,
          minSimilarity,
          maxSimilarity ?? null,
          max ?? null
        );
      } catch (e) {
        setError(e);
      } finally {
        setLoading(false);
      }
    },
    [abortControllerRef, platform]
  );

  return { loading, error, queryByItem };
}
