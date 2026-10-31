# Inspector

One shared Inspector implementation provides Related and Topics views for Articles and
Annotations. It replaces the standalone Add to Topic button and its drawer.
The Tags tab is disabled with a visible explanation until Entry tag assignment
is available. Its future picker belongs in this shared Inspector
content, not in a separate toolbar or drawer implementation.

## Target and view state

`ItemView` derives the Article panel target from the reader route's Entry ID
and owns the drawer target as an Entry ID with an optional Annotation ID.
Article navigation selects the Article. Annotation navigation selects and
scrolls to the passage without opening the Inspector drawer; the reader and
Article panel retain the parent Article. Opening Annotation inspection requires
the explicit Inspect action. Navigation may update an already-open drawer's
target, but does not open a closed drawer. Opening the
Inspector from the Annotation color tooltip explicitly targets that tooltip's
Annotation without navigating or changing list scope. The tooltip saves pending
edits before opening the drawer; a failed save leaves it open and reports the
error. Opening the drawer preserves the tooltip in place behind the modal
backdrop. Drawer interaction and focus changes do not dismiss it. Close and
backdrop clicks retain their drawer origin through dismissal, so the same
event does not also trigger the tooltip's click-away dismissal. Closing the
drawer restores focus to its tooltip opener when available, with the reader as
fallback. The tooltip remains visible after the drawer closes; a subsequent
click outside it dismisses it normally. Deleting the Annotation removes its
retained tooltip without saving it.
Read-only inspection does not save. Uncolored text selections do not offer the
Inspector action until a color has been assigned.

The drawer has one visible heading identifying its target: Annotation Inspector
or Article Inspector. The shared content does not repeat that heading inside
the drawer. The Annotation drawer remains scoped to its Annotation, without a
control to switch to Article inspection. Recoloring follows
the committed replacement event's old-to-new Annotation identity; removing the
inspected Annotation returns inspection to the parent Article. Navigation to a
different Article or Annotation resets the drawer target to that navigation context.
`articleScopeId` does not independently determine the target. Related uses the
inspected item's ID and retains its ranking and item-selection navigation.

`ItemView` owns Article and Annotation view selections and drawer visibility
independently of reader reload revisions. Each view selection lasts for the
app session across navigation, drawer reopening, and resizing. Changing the
Annotation drawer's tab does not change the Article panel's selected tab. This
state is not a library setting and is not persisted. Opening or closing the Inspector does not
navigate or remount the article reader, preserving its current reading position.

## Responsive presentation and keyboard access

For Article inspection, a 280-pixel panel sits beside the reader at
reader-container widths of at least 700 CSS pixels. Below that boundary, an
article-toolbar button opens the same Inspector in a right-hand drawer. The
breakpoint measures the reader container rather than the browser viewport.

Annotation inspection always opens the shared Inspector in the right-hand
drawer, including when opened from the Annotation tooltip on wide layouts.
The drawer overlays the main window. The Article panel and tooltip remain
mounted in place; opening it does not change their content, reader geometry,
or reading position. Both presentations share the Inspector implementation,
with explicit Article and Annotation targets. Resizing
does not move an inspected Annotation into the panel. Closing Annotation
inspection leaves the existing Article panel in place.

Tabs support keyboard switching. The narrow toolbar opener identifies a dialog
and exposes its expanded state. The drawer has an accessible name, a Close
action, Escape/backdrop dismissal, and focus containment. Closing returns focus
to the article-toolbar opener, or to the retained tooltip opener after opening from
a tooltip, with the reader iframe as fallback. Resizing an Article drawer to the
wide layout closes it and focuses the selected panel tab.

The shared drawer slides in from the right only when
`prefers-reduced-motion: no-preference` matches. Otherwise it opens without
animation. Topic row rearrangement follows the same motion preference, per the
[motion convention](../conventions/motion.md).

## Topic membership

The Topics view reads an Article target as an Entry. For an Annotation target,
it reads the parent Article through `readArticleScope` (without marking it read)
and resolves the Annotation by ID. A missing Annotation never falls back to
Article memberships. Both target types use the single inline `TopicMembership`
implementation for filtering, sorting, inline Topic creation, and membership
operations. Annotation writes carry the typed Annotation and its parent
reference. In read-only mode, membership remains visible while assignment, removal, and creation are disabled.

The library files remain authoritative. `useTopics` records successful creation
with a member, addition, and removal in a library-session projection owned by
`TopicsProvider`. Membership lookups apply those committed deltas over the
LanceDB reverse index, which may lag the writer. The projection survives picker
remounts and is discarded with the library session; it does not become another
persistent membership store. Failed writes do not change membership selection.
Results and pending membership writes from an earlier target cannot replace
the current target's displayed state.

## Color palette layout

The shared Article/Annotation palette uses the filled trash-can icon for
Remove marking. Its swatches slightly overlap when the Remove confirmation
label is shown. The label uses its natural content width so the
full text remains visible. Compaction is static base layout and works
independently of motion support. Label expansion, swatch spacing transitions,
and hover scaling animate only under `prefers-reduced-motion: no-preference`;
reduced-motion or unsupported media queries retain the same complete layout
without transitions.
