# ItemList does not reload after topic membership change

## Summary

Adding or removing an item through `TopicMenuButton` updates the Topic but does not update an open `ItemList` for that Topic. The visible membership remains stale until another reload trigger occurs.

## Requirements

- Reflect successful membership changes immediately in the affected Topic list.
- Respect the active filter, sort, and text filter when inserting or removing an item.
- Do not update the list when the backend rejects the change.
- Keep Topic counts and membership presentation consistent with the resulting list.

## Remaining work

- Decide whether membership mutations return enough data for local reduction or emit a domain event consumed by all views.
- Update the active Topic list and its counts after successful add and remove operations.
- Preserve selection when the selected item remains visible and define behavior when it is removed from the active Topic.
- Test entry and annotation membership across relevance and timestamp sorts.

## References

- `src/components/list/ItemList.tsx`
- `src/components/content/TopicMenuButton.tsx`
- `src/hooks/useTopics.tsx`
- `src-tauri/src/commands/topics.rs`
