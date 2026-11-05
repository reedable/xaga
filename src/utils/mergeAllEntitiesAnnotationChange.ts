import { isEntry, type Filter, type ManagedEntity, type ListEntity } from "../types";

export function entityMatchesTextFilter(entity: ManagedEntity, filterText: string): boolean {
  if (!filterText) return true;
  const lower = filterText.toLowerCase();
  if (entity.title.toLowerCase().includes(lower)) return true;
  if (isEntry(entity)) {
    if (entity.url.toLowerCase().includes(lower)) return true;
    if (entity.description?.toLowerCase().includes(lower)) return true;
  }
  return false;
}

/** Updates a visible annotation, or inserts a qualifying new All Entities annotation. */
export function mergeAllEntitiesAnnotationChange(
  current: ManagedEntity[],
  changed: ListEntity,
  isAllEntitiesActive: boolean,
  filter: Filter | undefined,
  filterText: string
): ManagedEntity[] {
  const existing = current.findIndex((entity) => entity.id === changed.id);
  if (existing >= 0) {
    const next = [...current];
    next[existing] = changed;
    return next;
  }

  if (
    !isAllEntitiesActive ||
    (filter !== "all" && filter !== "saved") ||
    !entityMatchesTextFilter(changed, filterText)
  ) {
    return current;
  }

  const insertion = current.findIndex(
    (entity) => entity.createdAt < changed.createdAt
  );
  if (insertion < 0) return [...current, changed];
  return [
    ...current.slice(0, insertion),
    changed,
    ...current.slice(insertion),
  ];
}
