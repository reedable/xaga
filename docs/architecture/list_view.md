# ListView behavior

The ListView preserves the user's browsing context while they work. Changing an
Entry's read/unread state changes its appearance without removing it
from the current list, even when the list was opened with the Unread filter.
Assigning, recoloring, or removing an article bookmark likewise updates the
displayed article without changing which articles are in the list, including a Site’s Saved list.
Explicit navigation or changing the filter establishes a new list.

Removing a pin removes its shortcut without closing or clearing the list opened
through that shortcut. The list remains open even after its shortcut is removed.

These are expected Xaga interactions to preserve when changing list loading,
state-event handling, or navigation. They do not specify behavior for deletion
of the underlying content.

Relevant code: `src/components/list/EntityList.tsx`,
`src/hooks/useRouter.tsx`, and `src/components/sidebar/ShortcutList.tsx`.

## Tag browsing

The Tags sidebar section and the Tags options in the existing selectable header
share the route's selected tag IDs. Multiple selections intersect: each Entry
must directly carry every selected identity. An empty selection in Tags browsing
shows all Entries, rather than the mixed Entry/Annotation All items shortcut.
With tags selected, Saved/All and text filtering remain available. The Unread
button is hidden, and a retained/default Unread filter is treated as All in that
view. Clearing tag selection restores the Unread control and the retained filter.
Selecting an article uses normal Article navigation and retains the selected tag set. Back/forward navigation
restores that set; it is not a library setting.

Sidebar tags sort alphanumerically by their current display names, ignoring case
and ordering numbers naturally (Tag 2 before Tag 10). Sorting changes the display order, not the order saved in the registry.

The header menu remains open while toggling Tags, applying each choice
immediately. Choosing another collection closes it and leaves Tags browsing.
Sidebar choices expose checkbox state and keyboard activation; the header uses
MUI's keyboard-operable multi-select options. Browsing is available read-only.

Labels come from the current library registry. Renaming a definition updates its
label without changing the selected identity or reloading list membership. A
selected definition that is absent from a registry refresh displays “Unavailable tag” and remains
deselectable; it is never displayed as an internal ID or silently converted to
an empty selection. A new query with an unavailable selection returns no rows.
Definition snapshots are session UI state, not a persisted name cache. The
explicit Delete action removes the selected ID while retaining the current list
and reading pane selection. A pending query for the previous selection is discarded. The next explicit
navigation, filter change, or refresh queries the remaining set, or all Entries
under the existing filters when it is empty. See the [deletion selection
contract](organization_model.md#delete).

Reading/color events update displayed articles without removing them. Assignment
changes and index notifications do not reload Tags list membership. Changing
the selected set, an available reading-state filter, text filter, or explicitly refreshing
establishes a fresh list. Feed refresh notifications also preserve the active
Tags list. Results for a previous collection or selection cannot replace the list for the current one.

The selected combination's unread count remains available through the query API
and navigation provider. The sidebar and ListView header do not display it.
The count uses the intersection independently of reading-state and text filters,
counting each matching Entry once. Index-drain and unread events refresh it with
debouncing; results from earlier selections are discarded.

See [Tag query API](tags.md#tag-query-api) and
[query scaling](design_scale.md#tag-query-scaling).

## Article shortcuts

An article shortcut sets which Article and its Annotations the list contains,
separately from the currently selected article or annotation (`articleScopeId`
versus `entryId`/`annotationId`). The list contains the article
first, then its annotations in passage order. Selecting an annotation opens its
parent article and scrolls to that annotation without changing the list target.
Unpinning or changing reading state preserves the open list and reading context.

Passage ordering resolves stored text segments and their zero-based occurrences
against the rendered full body (or summary only when the body is absent), using the
reading pane's per-text-node, non-overlapping occurrence convention. Multi-segment
annotations sort at their earliest resolved segment, even if another segment fails
to resolve. Annotations with no resolved segment appear last, without a guaranteed
order among them. Creation, edits, replacement and removal refresh the scoped list
from the article file. Text filtering remains available; reading-state filters and
Mark all read are omitted in this scope. Selection and keyboard movement use the Annotation ID when an Annotation is selected.

Pinned articles appear alongside pinned Feeds and Topics, including in the
selectable header when the sidebar is hidden. Article shortcuts display no count
and contribute zero to unread-count sorting regardless of their reading state;
alphabetical sorting uses their titles.
