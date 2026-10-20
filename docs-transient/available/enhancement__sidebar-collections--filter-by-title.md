# Filter sidebar collections by title

## Summary

Add one shared title filter for the sidebar so a user can quickly find pinned
Shortcuts, Topics, Feeds, and Sites while retaining the existing accordion
interaction.

## Requirements

- Apply one shared, case-insensitive title substring filter to the pinned
  collections in Shortcuts and to all collections in Topics, Feeds, and Sites.
- Show each accordion section's matching collection count in its header,
  including zero, whether the section is expanded or collapsed.
- Keep all four accordion sections visible when they have no matches.
- Do not expand, collapse, or otherwise change the user's accordion state when
  the filter changes.
- Keep **All items** visible at the top of Shortcuts regardless of the filter;
  exclude it from the Shortcuts matching count.
- Filtering a selected collection out of the sidebar must not close its content,
  change the selection, or navigate elsewhere.
- Clear the filter when the sidebar is hidden and when the application closes;
  do not persist it across sidebar visibility changes or application launches.
- Keep the filter and match counts usable by keyboard and exposed with accessible
  labels and state.

## Remaining work

- Add the shared sidebar title-filter control and derive the four filtered
  collection lists from current provider state.
- Add matching counts to the Shortcuts, Topics, Feeds, and Sites accordion headers
  without changing their expansion state.
- Clear the filter on every sidebar-hide path.
- Cover title matching, counts, zero matches, permanent **All items** visibility,
  preserved accordion state, preserved selection, and filter clearing in tests.
- Run frontend lint, tests, and the production build.

## References

- `src/AppLayout.tsx`
- `src/components/sidebar/Sidebar.tsx`
- `src/components/sidebar/ExpandableList.tsx`
- `src/components/sidebar/ShortcutList.tsx`
- `src/components/sidebar/TopicList.tsx`
- `src/components/sidebar/FeedList.tsx`
- `docs-transient/available/enhancement__sites--browse-by-hostname.md`
- `src/providers/AppearanceProvider.tsx`
