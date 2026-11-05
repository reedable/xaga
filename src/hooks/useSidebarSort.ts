import { useCallback, useEffect, useRef, useState } from "react";
import { getViewState, setViewState } from "../storage/viewState";

export const SIDEBAR_SORT_MODES = ["unread", "alphabetical"] as const;

export type SidebarSortMode = (typeof SIDEBAR_SORT_MODES)[number];
export type SidebarSection = "shortcuts" | "topics" | "feeds" | "sites";

const DEFAULT_SORT_MODE: SidebarSortMode = "unread";
const changes = new EventTarget();

function isSidebarSortMode(value: unknown): value is SidebarSortMode {
  return SIDEBAR_SORT_MODES.some((mode) => mode === value);
}

export function useSidebarSort(section: SidebarSection) {
  const [sortMode, setSortModeState] =
    useState<SidebarSortMode>(DEFAULT_SORT_MODE);
  const changedRef = useRef(false);

  useEffect(() => {
    let cancelled = false;
    changedRef.current = false;
    const onChange = (event: Event) => {
      const change = (
        event as CustomEvent<{
          section: SidebarSection;
          value: SidebarSortMode;
        }>
      ).detail;
      if (change.section === section) {
        changedRef.current = true;
        setSortModeState(change.value);
      }
    };
    changes.addEventListener("change", onChange);

    getViewState<unknown>(`sidebar.sort.${section}`)
      .then((value) => {
        if (!cancelled && !changedRef.current && isSidebarSortMode(value)) {
          setSortModeState(value);
        }
      })
      .catch(console.error);

    return () => {
      cancelled = true;
      changes.removeEventListener("change", onChange);
    };
  }, [section]);

  const setSortMode = useCallback(
    (value: SidebarSortMode) => {
      changedRef.current = true;
      setSortModeState(value);
      changes.dispatchEvent(
        new CustomEvent("change", { detail: { section, value } })
      );
      setViewState(`sidebar.sort.${section}`, value).catch(console.error);
    },
    [section]
  );

  return [sortMode, setSortMode] as const;
}
