# Combine sidebar sorting and filtering in a popup

## Summary

Replace the sidebar sorting menu with a popup containing a sorting dropdown and
the collection filter input. Remove the permanently displayed filter input from
the sidebar so collection lists have more vertical space.

## Requirements

- The existing sorting button opens one popup containing both controls. Keep
  All items outside the filtered collection lists.
- Preserve the shared sorting preference, existing sort choices, native-menu
  synchronization, save-failure behavior, and availability in read-only libraries.
- Reuse the existing collection-filter state and matching behavior, including
  shortcut ancestor paths and temporary expansion. Closing and reopening the
  popup retains the filter; users can clear it from the popup.
- Make an active filter discoverable while the popup is closed without reserving
  a permanent input row.
- Give the popup and its controls appropriate accessible names and roles, keyboard
  operation, visible focus, dismissal, and focus return to the trigger. Keep both
  controls usable in short windows and constrain the popup to the available space.
- Update affected browser workflows to open the popup before filtering or sorting.
  Test changes to both controls, retained filtering after dismissal, clearing,
  keyboard operation, read-only use, and shared sorting synchronization. Follow
  the testing conventions without hardcoded product copy.
- Update durable sidebar behavior documentation and affected user instructions in
  the implementation change. Complete when the inline input is removed, both
  popup controls preserve their contracts, and relevant checks pass.

## Remaining work

- Refactor the sorting control to accept the existing filter value and change
  handler, and replace its menu with the combined popup.
- Remove the inline filter layout and obsolete commented-out filter proposal.
- Update component and capture-platform browser coverage; run
  `npm run test:browser` and relevant frontend checks. Identify any native-menu
  verification gap separately.

## References

- [Sidebar](../../src/components/sidebar/Sidebar.tsx)
- [Sorting control](../../src/components/sidebar/SidebarSortControl.tsx)
- [Sorting provider](../../src/providers/SidebarSortProvider.tsx)
- [Sidebar sorting contract](../../docs/architecture/pins.md#sidebar-sorting)
- [Browser sorting workflow](../../e2e/sidebar-sort.spec.ts)
- [Browser collection workflows](../../e2e/workflows.spec.ts)
- [UI and UX testing](../../docs/conventions/ui_testing.md)
- [UI styling](../../docs/conventions/ui_styling.md)
- [Height-responsive collection lists](enhancement__sidebar-space--height-responsive-lists.md)
