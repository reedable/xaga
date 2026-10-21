# Sort sidebar collection sections

## Summary

Let a user independently order Shortcuts, Topics, Feeds, and Sites alphabetically
or by unread count, and restore each section's choice across application launches.

## Requirements

- Provide exactly two sort modes for each section: unread count and alphabetical
  title.
- Let Shortcuts, Topics, Feeds, and Sites hold independent sort selections.
- Place each section's sort control inside its expanded content, immediately
  before its collection rows; it does not need to remain reachable while the
  section is collapsed.
- Changing a sort selection must not expand, collapse, or otherwise change the
  accordion state.
- Default every section to unread-count ordering when it has no saved selection.
- Preserve the current unread-count ordering semantics: unread count descending,
  then alphabetical title.
- Sort alphabetically by title using the existing title comparison semantics.
- In Shortcuts, place the sort control before **All items**. Keep **All items**
  fixed as the first collection row and sort only the pinned collections beneath
  it.
- Apply sorting to current provider state without changing the authoritative
  order or data held by the Feed and Topic providers.
- Persist the four sort selections as client-side view state and restore them
  across application launches.
- Keep each sort control usable by keyboard and exposed with an accessible name
  and selected state.

## Remaining work

- Add an independent sort control to each sidebar section.
- Derive sorted Shortcuts, Topics, Feeds, and Sites for presentation while
  preserving **All items** placement.
- Store and restore each section's selection through the existing client-side
  view-state mechanism.
- Test both modes, unread-count title tie-breaking, independent selections,
  persistence, live collection updates, and permanent **All items** placement.
- Run frontend lint, tests, and the production build.

## References

- `src/components/sidebar/Sidebar.tsx`
- `src/components/sidebar/ExpandableList.tsx`
- `src/components/sidebar/ShortcutList.tsx`
- `src/components/sidebar/TopicList.tsx`
- `src/components/sidebar/FeedList.tsx`
- `src/components/sidebar/SiteList.tsx`
- `src/providers/FeedsProvider.tsx`
- `src/providers/TopicsProvider.tsx`
- `src/providers/ExpandedListProvider.tsx`
- `src/storage/viewState.ts`
- `src/utils/sortByTitle.ts`
- `src/utils/sortByUnreadCount.ts`
- `docs/architecture/sites.md`
