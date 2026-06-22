import { useCallback, useContext, useEffect, type RefObject } from "react";
import { usePlatform } from "../platform/Platform";
import FeedsContext from "../providers/FeedsContext";
import type { Feed } from "../types";
import { debounced, decorate } from "../utils/Decorator";
import sortByUnreadCount from "../utils/sortByUnreadCount";
import { useSidebar } from "./useSidebar";

export function useFeeds(abortControllerRef: RefObject<AbortController>) {
  const context = useContext(FeedsContext);

  if (!context) {
    throw new Error("useFeeds must be used inside FeedsProvider");
  }

  const {
    error,
    loading,
    loadingFeedIds,
    feeds,
    pinnedFeedIds,
    setError,
    setLoading,
    setLoadingSet,
    setFeeds,
    setFeedPinned,
  } = context;

  const platform = usePlatform();
  const { listFeeds } = useSidebar(abortControllerRef);

  const refreshAllFeeds = useCallback(
    async (callback: (feed: Feed) => void = () => {}) => {
      const signal = abortControllerRef.current.signal;
      const cb = decorate(callback).with(debounced, { wait: 400 });
      setError(null);
      try {
        const list = await platform.listFeeds(signal);
        if (!list) return;
        // One backend call refreshes every feed; mark them all loading for its duration.
        setLoadingSet(new Set(list.map((feed) => feed.id)));
        const refreshed = await platform.refreshAllFeeds(signal);
        refreshed?.forEach((feed) => cb(feed));
      } catch (e) {
        setError(e);
      } finally {
        setLoadingSet(new Set());
      }
    },
    [abortControllerRef, platform, setError, setLoadingSet]
  );

  const addFeed = useCallback(
    async (feedUrl: string) => {
      const signal = abortControllerRef.current.signal;
      setError(null);
      try {
        return await platform.addFeed(signal, feedUrl);
      } catch (e) {
        setError(e);
      }
    },
    [abortControllerRef, platform, setError]
  );

  const renameFeed = useCallback(
    async (feedId: string, title: string) => {
      const signal = abortControllerRef.current.signal;
      setError(null);
      try {
        return await platform.renameFeed(signal, feedId, title);
      } catch (e) {
        setError(e);
      }
    },
    [abortControllerRef, platform, setError]
  );

  const deleteFeed = useCallback(
    async (feedUrl: string) => {
      const signal = abortControllerRef.current.signal;
      setError(null);
      try {
        return await platform.removeFeed(signal, feedUrl);
      } catch (e) {
        setError(e);
      }
    },
    [abortControllerRef, platform, setError]
  );

  const requestFeeds = useCallback(async () => {
    const results = await listFeeds(sortByUnreadCount);
    if (results) setFeeds(results);
  }, [listFeeds, setFeeds]);

  useEffect(() => {
    setLoading(loadingFeedIds.size > 0);
  }, [loadingFeedIds, setLoading]);

  return {
    error,
    loading,
    loadingFeedIds,
    feeds,
    pinnedFeedIds,
    requestFeeds,
    refreshAllFeeds,
    addFeed,
    renameFeed,
    deleteFeed,
    setFeedPinned,
  };
}
