import { useCallback } from "react";
import { usePlatform } from "../platform/Platform";
import type { Feed, ListUpdate } from "../types";

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

  const onListUpdated = useCallback(
    (callback: (update: ListUpdate) => void) => {
      const unlistenPromise = platform.onListUpdated(callback);
      return () => {
        unlistenPromise.then((unlisten) => unlisten());
      };
    },
    [platform],
  );

  return {
    onFeedRefreshed,
    onListUpdated,
  };
}
