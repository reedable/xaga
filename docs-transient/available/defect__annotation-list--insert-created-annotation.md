# Insert newly created annotations into All Items

## Summary

`ItemList` handles `annotation:changed` by replacing a row with the same ID, so a
newly created annotation is not inserted into the active All Items list.

## Requirements

- Insert a newly created annotation into All Items when it matches the active
  item filter and text filter.
- Place the annotation according to the active sort without duplicating an
  existing row.
- Continue updating an existing annotation with the same event.
- Do not insert the annotation into feed or Topic lists through the All Items
  event path.
- Do not require a full list reload.

## Remaining work

- Extract or reuse the active-list membership, filter, and sort checks for an
  `annotation:changed` item absent from the current rows.
- Insert qualifying annotations only when All Items is active.
- Test matching and nonmatching filters, sort position, duplicate delivery, and
  feed and Topic isolation.

## References

- `src/components/list/ItemList.tsx`
- `src-tauri/src/commands/articles.rs`
- `crates/xaga-core/src/events.rs`

