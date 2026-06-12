import { useCallback } from "react";
import { usePlatform } from "../platform/Platform";
import type { Entry, Feed, SavedItem } from "../types";
import { debounced } from "../utils/Decorator";

export function subscribe(register: () => Promise<() => void>): () => void {
  let unlisten: (() => void) | null = null;
  let cancelled = false;
  register().then((fn) => {
    if (cancelled) fn();
    else unlisten = fn;
  });
  return () => {
    cancelled = true;
    unlisten?.();
  };
}

export function useEvents() {
  const platform = usePlatform();

  const onFeedRefreshed = useCallback(
    (callback: (feed: Feed) => void) =>
      subscribe(() => platform.onFeedRefreshed(callback)),
    [platform]
  );

  const onUnreadChanged = useCallback(
    (callback: (entry: Entry) => void) =>
      subscribe(() => platform.onUnreadChanged(callback)),
    [platform]
  );

  const onBookmarkChanged = useCallback(
    (callback: (entry: Entry) => void) =>
      subscribe(() => platform.onBookmarkChanged(callback)),
    [platform]
  );

  const onAnnotationChanged = useCallback(
    (callback: (item: SavedItem) => void) =>
      subscribe(() => platform.onAnnotationChanged(callback)),
    [platform]
  );

  const onAnnotationRemoved = useCallback(
    (callback: (id: string) => void) =>
      subscribe(() => platform.onAnnotationRemoved(callback)),
    [platform]
  );

  const onEmbeddingsChanged = useCallback(
    (callback: () => void) =>
      subscribe(() =>
        platform.onEmbeddingsChanged(debounced(callback, { wait: 400 }))
      ),
    [platform]
  );

  const onLoaderClosed = useCallback(
    (
      callback: (data: {
        loaderUrl: string;
        pageUrl: string;
        samePage: boolean;
        html: string;
        mimeType: string;
      }) => void
    ) => subscribe(() => platform.onLoaderClosed(callback)),
    [platform]
  );

  const onLoaderPageLoaded = useCallback(
    (
      callback: (data: { url: string; html: string; mimeType: string }) => void
    ) => subscribe(() => platform.onLoaderPageLoaded(callback)),
    [platform]
  );

  const onFind = useCallback(
    (callback: () => void) => subscribe(() => platform.onFind(callback)),
    [platform]
  );

  const setFindEnabled = useCallback(
    (enabled: boolean) => platform.setFindEnabled(enabled),
    [platform]
  );

  return {
    onFeedRefreshed,
    onUnreadChanged,
    onBookmarkChanged,
    onAnnotationChanged,
    onAnnotationRemoved,
    onEmbeddingsChanged,
    onLoaderClosed,
    onLoaderPageLoaded,
    onFind,
    setFindEnabled,
  };
}
