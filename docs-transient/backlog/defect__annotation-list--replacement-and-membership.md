# Keep annotation replacement and collection membership synchronized

## Summary

Recoloring replaces an annotation under a new ID. The list replaces a matching
old row, but selection remains anchored to the removed ID, and annotation events
do not consistently re-evaluate collection membership, filters, or sort position.

## Requirements

- Treat recolor replacement as one visible transition and move selection from the old ID to the new ID when applicable.
- Re-evaluate changed and replacement annotations against the active feed or Topic
  membership, item filter, text filter, and sort.
- Preserve event ordering when removal and replacement events arrive close together.
- Avoid requiring a full list reload for ordinary annotation changes.

## Remaining work

- Verify the existing replacement payload and event ordering across recolor and
  removal paths.
- Centralize list-event reduction for annotation edit, recolor, and delete.
- Re-evaluate membership, filter, and sort position before retaining or replacing
  an event item.
- Update routing when the selected annotation is replaced or removed.
- Test Topic, feed, filtered, sorted, and selected-item replacement cases.

## References

- `src/components/list/ItemList.tsx`
- `src/hooks/useRouter.tsx`
- `src-tauri/src/commands/articles.rs`
- `crates/xaga-core/src/events.rs`
- `docs/architecture/crash_consistency.md`
