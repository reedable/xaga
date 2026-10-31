import { createContext, useContext } from "react";
import type { PinTarget } from "../pins";
export const PinsContext = createContext<{
  pins: PinTarget[];
  refreshPins?: () => Promise<void>;
  setPin: (target: PinTarget, pinned: boolean) => Promise<void>;
} | null>(null);
export function usePins() {
  const context = useContext(PinsContext);
  if (!context) throw new Error("usePins requires PinsProvider");
  return context;
}
