import {
  useCallback,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { subscribe } from "../hooks/useEvents";
import { usePlatform } from "../platform/Platform";
import type { AllItemsCounts } from "../types";
import { debounced } from "../utils/Decorator";
import AllItemsCountsContext from "./AllItemsCountsContext";

export function AllItemsCountsProvider({ children }: { children: ReactNode }) {
  const platform = usePlatform();
  const [counts, setCounts] = useState<AllItemsCounts | null>(null);

  const refresh = useCallback(async () => {
    try {
      setCounts(await platform.getAllItemsCounts());
    } catch {
      setCounts(null);
    }
  }, [platform]);

  useEffect(() => {
    void platform.getAllItemsCounts().then(setCounts, () => setCounts(null));
    const refreshAfterChange = debounced(refresh, { wait: 400 });
    const stopUnread = subscribe(() =>
      platform.onUnreadChanged(refreshAfterChange)
    );
    const stopEmbeddings = subscribe(() =>
      platform.onEmbeddingsChanged(refreshAfterChange)
    );
    return () => {
      stopUnread();
      stopEmbeddings();
    };
  }, [platform, refresh]);

  const value = useMemo(() => counts, [counts]);
  return (
    <AllItemsCountsContext.Provider value={value}>
      {children}
    </AllItemsCountsContext.Provider>
  );
}
