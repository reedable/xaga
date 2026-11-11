# Sticky group headings in Shortcuts

## Summary

Resolve overflowing ancestor headings and scope implementation of sticky group
headings in the sidebar's Shortcuts hierarchy. The user has selected sticky group
headings as the starting direction: scrolling should not require remembering
group names that have left the viewport.

## Requirements

- Retain the selected direction of headings that stick while descendants scroll
  beneath them, with nested headings stacked below their ancestors and released
  when their group ends.
- Choose overflow behavior with the user, accounting for wrapped headings,
  reduced viewport height, and the supported 24-group limit. Use the prototype
  evidence and trade-offs rather than silently choosing a depth cap.
- Preserve group controls, unread counts, drag and drop, keyboard focus visibility,
  and accessible names. Identify any interference caused by overlapping headings.
- Complete when browser evidence establishes feasibility and limitations, and a
  bounded implementation story records the required changes and verification.
  Raise unresolved design decisions for the user before finalizing that story.

## Remaining work

- Present the overflow alternatives in the linked prototype evidence for the
  user's decision; prototype the chosen fallback to establish its limits.
- Capture a bounded implementation story covering heading offsets, updates after
  layout changes, stacking and backgrounds, the chosen overflow policy, and the
  browser/native verification gaps identified by the prototype. Delete this spike
  when its remaining work is resolved.

## References

- [Prototype evidence and overflow alternatives](../ideas/shortcuts-sticky-headings/index.md)
- [Nested shortcut groups](../../docs/architecture/pins.md#nested-groups-015)
- [Shortcut tree](../../src/components/sidebar/ShortcutTree.tsx)
- [Shortcut tree styles](../../src/components/sidebar/ShortcutTree.module.css)
- [Section scrolling and collapse](../../src/components/sidebar/ExpandableList.tsx)
- [Section styles](../../src/components/sidebar/ExpandableList.module.css)
- [Shortcut tree tests](../../src/components/sidebar/ShortcutTree.test.tsx)
- [UI and UX testing](../../docs/conventions/ui_testing.md)
- [UI styling](../../docs/conventions/ui_styling.md)
