import {
  useCallback,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { usePlatform } from "../platform/Platform";
import { useErrorSink } from "../hooks/useErrorSink";
import type { SidebarSortMode } from "../sidebarSort";
import { SidebarSortContext } from "./SidebarSortContext";

export function SidebarSortProvider({ children }: { children: ReactNode }) {
  const platform = usePlatform();
  const { pushError } = useErrorSink();
  const [sortMode, setSortMode] = useState<SidebarSortMode>("unread");
  useEffect(() => {
    let active = true;
    let revision = 0;
    const subscription = platform.onSidebarSortChanged((value) => {
      if (active) {
        revision++;
        setSortMode(value);
      }
    });
    // Subscribe before reading; a late startup read must not overwrite a change.
    void subscription
      .then(async () => {
        const requested = revision;
        const value = await platform.getSidebarSort();
        if (active && requested === revision) setSortMode(value);
      })
      .catch((error) => {
        if (active) pushError(error);
      });
    return () => {
      active = false;
      void subscription.then((stop) => stop()).catch(() => {});
    };
  }, [platform, pushError]);
  const saveSort = useCallback(
    async (value: SidebarSortMode) => {
      try {
        await platform.setSidebarSort(value);
      } catch (error) {
        pushError(error);
      }
    },
    [platform, pushError]
  );
  const value = useMemo(
    () => [sortMode, saveSort] as const,
    [sortMode, saveSort]
  );
  return (
    <SidebarSortContext.Provider value={value}>
      {children}
    </SidebarSortContext.Provider>
  );
}
