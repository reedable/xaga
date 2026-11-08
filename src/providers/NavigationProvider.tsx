import { useCallback, useMemo, useState, type ReactNode } from "react";
import { useLocation, type Location } from "react-router-dom";
import { ALL_ENTITIES } from "../Shortcuts";
import type { NavigationState } from "../types";
import { NavigationContext } from "./NavigationContext";

export function NavigationProvider({ children }: { children: ReactNode }) {
  const location = useLocation();
  const [selection, setSelection] = useState<{
    location: Location;
    state: NavigationState;
  } | null>(null);

  // Collection browsing belongs to this visit, never to the saved history entry.
  // Every actual navigation restores its own snapshot and discards this override.
  if (selection && selection.location !== location) setSelection(null);

  const state = useMemo<NavigationState>(
    () =>
      selection?.location === location
        ? selection.state
        : ((location.state as NavigationState | null) ?? {
            shortcutId: ALL_ENTITIES.id,
          }),
    [selection, location]
  );
  const selectCollectionState = useCallback(
    (state: NavigationState) => {
      setSelection({ location, state });
    },
    [location]
  );
  const value = useMemo(
    () => ({ state, selectCollectionState }),
    [state, selectCollectionState]
  );
  return (
    <NavigationContext.Provider value={value}>
      {children}
    </NavigationContext.Provider>
  );
}
