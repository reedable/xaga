# App layout

`AppLayout` owns independent Sidebar and EntityList widths in rem. Sidebar starts
at `15rem` and ranges from `15rem` to `22.5rem`; EntityList starts at `20rem` and
ranges from `20rem` to `30rem`. Changing one never changes the other. The shared
`ColumnResizeHandle` supports pointer capture, arrow keys in `0.0625rem` steps,
Home for minimum, and End for maximum. Pointer widths use the same increments.
The focusable vertical separators expose their names, controlled columns, bounds,
and current values, with visible keyboard focus.

Sidebar visibility depends on the app layout width in CSS pixels and the existing
Show Sidebar preference. It hides below 980 without Search, or below 1300 with
Search, including drawer Search. Equality permits visibility. Column widths do
not add another collapse rule. Existing height-dependent Sidebar rendering and
the hidden-Sidebar collection selector are retained.

The remaining reading-space container has a zero minimum width so flex content
cannot impose its intrinsic width on the surrounding columns. Its inner width,
excluding its border, determines Search presentation: at least `38rem` keeps
Search inline at `18rem`, leaving at least `20rem` for the reader. Below `38rem`,
Search uses a right modal drawer. The container always represents reader plus
potential inline Search space, so opening the drawer does not change the measured
budget or cause presentation oscillation. A ResizeObserver tracks layout changes,
including column resizing and root font scaling. With Search closed, no automatic
column shrinking or extra reader-width collapse rule applies.

`SearchPanel` moves a stable portal host between inline and MUI Drawer containers.
Search input, result filters, results, and scroll state remain mounted through
presentation changes. Drawer focus stays within the modal, Escape and backdrop
click dismiss it, and closing Search restores its opener when available. Search
navigation continues to use `useRouter`; presentation changes do not change the
selected collection or article. Inspector responsiveness remains owned by
`EntityView`; an open Search keeps the Article panel hidden as before.

Browser coverage in `e2e/resizable-layout.spec.ts` verifies independent bounds,
keyboard access, threshold equality, root font scaling, live transitions, result
and reading context, alignment, and drawer dismissal. Capture Search uses fixture
title matches; it does not verify native IPC, embedding ranking, or native WebKit
focus and pointer behavior.
