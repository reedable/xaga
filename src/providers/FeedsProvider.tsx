import {
  useCallback,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
  type SetStateAction,
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
  const [pinnedFeedIds, setPinnedFeedIdsState] = useState<Feed["id"][]>([]);

  useEffect(() => {
    platform
      .getPinnedFeedIds()
      .then(setPinnedFeedIdsState)
      .catch(console.error);
  }, [platform]);

  const setPinnedFeedIds = useCallback(
    (updater: SetStateAction<Feed["id"][]>) => {
      setPinnedFeedIdsState((prev) => {
        const next = typeof updater === "function" ? updater(prev) : updater;
        platform.setPinnedFeedIds(next).catch(console.error);
        return next;
      });
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
      setPinnedFeedIds,
    }),
    [error, loading, loadingSet, feeds, pinnedFeedIds, setPinnedFeedIds]
  );

  return (
    <FeedsContext.Provider value={value}>{children}</FeedsContext.Provider>
  );
}
