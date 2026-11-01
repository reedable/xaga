# ListView behavior

The ListView preserves the user's browsing context while they work. Changing an
item's read/unread state updates its displayed state without removing its row
from the current list, even when the list was opened with the Unread filter.
Assigning, recoloring, or removing an article bookmark likewise updates the
existing row without recomputing membership, including a Site’s Saved list.
Explicit navigation or changing the filter establishes a new list.

Removing a pin removes its shortcut without closing or clearing the list opened
through that shortcut. The open list's scope is independent of whether a shortcut
to it still exists.

These are expected Xaga interactions to preserve when changing list loading,
state-event handling, or navigation. They do not specify behavior for deletion
of the underlying content.

Relevant code: `src/components/list/ItemList.tsx`,
`src/hooks/useRouter.tsx`, and `src/components/sidebar/ShortcutList.tsx`.

## Tag browsing

The Tags sidebar section and the Tags options in the existing selectable header
share the route's selected tag IDs. Multiple selections intersect: each Entry
must directly carry every selected identity. An empty selection in Tags browsing
shows all Entries, rather than the mixed Entry/Annotation All items shortcut.
Unread/Saved/All and text filtering remain available. Selecting a row uses normal
Article navigation and retains the selected tag set. Back/forward navigation
restores that set; it is not a library setting.

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
explicit Delete action follows the [deletion selection
contract](organization_model.md#delete).

Reading/color events update existing rows without removing them. Assignment
changes and index notifications do not reload Tags list membership. Changing
the selected set, Unread/Saved/All, text filter, or explicitly refreshing
establishes a fresh list. Feed refresh notifications also preserve the active
Tags list. Superseded query results cannot replace a newer scope's list.

Both presentations show the selected combination's unread count, queried as an
intersection independently of the reading-state and text filters. It counts
each matching Entry once, including when its list row remains visible after
becoming read or losing an assignment. Index-drain and unread events refresh
the count with debouncing; results from earlier selections are discarded.

See [Tag query API](tags.md#tag-query-api) and
[query scaling](design_scale.md#tag-query-scaling).

## Article shortcuts

An article shortcut establishes an article list target independently of the selected
row (`articleScopeId` versus `entryId`/`annotationId`). The list contains the article
first, then its annotations in passage order. Selecting an annotation opens its
parent article and scrolls to that annotation without changing the list target.
Unpinning or changing reading state preserves the open list and reading context.

Passage ordering resolves stored text segments and their zero-based occurrences
against the rendered full body (or summary only when the body is absent), using the
reader's per-text-node, non-overlapping occurrence convention. Multi-segment
annotations sort at their earliest resolved segment, even if another segment fails
to resolve. Annotations with no resolved segment appear last, without a guaranteed
order among them. Creation, edits, replacement and removal refresh the scoped list
from the article file. Text filtering remains available; reading-state filters and
Mark all read are omitted in this scope. Row selection and keyboard movement use
annotation identity when an annotation is selected.

Pinned articles appear alongside pinned Feeds and Topics, including in the
selectable header when the sidebar is hidden. Article shortcuts display no count
and contribute zero to unread-count sorting regardless of their reading state;
alphabetical sorting uses their titles.
