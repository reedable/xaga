import {
  useCallback,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { subscribe } from "../hooks/useEvents";
import { usePlatform } from "../platform/Platform";
import type { AllEntitiesCounts } from "../types";
import { debounced } from "../utils/Decorator";
import AllEntitiesCountsContext from "./AllEntitiesCountsContext";

export function AllEntitiesCountsProvider({ children }: { children: ReactNode }) {
  const platform = usePlatform();
  const [counts, setCounts] = useState<AllEntitiesCounts | null>(null);

  const refresh = useCallback(async () => {
    try {
      setCounts(await platform.getAllEntitiesCounts());
    } catch {
      setCounts(null);
    }
  }, [platform]);

  useEffect(() => {
    void platform.getAllEntitiesCounts().then(setCounts, () => setCounts(null));
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
    <AllEntitiesCountsContext.Provider value={value}>
      {children}
    </AllEntitiesCountsContext.Provider>
  );
}
