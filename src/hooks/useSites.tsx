import { useCallback, useContext, useState, type RefObject } from "react";
import { usePlatform } from "../platform/Platform";
import SitesContext from "../providers/SitesContext";
import type { Filter } from "../types";

export function useSites(abortControllerRef: RefObject<AbortController>) {
  const context = useContext(SitesContext);
  const platform = usePlatform();
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<unknown>(null);

  if (!context) throw new Error("useSites must be used inside SitesProvider");

  const listSiteEntries = useCallback(
    async (hostname: string, filter: Filter = "unread") => {
      setLoading(true);
      setError(null);
      try {
        return await platform.listSiteEntries(
          abortControllerRef.current.signal,
          hostname,
          filter
        );
      } catch (reason) {
        setError(reason);
      } finally {
        setLoading(false);
      }
    },
    [abortControllerRef, platform]
  );

  return {
    ...context,
    pinnedSites: context.pinnedSites ?? [],
    listSiteEntries,
    loading,
    error,
  };
}
