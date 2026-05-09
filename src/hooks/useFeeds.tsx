import { useCallback, useContext, useEffect, type RefObject } from "react";
import { usePlatform } from "../platform/Platform";
import FeedsContext from "../providers/FeedsContext";
import type { Feed } from "../types";
import { debounced, decorate } from "../utils/Decorator";
import sortByUnreadCount from "../utils/sortByUnreadCount";
import { useCollections } from "./useCollections";

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
    setPinnedFeedIds,
  } = context;

  const platform = usePlatform();
  const { listFeeds } = useCollections(abortControllerRef);

  const refreshFeed = useCallback(
    async (feedId: string) => {
      const signal = abortControllerRef.current.signal;
      setError(null);
      setLoadingSet((prev) => new Set(prev).add(feedId));
      try {
        await platform.refreshFeed(signal, feedId);
        return feedId;
      } catch (e) {
        setError(e);
      } finally {
        setLoadingSet((prev) => {
          const next = new Set(prev);
          next.delete(feedId);
          return next;
        });
      }
    },
    [abortControllerRef, platform, setError, setLoadingSet]
  );

  const refreshAllFeeds = useCallback(
    async (callback: (feed: Feed) => void = () => {}) => {
      const signal = abortControllerRef.current.signal;
      const cb = decorate(callback).with(debounced, { wait: 400 });
      try {
        const list = await platform.listFeeds(signal);
        if (!list) return;
        await Promise.all(
          list.map((feed) =>
            refreshFeed(feed.id).then((id) => {
              if (id) cb(feed);
            })
          )
        );
      } catch (e) {
        setError(e);
      }
    },
    [abortControllerRef, platform, refreshFeed, setError]
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
    refreshFeed,
    refreshAllFeeds,
    addFeed,
    renameFeed,
    deleteFeed,
    setPinnedFeedIds,
  };
}
