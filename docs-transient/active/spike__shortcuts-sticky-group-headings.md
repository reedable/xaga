# Sticky group headings in Shortcuts

## Summary

Determine the changes needed to keep containing group headings visible while
scrolling through the Shortcuts hierarchy in the sidebar. The user has selected
sticky group headings as the starting direction: scrolling should not require
remembering group names that have left the viewport.

## Requirements

- Investigate headings that stick while their descendants scroll beneath them,
  with nested headings stacked below their ancestors and released when their
  group ends.
- Establish whether the existing scroll and MUI Collapse containers support this
  behavior with CSS alone or require component changes. Verify in the browser
  rather than inferring feasibility from the markup.
- Check wrapped group titles, sibling-group transitions, expansion and collapse,
  filtering, and reduced viewport height. Assess deep nesting against the
  supported 24-group limit; present any required overflow decision with evidence
  and trade-offs rather than silently choosing a depth cap.
- Preserve group controls, unread counts, drag and drop, keyboard focus visibility,
  and accessible names. Identify any interference caused by overlapping headings.
- Complete when browser evidence establishes feasibility and limitations, and a
  bounded implementation story records the required changes and verification.
  Raise unresolved design decisions for the user before finalizing that story.

## Remaining work

- Inspect the scroll ancestors, collapse overflow, heading layout, and existing
  shortcut workflow coverage.
- Build a local prototype and exercise the required scenarios, recording the
  observed behavior and any necessary structural changes.
- Describe the implementation scope and remaining uncertainties based on the
  prototype, then capture remaining implementation work in a story. Keep the
  prototype separate from a production implementation until that work is selected.

## References

- [Nested shortcut groups](../../docs/architecture/pins.md#nested-groups-015)
- [Shortcut tree](../../src/components/sidebar/ShortcutTree.tsx)
- [Shortcut tree styles](../../src/components/sidebar/ShortcutTree.module.css)
- [Section scrolling and collapse](../../src/components/sidebar/ExpandableList.tsx)
- [Section styles](../../src/components/sidebar/ExpandableList.module.css)
- [Shortcut tree tests](../../src/components/sidebar/ShortcutTree.test.tsx)
- [UI and UX testing](../../docs/conventions/ui_testing.md)
- [UI styling](../../docs/conventions/ui_styling.md)
