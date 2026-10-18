import type { Item, ListItem } from "../types";

/** Inserts one newly added Topic item without removing stale visible rows. */
export function mergeTopicAddition(
  current: Item[],
  authoritative: ListItem[],
  addedId: string
): Item[] {
  if (current.some((item) => item.id === addedId)) return current;
  const addedIndex = authoritative.findIndex((item) => item.id === addedId);
  if (addedIndex < 0) return current;

  const added = authoritative[addedIndex];
  for (let i = addedIndex + 1; i < authoritative.length; i++) {
    const next = current.findIndex((item) => item.id === authoritative[i].id);
    if (next >= 0)
      return [...current.slice(0, next), added, ...current.slice(next)];
  }
  for (let i = addedIndex - 1; i >= 0; i--) {
    const previous = current.findIndex(
      (item) => item.id === authoritative[i].id
    );
    if (previous >= 0)
      return [
        ...current.slice(0, previous + 1),
        added,
        ...current.slice(previous + 1),
      ];
  }
  return [...current, added];
}
