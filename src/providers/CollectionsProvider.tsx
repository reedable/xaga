import { useCallback, useEffect, useMemo, useState, type ReactNode } from "react";
import { useLocalStorage } from "../hooks/useLocalStorage";
import { subscribe } from "../hooks/useEvents";
import { usePlatform } from "../platform/Platform";
import { debounced } from "../utils/Decorator";
import type { Collection } from "../types";
import CollectionsContext, { type CollectionsContextType } from "./CollectionsContext";

export function CollectionsProvider({ children }: { children: ReactNode }) {
  const platform = usePlatform();
  const [collections, setCollections] = useState<Collection[]>([]);
  const [pinnedCollectionIds, setPinnedCollectionIds] = useLocalStorage<string[]>(
    "sidebar.pinnedCollections",
    []
  );

  const refreshCollections = useCallback(async () => {
    const results = await platform.listCollections(new AbortController().signal);
    if (results) setCollections(results);
  }, [platform]);

  useEffect(() => {
    const refresh = debounced(refreshCollections, { wait: 1000 });
    return subscribe(() => platform.onUnreadChanged(refresh));
  }, [platform, refreshCollections]);

  const value = useMemo<CollectionsContextType>(
    () => ({ collections, pinnedCollectionIds, setCollections, setPinnedCollectionIds }),
    [collections, pinnedCollectionIds, setPinnedCollectionIds]
  );

  return (
    <CollectionsContext.Provider value={value}>
      {children}
    </CollectionsContext.Provider>
  );
}
