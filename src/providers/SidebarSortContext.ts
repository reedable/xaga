import { createContext } from "react";
import type { SidebarSortMode } from "../sidebarSort";

export const SidebarSortContext = createContext<
  readonly [SidebarSortMode, (value: SidebarSortMode) => Promise<void>] | null
>(null);
