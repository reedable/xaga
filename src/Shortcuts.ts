import type { Shortcut } from "./types";

export const ALL_ITEMS: Shortcut = {
  id: "__ALL__",
  title: "All items",
  createdAt: new Date(0).toISOString(),
  entryCount: 0,
  unreadCount: 0,
  filter: "unread",
};

export const SHORTCUTS: Shortcut[] = [ALL_ITEMS];
