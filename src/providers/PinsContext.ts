import { createContext, useContext } from "react";
import type { PinTarget, PinNode, PinEdit } from "../pins";
export const PinsContext = createContext<{
  pins: PinTarget[];
  tree: PinNode[];
  organizePins: (edit: PinEdit) => Promise<void>;
  refreshPins?: () => Promise<void>;
  setPin: (target: PinTarget, pinned: boolean) => Promise<void>;
} | null>(null);
export function usePins() {
  const context = useContext(PinsContext);
  if (!context) throw new Error("usePins requires PinsProvider");
  return context;
}
