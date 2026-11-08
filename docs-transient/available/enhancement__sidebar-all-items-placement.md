# Move All items above the collection filter

## Summary

Move All items out of Shortcuts and place it above the sidebar's “Filter
collections…” input as a library-wide navigation item.

## Requirements

- Show All items once, below the existing sidebar toolbar and immediately above
  the collection filter, independently of section expansion and filter text.
- Preserve its navigation, selected state, unread count, and loading feedback.
- Keep the existing collection filter behavior and section order. Use spacing
  that makes the filter's relationship to the sections below clear.
- Preserve keyboard access and an accessible name for All items.

## Remaining work

- Move the All items rendering and its required state from Shortcuts into the
  sidebar, reusing existing navigation and count behavior.
- Update component and browser workflow coverage for its placement, navigation,
  selection, and availability while filtering or changing expanded sections.
- Run `npm run test:browser` and relevant component checks. Identify any native
  behavior those checks cannot verify and report remaining verification gaps.

## References

- [Sidebar](../../src/components/sidebar/Sidebar.tsx)
- [Shortcuts](../../src/components/sidebar/ShortcutList.tsx)
- [Shortcut component tests](../../src/components/sidebar/ShortcutList.test.tsx)
- [UI and UX testing](../../docs/conventions/ui_testing.md)
