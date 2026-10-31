import { usePins } from "../providers/PinsContext";
import { useEffect, useMemo, useState } from "react";
import { usePlatform } from "../platform/Platform";
import type { Entry } from "../types";
import { subscribe } from "./useEvents";
import { useErrorSink } from "./useErrorSink";

/** Resolved article references from the shared pin store, independent of cache recovery. */
export function usePinnedEntries() {
  const platform = usePlatform();
  const { pins } = usePins();
  const { pushError } = useErrorSink();
  const [entries, setEntries] = useState<Entry[]>([]);
  useEffect(() => {
    const controller = new AbortController();
    let request = 0;
    const refresh = async () => {
      const version = ++request;
      try {
        const result = await platform.listPinnedEntries(controller.signal);
        if (!controller.signal.aborted && version === request)
          setEntries(result);
      } catch (error) {
        if (!controller.signal.aborted) pushError(error);
      }
    };
    const stops = [
      subscribe(() => platform.onUnreadChanged(refresh)),
      subscribe(() => platform.onBookmarkChanged(refresh)),
      subscribe(() => platform.onEmbeddingsChanged(refresh)),
      subscribe(() => platform.onFaviconChanged(refresh)),
    ];
    void refresh();
    return () => {
      controller.abort();
      stops.forEach((stop) => stop());
    };
  }, [platform, pushError, pins]);
  return useMemo(
    () =>
      entries.filter((entry) =>
        pins.some((target) => target.type === "entry" && target.id === entry.id)
      ),
    [entries, pins]
  );
}
