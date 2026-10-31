# Article Inspector

The article Inspector replaces the Related-only aside with Related and Topics
views. The Tags tab is disabled with a visible explanation until Entry tag
assignment is available. Its future picker belongs in this shared Inspector
content, not in a separate toolbar or drawer implementation.

## Target and view state

The Inspector targets the article identified by the reader route's `entryId`.
Selecting an annotation keeps this parent Entry as its target; annotation IDs,
popup selections, and `articleScopeId` do not replace the inspected article.
Related retains its existing ranking and item-selection navigation.

`ItemView` owns the selected view and drawer visibility independently of route
identity and reader reload revisions. View selection lasts for the app session
across article navigation, drawer reopening, and resizing. This state is not a
library setting and is not persisted. Opening or closing the Inspector does not
navigate or remount the article reader, preserving its current reading position.

## Responsive presentation and keyboard access

At reader-container widths of at least 700 CSS pixels, a 280-pixel Inspector
panel sits beside the reader. Below that boundary, an article-toolbar button
opens the same view component in a right-hand drawer. The breakpoint measures
the reader container rather than the browser viewport, so sidebar and search
layout changes are reflected too.

Tabs support keyboard switching. The narrow opener identifies a dialog and
exposes its expanded state. The drawer has an accessible name, a Close action,
Escape/backdrop dismissal, focus containment, and focus return to its opener.
Resizing an open drawer to the wide layout closes it and focuses the selected
panel tab. Drawer presentation is static; Topic row rearrangement animates only
under `prefers-reduced-motion: no-preference`, following the
[motion convention](../conventions/motion.md).

## Topic membership

The Topics view reads the parent Entry and embeds the same Topic picker used by
existing Add to Topic controls. Filtering, sorting, inline Topic creation, and
membership operations share their implementation. In read-only mode, membership
remains visible while assignment, removal, and creation are disabled.

The library files remain authoritative. `useTopics` records successful creation
with a member, addition, and removal in a library-session projection owned by
`TopicsProvider`. Membership lookups apply those committed deltas over the
LanceDB reverse index, which may lag the writer. The projection survives picker
remounts and is discarded with the library session; it does not become another
persistent membership store. Failed writes do not change membership selection.
Results from an earlier article cannot replace the current article's state.
