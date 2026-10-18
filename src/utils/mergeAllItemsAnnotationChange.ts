import { isEntry, type Filter, type Item, type ListItem } from "../types";

export function itemMatchesTextFilter(item: Item, filterText: string): boolean {
  if (!filterText) return true;
  const lower = filterText.toLowerCase();
  if (item.title.toLowerCase().includes(lower)) return true;
  if (isEntry(item)) {
    if (item.url.toLowerCase().includes(lower)) return true;
    if (item.description?.toLowerCase().includes(lower)) return true;
  }
  return false;
}

/** Updates a visible annotation, or inserts a qualifying new All Items annotation. */
export function mergeAllItemsAnnotationChange(
  current: Item[],
  changed: ListItem,
  isAllItemsActive: boolean,
  filter: Filter | undefined,
  filterText: string
): Item[] {
  const existing = current.findIndex((item) => item.id === changed.id);
  if (existing >= 0) {
    const next = [...current];
    next[existing] = changed;
    return next;
  }

  if (
    !isAllItemsActive ||
    (filter !== "all" && filter !== "saved") ||
    !itemMatchesTextFilter(changed, filterText)
  ) {
    return current;
  }

  const insertion = current.findIndex(
    (item) => item.createdAt < changed.createdAt
  );
  if (insertion < 0) return [...current, changed];
  return [
    ...current.slice(0, insertion),
    changed,
    ...current.slice(insertion),
  ];
}
