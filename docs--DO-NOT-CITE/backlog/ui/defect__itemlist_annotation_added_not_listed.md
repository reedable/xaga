# Keep annotation list state synchronized

## Summary

`ItemList` applies `annotation:changed` only by replacing an item with the same ID. Newly created annotations are not inserted. Recoloring replaces an annotation under a new ID, so the removal is applied but the replacement is not inserted and selection remains anchored to the removed ID.

## Requirements

- Insert, update, or remove annotation rows according to the active collection, filter, text filter, and sort.
- Treat recolor replacement as one visible transition and move selection from the old ID to the new ID when applicable.
- Do not insert an annotation into a feed or Topic list unless it belongs there.
- Preserve event ordering when removal and replacement events arrive close together.
- Avoid requiring a full list reload for ordinary annotation changes.

## Remaining work

- Define the event payload or correlation needed to identify an old-to-new annotation replacement.
- Centralize list-event reduction for annotation create, edit, recolor, and delete.
- Re-evaluate membership, filter, and sort position before inserting an event item.
- Update routing when the selected annotation is replaced or removed.
- Test All Items, Topic, feed, filtered, sorted, and selected-item cases.

## References

- `src/components/list/ItemList.tsx`
- `src/hooks/useRouter.tsx`
- `src-tauri/src/commands/articles.rs`
- `crates/xaga-core/src/events.rs`
- `docs/architecture/crash_consistency.md`
