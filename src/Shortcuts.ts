import type { Shortcut } from "./types";

export const ALL_ITEMS: Shortcut = {
  id: "__ALL__",
  title: "All items",
  createdAt: new Date(0).toISOString(),
  filter: "all",
};

export const UNREAD_ITEMS: Shortcut = {
  id: "__ALL__",
  title: "Unread",
  createdAt: new Date(0).toISOString(),
  filter: "unread",
};

export const BOOKMARKED_ITEMS: Shortcut = {
  id: "__ALL__",
  title: "Bookmarked",
  createdAt: new Date(0).toISOString(),
  filter: "bookmarked",
};

export const RECOMMENDED_ITEMS: Shortcut = {
  id: "__RECOMMENDED_ITEMS__",
  title: "Recommended",
  createdAt: new Date(0).toISOString(),
  filter: "unread",
};
