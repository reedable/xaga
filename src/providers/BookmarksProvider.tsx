import {
  useCallback,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import useAbortController from "../hooks/useAbortController";
import { usePlatform } from "../platform/Platform";
import type { BookmarkMap } from "./BookmarksContext";
import BookmarksContext from "./BookmarksContext";

export function BookmarksProvider({ children }: { children: ReactNode }) {
  const [abort, abortControllerRef] = useAbortController();
  const platform = usePlatform();
  const [error, setError] = useState<unknown>(null);
  const [loading, setLoading] = useState(false);
  const [bookmarkMap, setBookmarkMap] = useState<BookmarkMap>(new Map());

  const listBookmarks = useCallback(async () => {
    setError(null);
    setLoading(true);

    try {
      const signal = abortControllerRef.current.signal;
      const results = await platform.listBookmarks(signal);

      if (results) {
        const map: BookmarkMap = new Map();

        // Load the lookup table
        for (const entry of results) {
          map.set(entry.id, entry);
        }

        setBookmarkMap(map);
      }

      return results;
    } catch (e) {
      setError(e);
    } finally {
      setLoading(false);
    }
  }, [abortControllerRef, platform]);

  const value = useMemo(
    () => ({
      error,
      loading,
      bookmarkMap,
      setError,
      setLoading,
      setBookmarkMap,
    }),
    [bookmarkMap, error, loading],
  );

  const handleImportBookmarks = useCallback(async () => {
    const signal = abortControllerRef.current.signal;
    try {
      const result = await platform.importBookmarks(signal);
      if (result !== null && result !== undefined) {
        await listBookmarks();
      }
    } catch (e) {
      setError(e);
    }
  }, [abortControllerRef, platform, listBookmarks, setError]);

  const handleExportBookmarks = useCallback(async () => {
    const signal = abortControllerRef.current.signal;
    try {
      await platform.exportBookmarks(signal);
    } catch (e) {
      setError(e);
    }
  }, [abortControllerRef, platform, setError]);

  useEffect(() => {
    const unlisten = platform.onImportBookmarks(handleImportBookmarks);
    return () => { unlisten.then((fn) => fn()); };
  }, [platform, handleImportBookmarks]);

  useEffect(() => {
    const unlisten = platform.onExportBookmarks(handleExportBookmarks);
    return () => { unlisten.then((fn) => fn()); };
  }, [platform, handleExportBookmarks]);

  useEffect(() => {
    abort();

    (async () => {
      listBookmarks();
    })();
  }, [abort, listBookmarks]);

  return (
    <BookmarksContext.Provider value={value}>
      {children}
    </BookmarksContext.Provider>
  );
}
