# Bookmark-import "updated" path switched to SyncState (pending — unapproved change awaiting decision)

In `src-tauri/src/commands/bookmarks.rs`, already-existing bookmarked entries on import now
enqueue `SyncState{is_bookmarked}` instead of the original content upsert. Behavior change made
during the decoupling work; flagged, not approved. Decision: ratify (state-only update
suffices for an existing entry) or revert (import refreshes content too).
