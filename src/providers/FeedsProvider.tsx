import {
  useCallback,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { subscribe } from "../hooks/useEvents";
import { usePlatform } from "../platform/Platform";
import type { Feed } from "../types";
import { debounced } from "../utils/Decorator";
import sortByUnreadCount from "../utils/sortByUnreadCount";
import FeedsContext, { type FeedsContextType } from "./FeedsContext";

export default function FeedsProvider({ children }: { children: ReactNode }) {
  const platform = usePlatform();
  const [error, setError] = useState<unknown>(null);
  const [loading, setLoading] = useState(false);
  const [loadingSet, setLoadingSet] = useState<Set<string>>(new Set());
  const [feeds, setFeeds] = useState<Feed[]>([]);

  // Pinned state lives on each feed's `.atom` (surfaced as `isPinned`); derive the id-set.
  const pinnedFeedIds = useMemo(
    () => feeds.filter((f) => f.isPinned).map((f) => f.id),
    [feeds]
  );

  // Optimistically flip the feed's `isPinned`, then persist to the `.atom` (fire-and-forget).
  const setFeedPinned = useCallback(
    (id: Feed["id"], pinned: boolean) => {
      setFeeds((prev) =>
        prev.map((f) => (f.id === id ? { ...f, isPinned: pinned } : f))
      );
      platform.setFeedPinned(id, pinned).catch(console.error);
    },
    [platform]
  );

  const refreshFeeds = useCallback(async () => {
    const results = await platform.listFeeds(new AbortController().signal);
    if (results) setFeeds([...results].sort(sortByUnreadCount));
  }, [platform]);

  useEffect(
    () =>
      subscribe(() =>
        platform.onFeedRefreshed((updatedFeed) => {
          setFeeds((prev) =>
            prev
              .map((f) => (f.id === updatedFeed.id ? updatedFeed : f))
              .sort(sortByUnreadCount)
          );
        })
      ),
    [platform]
  );

  useEffect(() => {
    const refresh = debounced(refreshFeeds, { wait: 1000 });
    return subscribe(() => platform.onUnreadChanged(refresh));
  }, [platform, refreshFeeds]);

  const handleImportFeeds = useCallback(async () => {
    const signal = new AbortController().signal;
    setError(null);
    try {
      const result = await platform.importFeeds(signal);
      if (result !== null && result !== undefined) {
        const results = await platform.listFeeds(signal);
        if (results) setFeeds([...results].sort(sortByUnreadCount));
      }
    } catch (e) {
      setError(e);
    }
  }, [platform, setFeeds]);

  const handleExportFeeds = useCallback(async () => {
    const signal = new AbortController().signal;
    setError(null);
    try {
      await platform.exportFeeds(signal);
    } catch (e) {
      setError(e);
    }
  }, [platform]);

  useEffect(
    () => subscribe(() => platform.onImportFeeds(handleImportFeeds)),
    [platform, handleImportFeeds]
  );

  useEffect(
    () => subscribe(() => platform.onExportFeeds(handleExportFeeds)),
    [platform, handleExportFeeds]
  );

  const value = useMemo<FeedsContextType>(
    () => ({
      error,
      loading,
      loadingFeedIds: loadingSet,
      feeds,
      pinnedFeedIds,
      setError,
      setLoading,
      setLoadingSet,
      setFeeds,
      setFeedPinned,
    }),
    [error, loading, loadingSet, feeds, pinnedFeedIds, setFeedPinned]
  );

  return (
    <FeedsContext.Provider value={value}>{children}</FeedsContext.Provider>
  );
}
