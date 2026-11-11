import { createContext } from "react";

export const SidebarListLayoutContext = createContext<{
  compact: boolean;
  actions: HTMLElement | null;
} | null>(null);
