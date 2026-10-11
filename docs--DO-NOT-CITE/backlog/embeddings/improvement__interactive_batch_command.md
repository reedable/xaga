# Batch rapid Interactive-tier updates

Large feed refreshes and bulk state changes enqueue per-item Interactive work.
Reduce their commit count by batching ingestion and state updates. Measure queue
drain time, backend-to-UI latency, and compaction pauses under sustained load.

## Paths to review

- `refresh_all_feeds`, OPML `import_feeds`, and `add_feed`: one `UpsertEntry`
  per new entry.
- Mark all read: one IPC call and `.md` rewrite per entry, followed by a
  `SyncState` whose delete-then-add row update costs approximately two commits.
- Bulk bookmark and annotation changes: per-item `SyncState` work.

## Batch design

- Ingestion: a batched Interactive job using `db.add_batch` for new entries,
  followed by per-entry `EmbedEntry` jobs on Regular. Define the job payload
  and chunk size.
- State updates: a batched command that updates file tags and batches row
  changes. Preserve per-entry unread/list events and the established writer rules.
- Keep visibility updates on Interactive and vector computation on Regular.
  Batching must retain periodic compaction for sustained work from other sources.
- Preserve backend-authoritative UI updates while measuring latency; do not
  mask backend delays with optimistic presentation changes as part of this work.

## Verification

Exercise a large refresh and mark-all-read burst, then perform an individual
read-state change while the batch is in flight. Measure commit count, list-read
latency, event delivery, queue fairness, and compaction pauses. Ensure rows and
file metadata agree after completion and recovery.
