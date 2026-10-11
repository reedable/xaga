# Bookmark-import update semantics

In `src-tauri/src/commands/bookmarks.rs`, importing an existing bookmarked entry
enqueues `SyncState{is_bookmarked}` instead of a content upsert.

Decide whether import should update only bookmark state for an existing entry
or refresh its content too.
