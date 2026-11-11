# Keep shortcut group headings visible while scrolling

## Summary

Implement sticky group headings in the sidebar's Shortcuts list so users can
identify containing groups while scrolling through their descendants. Display at
most five sticky headings without limiting stored group depth. Start with the
outermost five levels and provide a working implementation for hands-on review
before designing adaptive overflow behavior.

## Requirements

- Keep containing group headings visible at the top of the Shortcuts scrolling
  area while their descendants pass beneath them. Stack nested headings below
  their ancestors; release each heading when its group ends and replace sibling
  headings as scrolling enters another group.
- Make group levels one through five eligible for sticky positioning, counting
  root groups as level one. Deeper headings scroll normally. Preserve the complete
  hierarchy and existing group creation and move semantics, including trees up to
  the supported 24 levels.
- Use this fixed cap for the initial implementation. Do not add adaptive heading
  selection, reserved shortcut-space thresholds, or compact ancestor paths.
  Present insufficient-space behavior for hands-on review before deciding on a
  further overflow policy.
- Account for actual heading heights, including wrapped titles. Update nested
  offsets when titles, font metrics, available width, filtering, or expansion
  change. Do not assume a fixed height per depth.
- Use opaque theme-aware backgrounds and stacking order that keep headings and
  controls readable as descendants scroll beneath them. Follow the rem styling
  convention and preserve visible keyboard focus.
- Preserve expansion, actions menus, unread counts, drag and drop, filtering,
  saved expansion state, and read-only navigation. Reuse the existing group
  controls and accessible names rather than introducing duplicate focusable or
  screen-reader headings. Verify collapse behavior with both motion preferences.
- Work in both expandable-section and compact-selector sidebar layouts. The
  linked responsive-layout story owns their switching behavior and threshold.
- Cover nested stacking, sibling and root boundaries, wrapped titles, resizing,
  font scaling, the five-level sticky boundary, and keyboard traversal. Include
  depth five and deeper trees, actions/count visibility, filtering, expansion,
  and drag-and-drop workflows. Check access to the final shortcut and focused
  controls in short viewports; report any occlusion with a reproduction for review.
- Update durable group behavior documentation and affected user instructions.
  Complete when the fixed-cap behavior is available for hands-on review, relevant
  browser and component checks pass, and short-viewport limitations and any native
  verification gaps are explicitly reported.

## Remaining work

- Implement heading positioning, nested offsets and layout updates, backgrounds,
  stacking, and the five-heading cap in the existing shortcut tree.
- Extend the capture platform's fixtures and group-edit support for meaningful
  workflow coverage rather than introducing another mock backend.
- Add or update component and browser tests, run `npm run test:browser` and
  relevant frontend checks, and verify rendering and focus in native WKWebView.
  Coordinate compact-layout integration coverage with the responsive-layout story.

## References

- [Prototype evidence](../ideas/shortcuts-sticky-headings/index.md)
- [Height-responsive sidebar layout](../../docs/architecture/pins.md#sidebar-height)
- [Shortcut tree](../../src/components/sidebar/ShortcutTree.tsx)
- [Tree styles](../../src/components/sidebar/ShortcutTree.module.css)
- [Tree tests](../../src/components/sidebar/ShortcutTree.test.tsx)
- [Section scrolling and collapse](../../src/components/sidebar/ExpandableList.tsx)
- [Capture platform](../../src/platform/capture/MockPlatform.ts)
- [Nested group contract](../../docs/architecture/pins.md#nested-groups-015)
- [UI and UX testing](../../docs/conventions/ui_testing.md)
- [UI styling](../../docs/conventions/ui_styling.md)
- [Motion](../../docs/conventions/motion.md)
