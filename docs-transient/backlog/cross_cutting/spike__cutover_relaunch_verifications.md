# Outstanding relaunch verifications from the Phase C cutover (pending — may be complete)

Cutover steps recorded as "verify on relaunch" whose verification was not subsequently marked done in the working notes:

- Sidebar feed/topic counts via the Bitmap indexes — counts should match the old values.
- `EntryStateCache` retirement — frontend rebuild (the `isCacheReady` TS rename) + relaunch.
- `list_saved` repoint — bookmarks + annotations from LanceDB.
- `list_entries` per-feed repoint — `.atom` id-set → `entries_by_ids`.
- The annotation two-phase fix — highlight appears in Saved immediately; color change; un-highlight.

The app has been used since these were recorded, so some or all may be implicitly verified; this item exists so the checklist isn't lost with the working notes. Confirm each and close.
