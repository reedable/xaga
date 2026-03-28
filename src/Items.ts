import type { Item } from "./types";

export const ALL_ITEMS = {
  id: "__ALL__",
  title: "All items",
  createdAt: new Date(0).toISOString(),
};

export const RECOMMENDED_ITEMS = {
  id: "__RECOMMENDED_ITEMS__",
  title: "Recommended",
  createdAt: new Date(0).toISOString(),
} as Item;
