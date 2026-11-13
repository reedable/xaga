import { createContext, useContext, type RefObject } from "react";

export const ReadingPaneContext = createContext<{
  activate: () => void;
  returnFocus: RefObject<(() => void) | null>;
} | null>(null);

export function useReadingPane() {
  return useContext(ReadingPaneContext);
}
