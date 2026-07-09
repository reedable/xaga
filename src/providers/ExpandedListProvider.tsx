import { useCallback, useEffect, useMemo, useState, type ReactNode } from "react";
import { getViewState, setViewState } from "../storage/viewState";
import ExpandedListContext, {
  type ListType,
} from "./ExpandedListContext";

// View state, persisted client-side (IndexedDB) — not a backend user setting.
const STORAGE_KEY = "sidebar.expandedList";
const DEFAULT_EXPANDED: ListType = "SHORTCUTS";

/**
 * Single shared source for which sidebar section is expanded. Reads the persisted
 * value once from IndexedDB on mount and holds it, so the many `useSidebar` consumers
 * share one value (and see each other's changes live) instead of each fetching it.
 */
export function ExpandedListProvider({ children }: { children: ReactNode }) {
  const [expandedList, setExpandedListState] = useState<ListType | null>(
    DEFAULT_EXPANDED
  );

  useEffect(() => {
    let cancelled = false;
    getViewState<ListType | null>(STORAGE_KEY)
      .then((value) => {
        if (!cancelled) setExpandedListState(value ?? DEFAULT_EXPANDED);
      })
      .catch(console.error);
    return () => {
      cancelled = true;
    };
  }, []);

  const setExpandedList = useCallback(
    (
      updater:
        | ((prev: ListType | null) => ListType | null)
        | (ListType | null)
    ) => {
      setExpandedListState((prev) => {
        const next = typeof updater === "function" ? updater(prev) : updater;
        setViewState(STORAGE_KEY, next).catch(console.error);
        return next;
      });
    },
    []
  );

  const value = useMemo(
    () => ({ expandedList, setExpandedList }),
    [expandedList, setExpandedList]
  );

  return (
    <ExpandedListContext.Provider value={value}>
      {children}
    </ExpandedListContext.Provider>
  );
}
