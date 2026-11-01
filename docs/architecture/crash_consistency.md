# Crash Consistency: write ordering, checkpoints, recovery backstop

**Every multi-step write has a defined crash story: ordering decides what survives, checkpoints bound the re-work, and recovery heals the rest from the source of truth.**

## The coverage model

Two failure classes are distinguished, and they are covered by different mechanisms:

- **Graceful shutdown.** The service's biased `select!` (`run_loop`, `crates/xaga-core/src/embeddings/service.rs`) takes `Shutdown` between loop turns, so work done inline within one turn is atomic with respect to it. Work still queued at shutdown is dropped, not drained.
- **Hard kill / crash.** Nothing inline is guaranteed; a multi-commit sequence can stop between any two commits.

**Recovery is the universal backstop for both:** dropped jobs and half-applied sequences are rebuilt from the source files on the next run.

## Instances

- **Entry `.md` before feed `.atom`** (`commit_via_writer`, `crates/xaga-core/src/persistence/feeds.rs`, which awaits every entry write on its own lane before submitting the `.atom` write on the feed lane). Entry content — the irreplaceable part — is persisted before feed membership. A crash between the two leaves entries not yet in the atom; the next refresh re-adds the membership from the live feed and skips the already-written `.md` files, so no content is lost. The reverse order could lose content.
- **Chunked fan-out with per-chunk checkpoints** (`reconcile_topic_membership`, `crates/xaga-core/src/embeddings/pipeline.rs`). The mechanism: the topic's LanceDB row stores its member-id list (`items`) as the prior-membership base — persisted on the row rather than held in memory so it survives restart — and each reconcile diffs the topic `.md`'s members against that base, touching only the delta. The reconcile is the sole writer of the `topics` (member) and `items` (topic) columns; entry/annotation sync carries them forward untouched on every upsert, so the column lags the file and the diff stays meaningful. Deltas are written in chunks of 1024, and the `items` base is checkpointed after each chunk, so a crash mid-fan-out resumes from the remaining members instead of re-scanning the whole set. Known window: the delete-then-add within a chunk on a hard kill; recovery heals it.
- **Unconditional recovery rebuild, decoupled from unrelated gates.** The reverse-index rebuild runs on every recovery regardless of the checksum/embedding gate — a membership-only change never touches the entry `.md`, so a gated rebuild would skip it. It backstops both a dropped topic event (graceful shutdown before it ran) and a crash mid-fan-out (`crates/xaga-core/src/recovery/`).
- **Annotation color change is delete/create in one atomic write** (`replace_annotation`, `crates/xaga-core/src/persistence/articles.rs`). Color is part of the annotation's identity: a recolor replaces the annotation under a new id, and the delete and the create land in a single atomic frontmatter write (`write_atomic_async`), so no crash point exists where the annotation is deleted but not re-created. The changed id set is the crash story: a stranded recolor is visible to the membership diffs that already run (`annotations_differ` in recovery, the `items` compare in `sync_entry_row`), where a same-id color update would be visible to no gate — the entry checksum excludes annotation data and the id set would be unchanged. The same-id path is closed at the persistence layer (`EntryError::ColorChangeRequiresReplace`). LanceDB side, `sync_annotations` moves the vector and `topics` from the same-checksum row it deletes in the same pass — the vector is a pure function of the checksummed content — so the re-keyed row costs no embedder run.
- **Recovery reconciles the checksum of every row it knows about** — entries, topics, feeds, and annotations alike; the backstop is not universal otherwise. The walk streams an article when its entry embed-gate fires OR any of its annotations drifted: membership (id set), content (the annotation row's stored checksum vs the `.md`), or a pending vector (`annotations_differ`, `crates/xaga-core/src/recovery/embeddings.rs`). All compared against the meta map loaded once before the walk — no per-entry query. The content term is what makes a crash-stranded same-id **note** edit heal at the next startup: the entry checksum excludes annotation content and the id set is unchanged, so no other gate can see it. Every partial state a crash can leave thus differs from the files in a way some gate detects: entry embed-gate, id-set diff, annotation checksum, missing vector, the `items` membership checkpoint, or the unconditional reverse-index rebuild.
- **Local resources before the entry `.md`** (`save_article`, `crates/xaga-core/src/persistence/articles.rs`, via `persistence::images`). Body-image bytes are written to `<library>/images/<id>/<hash>.<ext>` before Markdown containing `../images/<id>/<hash>` URLs. A successfully acquired favicon atomically replaces `<library>/favicons/<normalized-url-hash>` before the article records that shared reference. Failed favicon acquisition or replacement preserves the article's existing reference and cached bytes. A crash before the Markdown write can leave an unreferenced resource; entry-image GC removes per-entry orphans on the next save, while shared favicon GC is a separate library-wide concern. Reading local content does not initiate acquisition. Remote loading of a new or refreshed article reacquires its favicon; read-only instances may read cached resources but cannot replace them. Entry deletion must delete the `.md` before removing its per-entry image directory.
- **Out-of-band changes are a recovery concern.** External edits and migrations are not reflected live; recovery reconciles them on its next run. Eventual sync, not live sync.

### Queued Annotation vector completion

Annotation row creation, deletion, and metadata reconciliation precede vector
completion. A queued embedding snapshot may have been overtaken by a committed
deletion, replacement, or note edit. Vector completion skips absent rows and
rows whose checksum differs from its input, and changes only the vector on a
matching pending row. It must not recreate a deleted Annotation or restore
older metadata. Entry row sync reconciles Annotation content as well as IDs,
so same-ID note edits discovered by recovery have current rows before embedding.

Recovery compares cached Annotation IDs against the Annotations actually present
in the source files. A surviving parent Article does not preserve an orphaned
Annotation row. The existing complete-source-walk guard still withholds deletes
when the library cannot be fully read.

### PDF representation replacement

PDF bytes and their Markdown identity are validated before durable mutation.
One normalized URL has one Markdown identity record: an existing
`entries/{hash12}.md` takes precedence, because it may contain richer feed
metadata and user-authored state. When that Entry is absent, the writer
atomically commits a frontmatter-only `pdfs/{hash12}.md` before atomically
committing `pdfs/{hash12}.pdf`. A crash can leave a harmless companion without
bytes; it cannot leave newly committed PDF bytes without reversible identity.
A failed companion commit prevents PDF replacement, while a failed PDF commit
leaves the complete identity record and previous PDF bytes.

A successful HTML Markdown save removes the URL-associated PDF first and its
companion second, under the same writer lane. A crash between removals can leave
only the companion. Markdown persistence and PDF-resource removal are not one transaction:
a crash before removal can still leave both representations, and local lookup
prefers the PDF. A successful HTML refresh removes those stale PDF resources.
Download staging remains outside the library.

### Article reading state and explicit library upgrades

Reading-state writes run in the entry's library-writer lane and atomically patch
`is_unread` without changing tags, annotations, content, or unrelated timestamps.
Unread is stored as `is_unread: true`; marking read removes the field. Command
handlers enqueue typed `StatePatch` updates after the file mutation. LanceDB's
`is_unread` column is derived; cache state changes do not synthesize tag markers.
Recovery compares the cached boolean with the file independently of the content
checksum, so a crash between the authoritative write and cache update heals.

The CLI performs the explicit 0.9-to-0.10 conversion under the library write lock.
The disposable CLI owns all legacy-format rules. It converts entries sequentially
with sibling temporary files, file fsync, atomic rename and directory fsync,
without using the runtime `LibraryWriter`. Core owns the version manifest and its migration-state writes; the legacy
content conversion rules remain in the CLI. An existing boolean wins over a conflicting legacy `unread` tag;
otherwise the tag determines state.
The field update and removal of the legacy marker share one atomic replacement.
Read state is represented by omission, including normalization of explicit false.
Files already in the new representation require no rewrite. Conversion applies
the existing removal rule for unsupported `images` metadata.

A crash may leave a library partially converted. Rerunning the upgrade is safe:
it preserves converted files and retries cache reconciliation even when no file
needs rewriting. Success requires both file conversion and cache reconciliation;
errors identify unfinished work and cause a nonzero CLI exit status. The app
never performs this format conversion automatically.

Cache reconciliation updates existing Entry rows in bounded batches of at most
1024 file identities, partitioned by the new boolean state. Each update changes
only `is_unread` and removes the legacy `unread` tag; vectors, memberships and
other columns survive. The bound limits SQL expression size and avoids a commit
per file. Missing rows are left for ordinary recovery. The converter retains at
most one article document and one batch of identities, rather than accumulating
the corpus in memory. Failed batches are retried from files on the next invocation.

After the file batches, a final cache pass removes legacy unread markers from
stale rows as well, without changing their stored boolean state. Normal recovery
removes rows whose source is gone.

### Central pin membership

Pin mutations serialize on one library-writer lane and atomically replace
`pins.yaml`. A crash leaves either the previous complete membership or the next
complete membership; entity files are untouched. Committed changes are announced
to the UI without waiting for embedding work. The cache is derived from the store,
with pin-only drift detected independently of content checksums during recovery.
Missing targets never cause removal of a reference.

The explicit CLI conversion holds the library lock and scans legacy Entry, Topic,
and Feed files. For each marker it commits the typed central reference first,
then atomically replaces the entity file without the legacy marker. A crash in
between leaves redundant legacy state, never a lost pin. Retrying merges with
already-transferred references, including unresolved references, and preserves
sequence order and unrelated entity data. It reconciles cached pin state from the
central store even when no markers remain. Runtime parsers never restore pins
from legacy markers. Conversion and cache errors make the CLI exit nonzero.

### Library version transitions

The core manifest API atomically records the pending target before the CLI opens
or changes a cache or converts source files. The last completed version is
retained while conversion is in progress; an initially unversioned library records
null as that source version. Only successful conversion and cache reconciliation
allows core to atomically replace the manifest with the completed target version.
A crash or reported failure leaves the pending marker, and retry resumes the same
target. Normal app startup opens mismatched, absent, incomplete or unreadable
metadata read-only and never alters it. See [library version](library_version.md).

### Article color markings

Article color assignment, recoloring, and removal serialize in the Entry writer
lane. One atomic frontmatter patch writes `color`, or removes it when unmarked,
without modifying annotations, organizational tags, reading state, or body.
The command carries the committed optional color in `StatePatch`: an absent
patch field leaves color unchanged; an explicitly absent value removes it.
Cache sync compares color values and emits `bookmark:changed` for assignment,
removal, and color-to-color changes. Recovery compares file color with cached
color independently of content checksums and repairs crash-stranded changes
without recomputing unchanged content embeddings. Queued vector completion
preserves cached color, reading state, and pin state rather than restoring the
older state snapshot carried by the embedding job.

The offline 0.13 converter commits color and legacy bookmark-marker removal in
the same atomic replacement under the library lock. A valid existing palette
color wins; a legacy marker without color supplies Blue. Retry re-reads converted
files and reconciles their cache state even if no file needs rewriting. It keeps
one document and at most 1024 identities/colors in memory, groups cache updates by
palette color, preserves vectors and membership columns, and only completes the
library version after schema conversion, reconciliation, and index creation.

### Tag definitions and transitions

Tag registry mutations serialize on the library writer's `tags` lane and
atomically replace `tags.yaml`. Article label changes remain in Entry lanes.
Recovery discovers definitions before indexing assignments and detects tag-only
drift independently of content checksums; unchanged vectors survive repair.
Queued vector completion preserves the row's current tag IDs.

The 0.14 CLI upgrade persists definitions before each batch of cache assignments.
A crash can leave definitions ahead of the cache; retry resolves the same names
to the same IDs and reconciles the cache again. It never completes the library
version before definitions and assignments have been reconciled.

Pending rename/merge/delete records retain reserved identities, names, and
file/index checkpoints outside the cache. A pending batch precedes its writes;
its checkpoint follows successful writes. Revision checks refuse stale checkpoint
or finalization requests. Newly discovered transitional labels invalidate relevant
completion/cursors. The [tag storage contract](tags.md#checkpoints-and-retirement)
specifies the records and the evidence required before retirement.
