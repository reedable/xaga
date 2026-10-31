import { useCallback, useState, type RefObject } from "react";
import { usePlatform } from "../platform/Platform";
import type { Entry } from "../types";

export function useBookmarks(abortControllerRef: RefObject<AbortController>) {
  const platform = usePlatform();
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<unknown>(null);

  const addBookmark = useCallback(
    async (entry: Entry, color: string) => {
      const signal = abortControllerRef.current.signal;
      setError(null);
      setLoading(true);
      try {
        return await platform.addBookmark(signal, entry.id, color);
      } catch (e) {
        setError(e);
      } finally {
        setLoading(false);
      }
    },
    [abortControllerRef, platform]
  );

  const removeBookmark = useCallback(
    async (entry: Entry) => {
      const signal = abortControllerRef.current.signal;
      setError(null);
      setLoading(true);
      try {
        return await platform.removeBookmark(signal, entry.id);
      } catch (e) {
        setError(e);
      } finally {
        setLoading(false);
      }
    },
    [abortControllerRef, platform]
  );

  return { loading, error, addBookmark, removeBookmark };
}
