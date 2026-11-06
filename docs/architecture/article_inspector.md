# Inspector

One shared Inspector implementation provides Related and Topics views for Articles and
Annotations, and a Tags view for Articles. It replaces the standalone Add to Topic
button and its drawer. The Tags view uses searchable selectable chips for
immediate assignment/removal and creation with assignment, through the
[Entry assignment API](tags.md#entry-assignment-api).

## Target and view state

`EntityView` derives the Article panel target from the reading pane route's Entry ID
and owns the drawer target as an Entry ID with an optional Annotation ID.
Article navigation selects the Article. Annotation navigation selects and
scrolls to the passage without opening the Inspector drawer; the reading pane and
Article panel retain the parent Article. Selecting an Annotation within the
already displayed Article preserves its resolved URL and presentation rather
than clearing and restoring the URL, so passage navigation does not reload the
reading pane. Annotation list rows carry their parent Article URL through navigation,
so changing Articles does not first render the reading pane without a URL and then reload it when the URL arrives.
Opening Annotation inspection requires
the explicit Inspect action. Navigation may update an already-open drawer's
target, but does not open a closed drawer. Opening the
Inspector from the Annotation color tooltip explicitly targets that tooltip's
Annotation without navigating or changing list scope. The tooltip saves pending
edits before opening the drawer; a failed save leaves it open and reports the
error. Opening the drawer preserves the tooltip in place behind the modal
backdrop. Drawer interaction and focus changes do not dismiss it. Close and backdrop click handlers identify those events as drawer actions even
after dismissal, preventing the same click from also dismissing the tooltip. Closing the
drawer restores focus to its tooltip opener when available, with the reading pane as
fallback. The tooltip remains visible after the drawer closes; a subsequent
click outside it dismisses it normally. Deleting the Annotation removes its
retained tooltip without saving it.
Read-only inspection does not save. Uncolored text selections do not offer the
Inspector action until a color has been assigned.

The drawer has one visible heading identifying its target: Annotation Inspector
or Article Inspector. The shared content does not repeat that heading inside
the drawer. The Annotation drawer remains scoped to its Annotation, without a
control to switch to Article inspection. After a saved recolor replaces an Annotation, the replacement event supplies
the old and new IDs so inspection follows the new Annotation; removing the
inspected Annotation returns inspection to the parent Article. Navigation to a
different Article or Annotation resets the drawer target to that navigation context.
`articleScopeId` does not independently determine the target. Related uses the
inspected entity's ID and retains its ranking and entity-selection navigation.

`EntityView` owns Article and Annotation view selections and drawer visibility
independently of reading pane reload revisions. Each view selection lasts for the
app session across navigation, drawer reopening, and resizing. Changing the
Annotation drawer's tab does not change the Article panel's selected tab. This
state is not a library setting and is not persisted. Opening or closing the Inspector does not
navigate or remount the reading pane, preserving its current reading position.

The tab panels keep their children mounted while the Inspector remains open.
Switching tabs hides content without resetting its local state or restarting the
target lookup. The lookup depends on the inspected entity’s ID. Changing that ID resets the
child state so data from one Article or Annotation is not shown for another.

## Responsive presentation and keyboard access

For Article inspection, a 280-pixel panel sits beside the reading pane at
reading-pane container widths of at least 700 CSS pixels. Below that boundary, an
article-toolbar button opens the same Inspector in a right-hand drawer. The
breakpoint measures the reading pane container rather than the browser viewport.

Annotation inspection always opens the shared Inspector in the right-hand
drawer, including when opened from the Annotation tooltip on wide layouts.
The drawer overlays the main window. The Article panel and tooltip remain
mounted in place; opening it does not change their content, reading pane geometry,
or reading position. Both presentations share the Inspector implementation,
with explicit Article and Annotation targets. Resizing
does not move an inspected Annotation into the panel. Closing Annotation
inspection leaves the existing Article panel in place.

Tabs support keyboard switching. The narrow toolbar opener identifies a dialog
and exposes its expanded state. The drawer has an accessible name, a Close
action, Escape/backdrop dismissal, and focus containment. Closing returns focus
to the article-toolbar opener, or to the retained tooltip opener after opening from
a tooltip, with the reading pane iframe as fallback. Resizing an Article drawer to the
wide layout closes it and focuses the selected panel tab.

The shared drawer slides in from the right only when
`prefers-reduced-motion: no-preference` matches. Otherwise it opens without
animation. The tab indicator uses the MUI theme's transition when motion is
permitted and switches without animation otherwise. Topic row rearrangement
follows the same motion preference, per the
[motion convention](../conventions/motion.md).

## Topic membership

Until the shared Topics provider has loaded its first list, the Topics view
shows Loading Topics… rather than treating an unloaded list as empty or offering
Create a New Topic. Once loaded, the list remains available during refreshes.

The Topics view reads an Article target as an Entry. For an Annotation target,
it reads the parent Article through `readArticleScope` (without marking it read)
and resolves the Annotation by ID. A missing Annotation never falls back to
Article memberships. Both target types use the single inline `TopicMembership`
implementation for filtering, sorting, inline Topic creation, and membership
operations. Annotation writes carry the typed Annotation and its parent
reference. In read-only mode, membership remains visible while assignment, removal, and creation are disabled.

The library files remain authoritative. While a library is open, `TopicsProvider`
retains the successful membership changes recorded by `useTopics`: creation with
a member, addition, and removal. Lookups combine these saved changes with the
LanceDB reverse index, which can lag behind file writes. This in-memory state
survives picker remounts and is discarded when the library session ends; it is
not saved as another membership store. Failed writes do not change membership selection.
Results and pending membership writes from an earlier target cannot replace
the current target's displayed state.

## Color palette layout

The shared Article/Annotation palette uses the filled trash-can icon for
Remove marking. Its swatches slightly overlap when the Remove confirmation
label is shown. The label uses its natural content width so the
full text remains visible. The base layout already uses the compact spacing; it does not depend on animation. Label expansion, swatch spacing transitions,
and hover scaling animate only under `prefers-reduced-motion: no-preference`;
reduced-motion or unsupported media queries retain the same complete layout
without transitions.
