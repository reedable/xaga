import {
  useCallback,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { subscribe } from "../hooks/useEvents";
import { usePlatform } from "../platform/Platform";
import { emptySite } from "../sites";
import type { Site } from "../types";
import { debounced } from "../utils/Decorator";
import sortByUnreadCount from "../utils/sortByUnreadCount";
import { usePins } from "./PinsContext";
import SitesContext from "./SitesContext";

export function SitesProvider({
  children,
  enabled = true,
}: {
  children: ReactNode;
  enabled?: boolean;
}) {
  const platform = usePlatform();
  const { pins } = usePins();
  const [rawSites, setSites] = useState<Site[]>();
  const sites = useMemo(() => rawSites ?? [], [rawSites]);
  const hasLoaded = !enabled || rawSites !== undefined;

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

  const pinnedSites = useMemo(
    () =>
      pins.flatMap((pin) => {
        if (pin.type !== "site" || !hasLoaded) return [];
        return [
          sites.find((site) => site.hostname === pin.hostname) ??
            emptySite(pin.hostname),
        ];
      }),
    [pins, sites, hasLoaded]
  );
  const value = useMemo(
    () => ({ sites, pinnedSites, hasLoaded, refreshSites }),
    [sites, pinnedSites, hasLoaded, refreshSites]
  );
  return (
    <SitesContext.Provider value={value}>{children}</SitesContext.Provider>
  );
}
