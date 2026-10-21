import type { SidebarSortMode } from "../hooks/useSidebarSort";
import sortByTitle from "./sortByTitle";
import sortByUnreadCount from "./sortByUnreadCount";

type SidebarCollection = { title: string; unreadCount: number };

export default function sortSidebarCollections<T extends SidebarCollection>(
  collections: T[],
  sortMode: SidebarSortMode
): T[] {
  return [...collections].sort(
    sortMode === "alphabetical" ? sortByTitle : sortByUnreadCount
  );
}
