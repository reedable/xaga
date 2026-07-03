# Trigger annotation embedding for semantic search (pending — possibly stale)

The item as recorded: annotation embedding is not triggered for semantic search (cited
`articles.rs`).

Status caveat, honestly marked: the current code appears to cover this —
`add_annotation` (`src-tauri/src/commands/articles.rs`) enqueues `SyncAnnotations` then
`EmbedEntry`, whose call-site comment states the deferred embed fills the pending annotation
vector via `embed_annotations`. First step is to verify whether any un-embedded annotation
path remains (e.g. annotations created by routes other than `add_annotation`); if none, close
this item as done.
