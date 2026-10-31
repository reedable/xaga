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
