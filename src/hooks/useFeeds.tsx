import { useCallback, useContext, useEffect, type RefObject } from "react";
import { usePlatform } from "../platform/Platform";
import FeedsContext from "../providers/FeedsContext";
import { isAbortError } from "../types";
import { useErrorSink } from "./useErrorSink";
import {
  feedRefreshError,
  feedRefreshRequestError,
} from "../utils/feedRefreshError";
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
  const { pushError } = useErrorSink();
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
        if (signal.aborted) return;
        setFeeds((current) =>
          current.map(
            (feed) =>
              refreshed.feeds.find((updated) => updated.id === feed.id) ?? feed
          )
        );
        refreshed.feeds.forEach((feed) => cb(feed));
        if (refreshed.failures.length) {
          const failure = feedRefreshError(refreshed.failures);
          setError(failure);
          pushError(failure);
        }
      } catch (e) {
        if (!signal.aborted && !isAbortError(e)) {
          const failure = feedRefreshRequestError(e);
          setError(failure);
          pushError(failure);
        }
      } finally {
        setLoadingSet(new Set());
      }
    },
    [abortControllerRef, platform, setError, setLoadingSet, setFeeds, pushError]
  );

  const addFeed = useCallback(
    async (feedUrl: string) => {
      const signal = abortControllerRef.current.signal;
      setError(null);
      try {
        return await platform.addFeed(signal, feedUrl);
      } catch (e) {
        setError(e);
        throw e;
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
        throw e;
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
        throw e;
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
