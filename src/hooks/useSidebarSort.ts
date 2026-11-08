import { useContext } from "react";
import { SidebarSortContext } from "../providers/SidebarSortContext";
export { SIDEBAR_SORT_MODES, type SidebarSortMode } from "../sidebarSort";

export function useSidebarSort() {
  const context = useContext(SidebarSortContext);
  if (!context)
    throw new Error("useSidebarSort must be used inside SidebarSortProvider");
  return context;
}
