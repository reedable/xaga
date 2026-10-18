# Define existing-entry bookmark import semantics

## Summary

Importing a bookmark whose normalized URL already exists adds the bookmark tag and synchronizes that state to the embeddings database. Metadata from the bookmark file is ignored. Define whether existing entries should remain unchanged, merge selected imported metadata, or trigger a separate remote-content refresh; bookmark files do not contain article body content.

## Requirements

- Always preserve the existing entry's body, annotations, unrelated tags, and collection membership.
- Keep the existence check and resulting write serialized through the entry's library-writer lane.
- Do not treat bookmark-file metadata as remotely verified article content.
- Do not add network access to import without an explicit decision and the normal remote-content security controls.
- Keep the persisted entry and embeddings row consistent for every field the chosen behavior changes.

## Remaining work

- Compare tag-only behavior, merging selected bookmark metadata, and a separate remote refresh, including conflicts with locally stored and feed-derived values.
- Decide which behavior applies when imported fields are absent, older, or different from the existing entry.
- Define whether unchanged or already-bookmarked entries count as updated in the import result.
- Implement the chosen semantics and enqueue the corresponding state sync or entry upsert.
- Test existing, already-bookmarked, newly created, duplicate, and concurrently created entries.

## References

- `crates/xaga-core/src/persistence/bookmarks.rs`
- `crates/xaga-core/src/library_writer/mod.rs`
- `src-tauri/src/native/bookmarks.rs`
- `docs/architecture/security/handling_user_supplied_content.md`
- `docs/architecture/security/handling_remote_content.md`
- `docs/architecture/single_writer_library_lock.md`
