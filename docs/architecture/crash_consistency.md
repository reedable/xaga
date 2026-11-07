# Crash Consistency: write ordering, checkpoints, and recovery

**For every multi-step write, specify what can remain after interruption, which
saved checkpoints allow retry, and how recovery detects and repairs unfinished
work from the source files.**

## The coverage model

- **Graceful shutdown:** biased `select!` in the embedding service’s `run_loop`
  accepts `Shutdown` between processing iterations. Inline work within an
  iteration completes before that handling; queued work is dropped rather than
  drained (`crates/xaga-core/src/embeddings/service.rs`).
- **Hard kill or crash:** execution can stop between any two commits, including
  inside an iteration. Cooperative shutdown handling does not protect it.

Recovery rebuilds dropped or partially applied derived work from source files on
the next run.

## Instances

- **Entry `.md` before feed `.atom`:** `commit_via_writer` waits for each Entry’s
  lane before writing the feed lane (`crates/xaga-core/src/persistence/feeds.rs`).
  A crash between them leaves saved content not yet listed in the feed. The next
  refresh restores membership from the live feed and skips existing Entry files.
  Reversing the order could leave membership whose content was never saved.
- **Topic membership updates in batches:** `reconcile_topic_membership` compares
  the Topic file with its LanceDB `items` member-ID list, which saves the previous
  processed membership across restarts. Only this operation writes the member
  `topics` and Topic `items` columns; Entry and Annotation upserts preserve them.
  Changes are applied in chunks of 1,024, saving the updated `items` checkpoint
  after each chunk. Retry handles remaining differences rather than restarting
  the whole set. A hard kill between a chunk’s deletions and additions can leave
  partial database state, which recovery repairs
  (`crates/xaga-core/src/embeddings/pipeline.rs`).
- **Rebuild the reverse index on every recovery:** Topic membership changes do
  not modify Entry Markdown or its content checksum. The reverse index must be
  rebuilt regardless of whether Entry content needs embedding. This repairs both
  dropped Topic events and interrupted batches (`crates/xaga-core/src/recovery/`).
- **Annotation color replacement:** `replace_annotation` assigns a new ID when
  color changes. One atomic frontmatter write removes the old Annotation and adds
  its replacement, so a crash cannot leave only the removal. The changed ID set
  lets existing comparisons detect unfinished database work (`annotations_differ`
  and the `items` check in `sync_entry_row`). Keeping the same ID would evade
  those checks because the Entry checksum excludes Annotation data.
  `EntryError::ColorChangeRequiresReplace` refuses that path. `sync_annotations`
  transfers the vector and `topics` from the old same-checksum row, avoiding
  unnecessary embedding (`crates/xaga-core/src/persistence/articles.rs`).
- **Compare all cached entity checksums:** recovery covers Entries, Topics,
  Feeds, and Annotations. It processes an Article if its Entry needs embedding
  or an Annotation differs by ID, checksum, or pending vector. Comparisons use
  one metadata map loaded before the file walk, not a query per Entry
  (`crates/xaga-core/src/recovery/embeddings.rs`). Annotation checksum comparison
  detects comment edits that retain the ID and do not affect the Entry checksum.
  The checks cover content, ID changes, missing vectors, and membership
  checkpoints; the unconditional reverse-index rebuild covers Topic changes.
- **Local resources before Entry Markdown:** body images are saved to
  `<library>/images/<id>/<hash>.<ext>` before Markdown referring to
  `../images/<id>/<hash>`. A fetched favicon replaces
  `<library>/favicons/<normalized-url-hash>` atomically before its reference is
  saved. Failed acquisition or replacement preserves the previous reference and
  bytes. A crash before Markdown can leave unused resources; per-entry image
  cleanup runs on the next save, while shared favicon cleanup is a separate
  library-wide operation. Local reads do not fetch resources. Remote article
  loads reacquire favicons; read-only instances cannot replace them. Delete the
  Entry `.md` before its image directory (`save_article` and `persistence::images`).
- **External changes:** startup recovery reconciles external edits and migrations
  from files. They are not synchronized live.

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
Annotation row. Recovery still withholds deletes when the library cannot be fully read.

### PDF representation replacement

PDF bytes and their Markdown identity are validated before durable mutation.
One normalized URL has one Markdown identity record: an existing
`entries/{hash12}.md` takes precedence, because it may contain richer feed
metadata and user-authored state. When that Entry is absent, the writer
atomically commits a frontmatter-only `pdfs/{hash12}.md` before atomically
committing `pdfs/{hash12}.pdf`. A crash can leave a harmless companion without
bytes; it cannot leave newly committed PDF bytes without a Markdown record identifying their source URL.
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
`is_unread` column is derived; cache state changes do not create tag markers.
Recovery compares the cached boolean with the file independently of the content
checksum, so recovery repairs a crash between the file write and database update.

The CLI performs the explicit 0.9-to-0.10 conversion under the library write lock.
The CLI conversion code owns all legacy-format rules. It converts entries sequentially
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
with pin-state differences detected independently of content checksums during recovery.
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
color independently of content checksums and repairs changes not yet copied to the database when a crash occurred
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
Recovery discovers definitions before indexing assignments and detects tag assignment differences independently of content checksums; unchanged vectors survive repair.
Queued vector completion preserves the row's current tag IDs.

The 0.14 CLI upgrade persists definitions before each batch of cache assignments.
A crash can leave definitions ahead of the cache; retry resolves the same names
to the same IDs and reconciles the cache again. It never completes the library
version before definitions and assignments have been reconciled.

Pending rename/merge/delete records retain reserved identities, names, and
file/index checkpoints outside the cache. A pending batch precedes its writes;
its checkpoint follows successful writes. Revision checks refuse stale checkpoint
or finalization requests. Discovering more labels involved in an active operation invalidates the affected
completion checks or query cursors. The [tag storage contract](tags.md#checkpoints-and-retirement)
specifies the records and the evidence required before retirement.

Tag deletion commits its reservation before submitting indexed cleanup. Each batch
persists both pending lists, removes file labels, then removes cached IDs before
advancing progress. A failure retains the batch for replay; recovery resumes it
before label discovery, including pending files when the cache has been rebuilt.
Completion releases the name without a source verification pass. Missed source
labels can then create a new identity; an active deletion reservation prevents
that recreation. Queued snapshots discard retired IDs and remove labels belonging to an active deletion before dropping the affected rows
from its candidate set. See [runtime deletion](tags.md#runtime-deletion).
