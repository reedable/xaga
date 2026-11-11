# Resizable columns and conditional Search drawer

## Summary

Allow the user to resize Sidebar and EntityList by dragging their right edges.
Retain the existing Sidebar visibility rules and adapt Search presentation to the
space remaining for article content. Implement the selected layout represented by
the simplified-rule captures, without adding automatic column rearrangement.

## Requirements

- Sidebar starts at `15rem` and can be resized from `15rem` to `22.5rem`;
  EntityList starts at `20rem` and can be resized from `20rem` to `30rem`.
  Both right edges support pointer dragging and
  keyboard resizing, with accessible separator names, values, and visible focus.
- Retain the current Sidebar visibility decision based on the full app layout
  width: hide below 980 CSS pixels without Search and below 1300 CSS pixels with
  Search. Equality permits the Sidebar to show, subject to the user's existing
  Show Sidebar preference. Column widths and article width add no collapse rule.
- Preserve the existing height-dependent Sidebar rendering and collection dropdown
  when Sidebar is hidden.
- After the Sidebar visibility decision, keep Search inline when the space for
  article content plus Search is at least `38rem` (`608px` at a `16px` root size).
  Below that, present Search in a right overlay drawer. Search remains `18rem`
  wide, leaving at least `20rem` for an inline reader. Account for borders when
  measuring available space; do not use the entire window as this measurement.
- Determine Search presentation from the space it would occupy inline, including
  when the drawer is open. Opening or closing the drawer must not cause repeated
  switches between presentations. Do not introduce extra breakpoint bands.
- Keep the Sidebar rule based on whether Search is open, including drawer mode.
  Search presentation must not change the Sidebar threshold.
- Preserve the active query, results, article, and collection when Search changes
  presentation. Provide keyboard access, dismissal, and appropriate focus handling
  for the drawer. Closing Search must leave the reading context intact.
- Preserve the article inspector's existing responsive behavior. Do not introduce
  automatic column shrinking or additional Sidebar collapse to enforce a minimum
  article width when Search is closed.
- Review affected browser workflow coverage, including resizing, exact boundaries,
  drawer focus/dismissal, and retained reading/search state. Follow the shared UI
  testing and formatting conventions and report native WebKit verification gaps.

## Remaining work

- Implement the shared column-width layout and accessible resize controls.
  Confirm any proposed width persistence or reset interaction with the user;
  these behaviors were not selected by the screenshot study.
- Implement conditional inline/drawer Search using actual available layout space.
- Validate one pixel below, at, and above the existing Sidebar thresholds and the
  Search presentation boundaries for minimum and wider columns, including live
  resizing and root font scaling. Use the captures as visual references rather
  than fixed screenshot assertions.
- Update durable layout documentation and relevant User Guide instructions to
  describe the implemented rule and controls.

## References

- [Selected simplified-rule captures](../ideas/sidebar-search-simple/index.md)
- [Capture gallery](../ideas/sidebar-search-simple/index.html)
- [Reader clipping defect](defect__article-reader-horizontal-clipping.md)
- [UI testing convention](../../docs/conventions/ui_testing.md)
- [UI styling convention](../../docs/conventions/ui_styling.md)
- [App layout](../../src/AppLayout.tsx)
- [Sidebar sizing](../../src/components/sidebar/Sidebar.module.css)
- [EntityList sizing](../../src/components/list/EntityList.module.css)
- [Search panel](../../src/components/search/SearchResultList.tsx)
- [Article inspector layout](../../src/components/content/EntityView.tsx)
