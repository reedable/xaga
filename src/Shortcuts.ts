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

export const SAVED_ITEMS: Shortcut = {
  id: "__ALL__",
  title: "Saved",
  createdAt: new Date(0).toISOString(),
  filter: "saved",
};

export const BOOKMARKED_ITEMS: Shortcut = {
  id: "__ALL__",
  title: "Bookmarked",
  createdAt: new Date(0).toISOString(),
  filter: "bookmarked",
};

export const ANNOTATION_ITEMS: Shortcut = {
  id: "__ANNOTATION_ITEMS__",
  title: "From Annotations",
  createdAt: new Date(0).toISOString(),
  filter: "unread",
};

export const RECOMMENDED_ITEMS: Shortcut = {
  id: "__RECOMMENDED_ITEMS__",
  title: "Recommended",
  createdAt: new Date(0).toISOString(),
  filter: "unread",
};
