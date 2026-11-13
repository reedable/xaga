# Responsive article drawer and centralized layout widths

## Summary

Make the reading pane responsive without changing article routing or rebuilding
its DOM, and centralize window layout widths in CSS.

## Requirements

- Set the main window minimum size to 320 × 256 logical pixels. Window dimensions
  use pixels; content dimensions use `rem`, independently of root font scaling.

- Below `20rem` of available inline reading space, present the reading pane as a
  right drawer with a minimum width of `20rem` and maximum of `46.5rem`. At or
  above the threshold, present it inline.
- Keep the same component subtree, DOM nodes, parentage, reader state, and hook
  wiring across presentation changes and dismissal. Do not unmount, recreate,
  reparent, portal elsewhere, or render a second reader.
- Manage drawer visibility in React state outside router state, scoped to survive
  article navigation. Selecting an Entry opens it when narrow, including the
  current Entry without adding a history entry. Existing annotation, article
  shortcut, and URL actions that display content must also show it in the
  current presentation. Narrowing with an article
  selected opens it; widening displays that article inline. Back and Forward
  render the destination article in the current presentation without closing an
  open drawer. Presentation changes and dismissal preserve selection and history.
  Back/Forward controls are hidden after drawer dismissal; do not introduce a
  separate closed-drawer history policy or a new navigation mechanism.
- Provide a visible close control, Escape dismissal, and backdrop dismissal.
  Return focus to the article's EntityList entry, or its header if unavailable.
- Retain all toolbar controls in their current arrangement inside the reading
  pane; closing the drawer hides them. Search and article details appear above
  the article drawer. Escape dismisses the topmost surface first; cancelling
  address editing or closing find must not also dismiss the article drawer.
- Centralize widths, bounds, and responsive width thresholds for Sidebar,
  EntityList, reading pane, Inspector, Search, and article content in CSS custom
  properties using `rem`. Preserve existing dimensions at the default root font
  size. The article content maximum is `42.5rem`; derive horizontal padding from
  its difference from the `46.5rem` pane maximum. JavaScript reads these
  authoritative values where interaction requires them.

## Remaining work

- Set the main-window minimum in its Tauri configuration and verify native
  resizing respects both dimensions.

- Address `ReadingSpace` overflow clipping and measure available inline space
  independently of drawer width to prevent oscillation. This work covers clipping
  that obstructs drawer presentation; it does not absorb the separate article
  content clipping defect. Close that defect only if its own reproduction and
  verification establish it is resolved by this change.
- Replace scattered constants in styles, resize handles, responsive measurements,
  and generated article HTML with the central properties. Convert browser
  geometry using the current root font size.
- Connect Entry activation despite the current router no-op for reselection.
  Integrate virtualized entry focus, a programmatically focusable header fallback,
  and iframe Escape messages with the existing controls.
- Verify through visible controls: narrow launch → Collection → first Entry →
  dismiss → second Entry → Back; and wide launch → Collection → first Entry →
  second Entry → narrow → Back → Forward → wide → Back. Also cover reopening
  the current Entry, annotation/shortcut/URL content opening, threshold equality and adjacent widths, root font scaling,
  column resizing, drawer bounds, toolbar access, focus, nested dismissal, and
  DOM identity and parentage across presentation changes.
- Update layout documentation, correcting its stale Search breakpoint and
  portal-host descriptions against the implementation.

## References

- [Layout architecture](../../docs/architecture/app_layout.md)
- [App layout](../../src/AppLayout.tsx)
- [Column resizing](../../src/components/ColumnResizeHandle.tsx)
- [Reading pane and inspector](../../src/components/content/EntityView.tsx)
- [Article toolbar](../../src/components/content/Toolbar.tsx)
- [Navigation hooks](../../src/hooks/useRouter.tsx)
- [Generated article HTML](../../src/text/buildEntryHtml.ts)
- [Existing clipping story](defect__article-reader-horizontal-clipping.md)
