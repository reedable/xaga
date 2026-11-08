export const SIDEBAR_SORT_MODES = ["unread", "alphabetical"] as const;
export type SidebarSortMode = (typeof SIDEBAR_SORT_MODES)[number];
