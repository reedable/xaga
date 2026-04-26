import { useCallback, useContext, type RefObject } from "react";
import { usePlatform } from "../platform/Platform";
import BookmarksContext from "../providers/BookmarksContext";
import type { Entry } from "../types";

export function useBookmarks(abortControllerRef: RefObject<AbortController>) {
  const context = useContext(BookmarksContext);
  const platform = usePlatform();

  if (!context) {
    throw new TypeError("useBookmarks must be used within BookmarksProvider");
  }

  const { error, loading, bookmarkMap, setError, setLoading, setBookmarkMap } =
    context;

  const addBookmark = useCallback(
    async (entry: Entry) => {
      const signal = abortControllerRef.current.signal;
      setError(null);
      setLoading(true);

      try {
        const result = await platform.addBookmark(signal, entry.id);
        return setBookmarkMap((prev) => new Map(prev).set(result.id, result));
      } catch (e) {
        setError(e);
      } finally {
        setLoading(false);
      }
    },
    [abortControllerRef, platform, setBookmarkMap, setError, setLoading],
  );

  const removeBookmark = useCallback(
    async (entry: Entry) => {
      const signal = abortControllerRef.current.signal;
      setError(null);
      setLoading(true);

      try {
        const result = await platform.removeBookmark(signal, entry.id);

        setBookmarkMap((prev) => {
          const newValue = new Map(prev);
          newValue.delete(result);
          return newValue;
        });
        return result;
      } catch (e) {
        setError(e);
      } finally {
        setLoading(false);
      }
    },
    [abortControllerRef, platform, setBookmarkMap, setError, setLoading],
  );

  return {
    loading,
    error,
    bookmarkMap,
    addBookmark,
    removeBookmark,
  };
}
