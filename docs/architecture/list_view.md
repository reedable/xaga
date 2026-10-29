# ListView behavior

The ListView preserves the user's browsing context while they work. Changing an
item's read/unread state updates its displayed state without removing its row
from the current list, even when the list was opened with the Unread filter.
Explicit navigation or changing the filter establishes a new list.

Removing a pin removes its shortcut without closing or clearing the list opened
through that shortcut. The open list's scope is independent of whether a shortcut
to it still exists.

These are expected Xaga interactions to preserve when changing list loading,
state-event handling, or navigation. They do not specify behavior for deletion
of the underlying content.

Relevant code: `src/components/list/ItemList.tsx`,
`src/hooks/useRouter.tsx`, and `src/components/sidebar/ShortcutList.tsx`.
