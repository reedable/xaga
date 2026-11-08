import { createContext, useContext } from "react";
import type { NavigationState } from "../types";

export const NavigationContext = createContext<{
  state: NavigationState;
  selectCollectionState: (state: NavigationState) => void;
} | null>(null);

export function useNavigationState() {
  const context = useContext(NavigationContext);
  if (!context)
    throw new Error(
      "useNavigationState must be used inside NavigationProvider"
    );
  return context;
}
