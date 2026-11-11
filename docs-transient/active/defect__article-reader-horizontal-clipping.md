# Article reader horizontal clipping

## Summary

Resolve left-edge clipping of article content in narrow reading panes. The browser
capture at viewport width 1089 CSS pixels with EntityList at `30rem` and inline
Search at `18rem` shows a `20rem` reader horizontally displaced beneath EntityList.
This occurs with the existing layout, independently of the Search drawer prototype.

## Requirements

- Article content must start inside the reading pane without retained horizontal
  scroll displacing its left edge beneath EntityList.
- Preserve article wrapping, toolbar access, keyboard focus, and Search behavior
  across layout transitions. Fix the cause rather than resetting scroll solely
  for screenshots or changing the selected Search/Sidebar thresholds.
- Keep regression coverage focused on visible content alignment and the relevant
  interaction. Follow the shared UI testing and formatting conventions.

## Remaining work

- Reproduce with capture mode, an open article, EntityList at `30rem`, and Search
  opened at viewport width 1089 and height 800 CSS pixels. Identify why the reader
  layout retains horizontal scroll after Search opens. In the observed state,
  the reader starts at x=423 while its layout container starts at x=481 and has
  `scrollLeft=58`; the displacement persists after 5.5 seconds and input blur.
- Correct the overflow/focus/scroll interaction established by that investigation.
  Verify adjacent widths and repeated Search opening, closing, and presentation
  transitions alongside the resizable-column implementation.
- Run relevant browser checks and identify any native WebKit verification gap.

## References

- [Clipped capture](../ideas/sidebar-search-boundaries/w1089-s360-l480-search.png)
- [Capture geometry](../ideas/sidebar-search-boundaries/measurements.json)
- [App layout](../../docs/architecture/app_layout.md)
- [Reader layout](../../src/components/content/EntityView.module.css)
- [Article reader layout](../../src/components/content/ArticleReader.module.css)
- [Article toolbar](../../src/components/content/Toolbar.tsx)
- [UI testing convention](../../docs/conventions/ui_testing.md)
