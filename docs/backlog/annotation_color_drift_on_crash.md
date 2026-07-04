# Stale annotation vector/color after crash during an in-app annotation edit (resolved for color — see bottom)

Rewritten 2026-07-03. The originally recorded concern — annotation embedding never triggered for
semantic search — was verified against the code and refuted: `add_annotation`
(`src-tauri/src/commands/articles.rs`) enqueues `SyncAnnotations` then `EmbedEntry`, and
`embed_entry_vector` fills pending annotation vectors via `embed_annotations`
(`crates/xaga-core/src/embeddings/pipeline.rs`). What that verification surfaced instead is the
narrower defect below.

## The defect

Editing a **pre-existing** annotation in the app (the reader tooltip edits note and color on an
existing id; core `add_annotation` is an upsert by id), then crashing before the enqueued jobs
run, leaves LanceDB stale — and recovery does not detect it:

1. `add_annotation` writes the `.md` (source of truth), then enqueues `SyncAnnotations` +
   `EmbedEntry` on in-memory flume channels (`embeddings/service.rs`); the queue does not
   survive a crash.
2. On restart, recovery's two gates both pass: the entry checksum excludes annotation text
   (`embeddings/row.rs`, `article_embed_text` = description-or-body), and the annotation id set
   is unchanged because the annotation pre-existed (`recovery/embeddings.rs`,
   `annotations_differ`). The article is never streamed; the row keeps the old vector, old
   checksum, and old color.

The crash-window protection in `sync_annotations` (membership written as the final step so the
next recovery diff flags the entry) covers annotation *adds/removes* only — an *edit* leaves the
id set identical and there is nothing inconsistent inside LanceDB for recovery to notice.

## Healing surfaces (asymmetric)

- **Vector**: heals on the next event that carries this article through the pipeline —
  `save_article`, feed refresh, bookmark ingest — via the `EmbedEntry` follow-up's
  per-annotation checksum compare (`embed_annotations`). Persists indefinitely if the entry is
  never written again.
- **Color**: strictly fewer healing events. The save/refresh path (`sync_entry_row`) calls
  `sync_annotations` only when the id set differs, and `embed_annotations` skips color-only
  staleness (color is not in `annotation_checksum`). Heals only when a `SyncAnnotations` job
  runs for this article (some annotation added/edited/removed) or on a full DB reset rebuild.

Frequency of the trigger (edit + crash inside the window) is unknown — no data.

## Fix directions considered, not chosen

- Include per-annotation checksums in the recovery diff — `list_entries_meta` already loads
  every annotation row's checksum; the diff currently reads only the `items` membership.
- Include annotation text in the entry checksum (widens re-embeds of the entry body).
- Add color to `annotation_checksum` or compare color in the recovery/embed gates (fixes the
  color surface only).
- Read-time healing (enqueue `SyncAnnotations` on `read_article`) — heals only opened
  articles; adds one filtered scan + one point-read per open, paid on every open forever.

All four charge the cost to bystanders (every recovery, every embed, or every read) rather
than to the thing that can fail.

## Resolution (2026-07-03): color change is delete/create

The durable description of this design lives in `docs/architecture/crash_consistency.md`
("Annotation color change is delete/create in one atomic write"); the account below is this
item's historical record.

A color change is not an *update* to an annotation — it is delete/create. Color is part of
the annotation's identity: a recolor replaces the annotation under a new id. The delete and
the create land in **one atomic frontmatter write** (`replace_annotation`,
`crates/xaga-core/src/persistence/articles.rs`, via `write_atomic_async`), so no crash point
exists where the annotation is deleted but not re-created. A crash that strands the queued
jobs now leaves a *changed id set*, which is exactly what the existing membership diffs
detect (`annotations_differ` in recovery, the `items` compare in `sync_entry_row`) — the
stranded change heals on the next recovery or sync with no new gates, no new reads. The cost
is attributed to the annotation that changed, not to every read/recovery/embed — and the
embed itself is eliminated: `sync_annotations` moves the vector and `topics` from a
same-checksum row deleted in the same pass to the inserted row (the vector is a pure function
of the checksummed content), so the embed pass checksum-skips the re-keyed row. The lineage
is inferred from content identity rather than carried symbolically; at design scale the
SHA-256 collision rate makes the inference practically guaranteed. A recolor's total cost:
one delete, two row upserts, per-topic column writes — no ONNX run.

Implementation: core `replace_annotation` (atomic write, `created_at` carried — the highlight
is the same selection); `replace_topic_item` (in-place member id swap, position and type
preserved, `crates/xaga-core/src/persistence/topics.rs`); `replace_annotation` command
(topic membership migrated via the LanceDB reverse index, `RemoveMember`/`AddMember` jobs,
`annotation:removed` emitted for the old id, `src-tauri/src/commands/articles.rs`); UI
recolor path routes through `onReplaceAnnotation` with a freshly minted id
(`src/components/content/HtmlReader.tsx`, `useAnnotations.tsx`).

The note-edit remainder (2026-07-03, same day): a crash-stranded **note** edit (same id) was
still invisible to recovery — its two gates (entry checksum, id-set diff) both passed over
it, while the per-annotation checksums recovery already loads went unconsulted. Closed by
extending the recovery walk's predicate: `annotations_differ`
(`crates/xaga-core/src/recovery/embeddings.rs`) now also streams the article when any
annotation row is absent, its stored checksum diverges from the `.md`, or its vector is
pending. Recovery reconciles the checksum of every row it knows about — restoring the
universal-backstop contract of `docs/architecture/crash_consistency.md`.
