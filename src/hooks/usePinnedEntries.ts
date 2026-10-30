import { useEffect, useState } from "react";
import { usePlatform } from "../platform/Platform";
import type { Entry } from "../types";
import { subscribe } from "./useEvents";
import { useErrorSink } from "./useErrorSink";

/** A projection of persisted article pins; mutation events precede cache sync. */
export function usePinnedEntries() {
  const platform = usePlatform();
  const { pushError } = useErrorSink();
  const [entries, setEntries] = useState<Entry[]>([]);
  useEffect(() => {
    const controller = new AbortController();
    const pending = new Map<string, Entry>();
    let request = 0;
    const refresh = async () => {
      const version = ++request;
      try {
        const result = await platform.listPinnedEntries(controller.signal);
        if (controller.signal.aborted || version !== request) return;
        const merged = new Map(result.map((entry) => [entry.id, entry]));
        // File-committed intent wins over an asynchronously updated cache.
        for (const entry of pending.values()) {
          if (entry.isPinned)
            merged.set(entry.id, {
              ...(merged.get(entry.id) ?? entry),
              isPinned: true,
            });
          else merged.delete(entry.id);
        }
        setEntries([...merged.values()]);
      } catch (error) {
        if (!controller.signal.aborted) pushError(error);
      }
    };
    const changed = (entry: Entry) => {
      pending.set(entry.id, entry);
      setEntries((current) => [
        ...current.filter((value) => value.id !== entry.id),
        ...(entry.isPinned ? [entry] : []),
      ]);
    };
    const stateChanged = (entry: Entry) => {
      const pin = pending.get(entry.id);
      const updated = pin ? { ...entry, isPinned: pin.isPinned } : entry;
      if (pin) pending.set(entry.id, updated);
      setEntries((current) =>
        current.map((value) => (value.id === entry.id ? updated : value))
      );
    };
    const unsubscribers = [
      subscribe(() => platform.onEntryPinChanged(changed)),
      subscribe(() => platform.onUnreadChanged(stateChanged)),
      subscribe(() => platform.onBookmarkChanged(stateChanged)),
      subscribe(() => platform.onEmbeddingsChanged(refresh)),
    ];
    void refresh();
    return () => {
      controller.abort();
      unsubscribers.forEach((unsubscribe) => unsubscribe());
    };
  }, [platform, pushError]);
  return entries;
}
