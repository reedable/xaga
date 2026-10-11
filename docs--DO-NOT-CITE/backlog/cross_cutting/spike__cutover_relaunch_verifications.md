# Outstanding relaunch verifications from the Phase C cutover

Verify these behaviors after rebuilding and relaunching:

- Sidebar feed/topic counts via the Bitmap indexes — counts should match the old values.
- `EntryStateCache` retirement — frontend rebuild (the `isCacheReady` TS rename) + relaunch.
- `list_saved` repoint — bookmarks + annotations from LanceDB.
- `list_entries` per-feed repoint — `.atom` id-set → `entries_by_ids`.
- The annotation two-phase fix — highlight appears in Saved immediately; color change; un-highlight.
