# Adapt sidebar collection lists to available height

## Summary

Replace the expandable section headers with a collection-list dropdown below
All items when vertical space is limited. The selected list occupies the remaining
space. Integrate with the sticky-heading behavior defined in its separate story.

## Requirements

- Let `A` be the sidebar height available below All items and `H` the sum of the
  rendered heights of its expandable section headers. Use compact mode when
  `A < 2 * H`; use the expandable sections when `A >= 2 * H`. Measure the actual
  layout, including font scaling and wrapped labels, rather than a fixed window
  height. Keep the threshold independent of whether those headers are currently
  displayed to avoid layout oscillation.
- In compact mode, show one dropdown immediately below All items, offering
  Shortcuts, Topics, Feeds, Sites, and Tags. Replace the expandable headers;
  the selected list fills the remaining area and scrolls independently.
- Reuse the shared expanded-list selection rather than maintaining a competing
  selected-list value. Resizing and choosing a list must preserve the displayed
  article and browsing scope. When all sections are collapsed, show Shortcuts as
  the compact selector's default, reusing the existing default list choice.
- Preserve access to section counts, creation actions, missing-shortcut handling,
  filtering, sorting, group expansion and organization, and read-only behavior.
  Hidden lists must not compete for keyboard focus or accessibility queries.
- Preserve the full shortcut hierarchy and integrate the fixed display cap owned
  by the sticky-heading story. Check scrolling and keyboard navigation across
  layout changes and report any short-viewport sticky-heading occlusion for review.
- Cover both sides of the threshold and equality, live resizing, font scaling,
  empty and long lists, selection continuity, creation actions, filtering,
  read-only use, and integration with sticky headings. Check the final shortcut
  at depth five and beyond, including the supported 24-level hierarchy, and
  document any remaining sticky-heading space limitations.
- Update durable behavior documentation and affected user instructions. Complete
  when both layouts and their sticky-heading integration satisfy the selected policies and
  relevant browser and component checks pass.

## Remaining work

- Add height measurement and the compact section selector using the shared list
  state and existing list content. Keep section actions accessible without
  restoring the full stack of headers.
- Coordinate compact-layout integration with the separate sticky-heading story.
- Extend meaningful component/browser coverage. Run `npm run test:browser` and
  relevant checks; verify native WebKit
  layout separately and report any remaining verification gap.

## References

- [Sidebar sorting and filtering](../../docs/architecture/pins.md#sidebar-sorting)
- [Sticky group headings](enhancement__sidebar-space--sticky-group-headings.md)
- [Sticky-heading browser evidence](../ideas/shortcuts-sticky-headings/index.md)
- [Sidebar](../../src/components/sidebar/Sidebar.tsx)
- [Expandable section](../../src/components/sidebar/ExpandableList.tsx)
- [Section styles](../../src/components/sidebar/ExpandableList.module.css)
- [Shared expanded-list state](../../src/providers/ExpandedListProvider.tsx)
- [Shortcut tree](../../src/components/sidebar/ShortcutTree.tsx)
- [Nested group contract](../../docs/architecture/pins.md#nested-groups-015)
- [UI and UX testing](../../docs/conventions/ui_testing.md)
- [UI styling](../../docs/conventions/ui_styling.md)
