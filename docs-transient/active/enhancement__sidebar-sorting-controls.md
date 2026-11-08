# Shared sidebar sorting control

## Summary

Replace per-section sorting controls with one sidebar sort button and synchronized
commands in the macOS View menu.

## Requirements

- Place a keyboard-accessible sort button immediately left of Refresh. Its menu
  offers **Sort by unread** and **Sort by name**, indicating the current choice.
- Put the same commands in the macOS View menu, grouped with **Show Sidebar**.
  Both surfaces update and reflect one persisted preference.
- Initialize the shared preference to unread, ignoring old per-section choices.
  Restore the shared choice on subsequent launches.
- Apply the choice to Shortcuts, Topics, Feeds, and Sites, including nested
  shortcut groups. Tags remain sorted by name; All items stays fixed above the
  collection filter.
- Preserve existing sort semantics and stored pin order. Keep sorting available
  in read-only libraries.

## Remaining work

- Replace per-section controls and preferences with the shared control and state.
- Test keyboard access, checked state, synchronization, persistence, read-only
  use, nested groups, and unread-count updates. Run relevant component checks,
  `npm run test:browser`, and native menu verification.

## References

- [Nested pin groups](../../docs/architecture/pins.md#nested-groups-015)
- [Pin-store contract](../../docs/architecture/pins.md)
- [Sidebar](../../src/components/sidebar/Sidebar.tsx)
- [Sort preference](../../src/hooks/useSidebarSort.ts)
- [Sort control](../../src/components/sidebar/SidebarSortControl.tsx)
