import { usePins } from "./PinsContext";
import {
  useCallback,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { useBookmarkImportEvents } from "../hooks/useBookmarkImportEvents";
import { importFailure } from "../utils/importFailure";
import { useErrorSink } from "../hooks/useErrorSink";
import { subscribe } from "../hooks/useEvents";
import { usePlatform } from "../platform/Platform";
import type { ExportOutcome, Feed, ImportOutcome } from "../types";
import { debounced } from "../utils/Decorator";
import sortByUnreadCount from "../utils/sortByUnreadCount";
import FeedsContext, { type FeedsContextType } from "./FeedsContext";

export default function FeedsProvider({ children }: { children: ReactNode }) {
  const platform = usePlatform();
  useBookmarkImportEvents();
  const { pushError } = useErrorSink();
  const [error, setError] = useState<unknown>(null);
  const [loading, setLoading] = useState(false);
  const [loadingSet, setLoadingSet] = useState<Set<string>>(new Set());
  const [rawFeeds, setFeeds] = useState<Feed[]>([]);
  const { pins, setPin } = usePins();
  const feeds = useMemo(
    () =>
      rawFeeds.map((value) => ({
        ...value,
        isPinned: pins.some(
          (pin) => pin.type === "feed" && pin.id === value.id
        ),
      })),
    [rawFeeds, pins]
  );

  const pinnedFeedIds = useMemo(
    () => pins.filter((pin) => pin.type === "feed").map((pin) => pin.id),
    [pins]
  );
  const setFeedPinned = useCallback(
    (id: string, pinned: boolean) => {
      void setPin({ type: "feed", id }, pinned).catch(pushError);
    },
    [setPin, pushError]
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

  // Import/export are orchestrated natively (no webview command — see
  // docs/architecture/security/command_surface.md); the outcome arrives by event. On
  // success we re-list feeds; on failure we surface the error, preserving the
  // pre-refactor UX where the JS layer drove the call.
  const handleFeedsImported = useCallback(
    async (outcome: ImportOutcome<Feed>) => {
      setError(null);
      if (outcome.error !== null) {
        setError(outcome.error);
        pushError(outcome.error);
        return;
      }
      const failure = importFailure("feeds", outcome);
      if (failure) {
        setError(failure);
        pushError(failure);
      }
      const signal = new AbortController().signal;
      try {
        const results = await platform.listFeeds(signal);
        if (results) setFeeds([...results].sort(sortByUnreadCount));
      } catch (e) {
        setError(e);
        pushError(e);
      }
    },
    [platform, setFeeds, pushError]
  );

  const handleFeedsExported = useCallback(
    (outcome: ExportOutcome) => {
      setError(outcome.error);
      if (outcome.error !== null) pushError(outcome.error);
    },
    [pushError]
  );

  useEffect(
    () => subscribe(() => platform.onFeedsImported(handleFeedsImported)),
    [platform, handleFeedsImported]
  );

  useEffect(
    () => subscribe(() => platform.onFeedsExported(handleFeedsExported)),
    [platform, handleFeedsExported]
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
