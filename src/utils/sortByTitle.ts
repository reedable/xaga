import type { Item } from "../types";

export default function sortByTitle(a: Item, b: Item) {
  return a.title.localeCompare(b.title);
}
