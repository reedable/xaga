import { useCallback, useContext, useEffect, type RefObject } from "react";
import { usePlatform } from "../platform/Platform";
import FeedsContext from "../providers/FeedsContext";
import type { Feed } from "../types";
import { debounced, decorate } from "../utils/Decorator";

export function useFeeds(abortControllerRef: RefObject<AbortController>) {
  const context = useContext(FeedsContext);
  const platform = usePlatform();

  if (!context) {
    throw new Error("useFeeds must be used inside FeedsProvider");
  }

  const { error, loading, loadingSet, setLoading, setLoadingSet, setError } =
    context;

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
          const newSet = new Set(prev);
          newSet.delete(feedId);
          return newSet;
        });
      }
    },
    [abortControllerRef, platform, setError, setLoadingSet],
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
    [abortControllerRef, platform, setError],
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
    [abortControllerRef, platform, setError],
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
    [abortControllerRef, platform, setError],
  );

  const importFeeds = useCallback(
    async () => {
      const signal = abortControllerRef.current.signal;
      setError(null);

      try {
        return await platform.importFeeds(signal);
      } catch (e) {
        setError(e);
      }
    },
    [abortControllerRef, platform, setError],
  );

  const exportFeeds = useCallback(
    async () => {
      const signal = abortControllerRef.current.signal;
      setError(null);

      try {
        await platform.exportFeeds(signal);
      } catch (e) {
        setError(e);
      }
    },
    [abortControllerRef, platform, setError],
  );

  const DEFAULT_CALLBACK = () => {};

  const refreshAllFeeds = useCallback(
    async (callback: (feed: Feed) => void = DEFAULT_CALLBACK) => {
      const signal = abortControllerRef.current.signal;

      const debounced_callback = decorate(callback).with(debounced, {
        wait: 400,
      });

      try {
        const feeds = await platform.listFeeds(signal);
        if (!feeds) return;

        const promises = feeds.map((feed) =>
          refreshFeed(feed.id).then((feedId?: string) => {
            if (feedId) debounced_callback(feed);
          }),
        );
        await Promise.all(promises);
      } catch (e) {
        setError(e);
      }
    },
    [abortControllerRef, platform, refreshFeed, setError],
  );

  useEffect(() => {
    setLoading(loadingSet.size > 0);
  }, [loadingSet, setLoading]);

  return {
    error,
    loading,
    loadingSet,
    addFeed,
    importFeeds,
    exportFeeds,
    renameFeed,
    deleteFeed,
    refreshFeed,
    refreshAllFeeds,
  };
}
