# Browse Entries by Tags

## Summary

Provide tag navigation that scopes the existing ListView to matching Entries,
with normal article reading and stable browsing context.

## Requirements

- Use the [Tag storage](../../docs/architecture/tags.md)
  contract and existing 0.14 format.
- Resolve selected tag names through library definitions and query indexed tag
  IDs. Display current names rather than internal IDs.
- Show directly tagged Entries only; no implicit tagging through collections
  and no mixed Feed/Topic/Site results.
- Selecting an Entry opens its article through existing navigation.
- Provide a Tags sidebar section with a multi-select list. Match the intersection
  of selected tags: an Entry must carry every selected tag, never merely any one.
- Preserve access to tag selection when the sidebar is hidden.
- With no tags selected, show all Entries subject to Unread/Saved/All and text
  filtering.
- Extend the existing selectable ListView-header navigation used when the
  sidebar is hidden to provide multi-select Tags access. Keep the same selected
  tag set and immediate intersection filtering in both presentations; do not
  introduce a separate Tags button/navigation pattern.
- Retain existing text filtering and Unread/Saved/All controls.
- Show the selected combination's unread count by counting unread Entries that
  carry every selected tag. Count each matching Entry once; never sum individual
  tag counts to obtain the intersection count.
- Preserve the current list when an Entry becomes read, loses its color marking,
  or loses a selected tag. Explicit navigation or filter changes, including
  changing selected tags, establish refreshed membership.
- Reflect assignment and definition changes without losing current navigation
  context; handle empty results and responsive navigation.
- Do not introduce a second tag authority or persisted display-name cache.

## Remaining work

- Implement query predicates and suitable list-tag indexing; verify query plans
  and performance at the multi-million-entry design target.
- Connect platform queries, navigation scope, ListView loading, and live updates.
- Verify matching/nonmatching Entries, empty sets, keyboard/responsive access,
  read-only browsing, tag rename/removal, and current-list stability.
- Run affected Rust/frontend checks and production build; document the behavior.

## References

- [Tag storage](../../docs/architecture/tags.md).
- [Organization model](../../docs/architecture/organization_model.md)

- [Tag creation API](../../docs/architecture/tags.md#app-creation-api).
- [Entry assignment API](../../docs/architecture/tags.md#entry-assignment-api).
- `src/components/list/ItemList.tsx`
- `src/components/sidebar/Sidebar.tsx`
- `src/hooks/useRouter.tsx`
- `crates/xaga-core/src/embeddings/db.rs`
- `docs/architecture/list_view.md`
- `docs/architecture/design_scale.md`
