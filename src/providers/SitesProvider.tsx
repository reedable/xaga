import {
  useCallback,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { subscribe } from "../hooks/useEvents";
import { usePlatform } from "../platform/Platform";
import type { Site } from "../types";
import { debounced } from "../utils/Decorator";
import sortByUnreadCount from "../utils/sortByUnreadCount";
import SitesContext from "./SitesContext";

export function SitesProvider({
  children,
  enabled = true,
}: {
  children: ReactNode;
  enabled?: boolean;
}) {
  const platform = usePlatform();
  const [sites, setSites] = useState<Site[]>([]);

  const refreshSites = useCallback(async () => {
    if (!enabled) return;
    try {
      const result = await platform.listSites(new AbortController().signal);
      setSites([...result].sort(sortByUnreadCount));
    } catch {
      setSites([]);
    }
  }, [enabled, platform]);

  useEffect(() => {
    if (!enabled) return;
    void platform.listSites(new AbortController().signal).then(
      (result) => setSites([...result].sort(sortByUnreadCount)),
      () => setSites([])
    );
    const refresh = debounced(refreshSites, { wait: 400 });
    const stopEmbeddings = subscribe(() =>
      platform.onEmbeddingsChanged(refresh)
    );
    const stopUnread = subscribe(() => platform.onUnreadChanged(refresh));
    return () => {
      stopEmbeddings();
      stopUnread();
    };
  }, [enabled, platform, refreshSites]);

  const value = useMemo(() => ({ sites, refreshSites }), [sites, refreshSites]);
  return (
    <SitesContext.Provider value={value}>{children}</SitesContext.Provider>
  );
}
