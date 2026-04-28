import { useCallback } from "react";
import { usePlatform } from "../platform/Platform";
import type { Entry, Feed } from "../types";

export function useEvents() {
  const platform = usePlatform();

  const onFeedRefreshed = useCallback(
    (callback: (feed: Feed) => void) => {
      const unlistenPromise = platform.onFeedRefreshed(callback);
      return () => {
        unlistenPromise.then((unlisten) => unlisten());
      };
    },
    [platform],
  );

  const onUnreadChanged = useCallback(
    (callback: (entry: Entry) => void) => {
      const unlistenPromise = platform.onUnreadChanged(callback);
      return () => {
        unlistenPromise.then((unlisten) => unlisten());
      };
    },
    [platform],
  );

  const onBookmarkChanged = useCallback(
    (callback: (entry: Entry) => void) => {
      const unlistenPromise = platform.onBookmarkChanged(callback);
      return () => {
        unlistenPromise.then((unlisten) => unlisten());
      };
    },
    [platform],
  );

  const onEmbeddingsChanged = useCallback(
    (callback: () => void) => {
      const unlistenPromise = platform.onEmbeddingsChanged(callback);
      return () => {
        unlistenPromise.then((unlisten) => unlisten());
      };
    },
    [platform],
  );

  return {
    onFeedRefreshed,
    onUnreadChanged,
    onBookmarkChanged,
    onEmbeddingsChanged,
  };
}
