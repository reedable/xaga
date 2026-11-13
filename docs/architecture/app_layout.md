# App layout

`src/layout.css` owns window content widths, bounds, and responsive thresholds as
CSS custom properties in `rem`. Styles consume these properties directly;
`getLayoutWidth` reads their rem values for JavaScript interactions. Browser
geometry is converted using the current document root font size. Generated
article HTML includes the same stylesheet: article content has a `42.5rem`
maximum, and horizontal padding derives from the difference between that maximum
and the `46.5rem` reading-pane maximum.

The native main window has a minimum content size of 320 × 256 logical pixels,
configured in Tauri. Native window dimensions are independent of WebKit root font
scaling. `main_window_layout` verifies the Cocoa constraints and resizing at both
minimum dimensions without opening an account or library.

`AppLayout` retains independent Sidebar and EntityList widths in refs, in rem.
Sidebar starts at `15rem` and ranges from `15rem` to `22.5rem`; EntityList starts
at `20rem` and ranges from `20rem` to `30rem`. Changing one never changes the
other. The shared `ColumnResizeHandle` reads defaults and bounds from CSS and
supports pointer capture, arrow keys in `0.0625rem` steps, Home for minimum, and
End for maximum. Pointer widths use the same increments. The focusable vertical
separators expose their names, controlled columns, bounds, and current values,
with visible keyboard focus.

During pointer dragging, the handle records the latest width and schedules one
animation-frame update to the column's CSS custom property and accessible value.
The last update is flushed on release, cancellation, or lost pointer capture.
Keyboard changes apply immediately. Width changes do not update React state or
rerender the app tree; each column's CSS property controls its own flex width.
Refs retain widths when Sidebar temporarily unmounts. Sidebar visibility follows
the Show Sidebar preference; window width does not automatically hide it.
Existing height-dependent Sidebar rendering and the hidden-Sidebar collection
selector are retained.

The remaining `ReadingSpace` container has a zero minimum width so flex content
cannot impose its intrinsic width on the surrounding columns. Its inner width,
excluding its border, determines both reading-pane and Search presentation.
ResizeObservers track column resizing, window resizing, and root font scaling.
The measured container remains independent of drawer widths, so opening or
closing a drawer does not change the budget or cause presentation oscillation.
`ReadingSpace` permits overflow so the fixed article drawer and backdrop can
cover the columns; individual reader and Search surfaces own their clipping.

Below `20rem` of available inline space, the reading pane uses a right modal
drawer with a `20rem` minimum and `46.5rem` maximum. Equality presents it inline.
One permanently mounted, nonportaled Modal host contains the reading pane in
both presentations. Positioning, visibility, and modal behavior change; its
component subtree, DOM parentage, reader state, and hooks remain in place.
The `EntityView` flex item has a zero minimum height so a populated Inspector
scrolls within the window rather than stretching the reading pane and outer page.
Dismissal hides the toolbar along with the article. A visible close button,
Escape, and backdrop click dismiss the drawer. The underlying columns are inert
while it is open, and focus remains within the topmost modal surface.

`AppLayout` owns article visibility outside router state. `ReadingPaneContext`
passes content activation from navigation actions, including reselection of the
current Entry, without adding history. Entry, annotation, article-shortcut, and
URL actions show their content in the current presentation. Narrowing with
content selected opens the drawer; widening shows it inline. Back and Forward
use the existing router behavior while retaining an open drawer. Presentation
changes and dismissal do not change selection or history. On dismissal,
EntityList focuses the selected article, scrolling its virtualized entry into
view if necessary; an unavailable entry falls back to its focusable header.

Search and article/annotation details use modal surfaces above the article
drawer. Escape closes the topmost surface first. Address editing and Find
consume their own Escape events. The HTML reader accepts Escape messages only
from its own iframe, closes Find or the annotation tooltip first, and otherwise
forwards the key through its persistent parent hierarchy to modal handling.

At least `45rem` of reading space keeps Search inline at `18rem`; below that,
Search uses a right modal drawer. `SearchPanel` renders separate inline and drawer
result subtrees, with the drawer retained while closed. Their local input drafts
are separate; the committed search query belongs to the router. Presentation
changes do not change the selected collection or article. Inline Search expands
over the Inspector inside an absolutely positioned, overflow-hidden container.
Its 300 ms expansion uses the existing easing when motion is permitted; reduced
motion opens immediately. The Inspector remains mounted at its full content
width. The Article pane keeps the same position and width throughout the slide.
Inspector responsiveness remains owned by `EntityView`, using the shared
`43.75rem` threshold; Search does not hide or resize the Inspector.

Browser coverage in `e2e/article-drawer.spec.ts` verifies article navigation,
reselection, history, threshold equality, root font scaling, column resizing,
drawer bounds, toolbar access, dismissal, focus return, and reader DOM identity.
`e2e/resizable-layout.spec.ts` verifies independent column bounds, keyboard
access, Search transitions, results and reading context, alignment, and dismissal.
Capture Search uses fixture title matches; browser checks do not verify native
IPC, embedding ranking, or native WebKit focus and pointer behavior.
