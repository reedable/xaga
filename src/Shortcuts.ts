import type { Shortcut } from "./types";

export const ALL_ITEMS: Shortcut = {
  id: "__ALL__",
  title: "All items",
  createdAt: new Date(0).toISOString(),
  filter: "unread",
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
