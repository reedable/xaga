# Tag storage

`<library>/tags.yaml` owns Entry tag identities and display names. Article
frontmatter owns readable-name assignments; LanceDB's existing `tags` string-list
column stores resolved UUIDs. Definitions are library-scoped. Feed, Topic, Site,
Note, and Annotation tagging are not covered by this format.

```yaml
tags:
  - id: 46fa4a27-1ddf-448a-a5e6-95691a29c602
    name: Research
```

IDs are canonical UUID v4 strings. Comparison trims surrounding whitespace and
uses Unicode lowercase conversion. There is no case folding, Unicode
normalization, transliteration, or locale-dependent comparison. `Research` and
`RESEARCH` match; `straße` and `strasse` do not. Creation or first-discovered
spelling is retained for display until explicit rename. Empty names are refused;
punctuation-only names are accepted. Duplicate IDs or comparison values are
invalid. Reads never initialize a missing store, which represents no definitions.

Registry reads and serialization are bounded to 12 MiB, matching the pin-store limit. This is a bound on definitions and active transition records, not on
the number of article assignments. Malformed YAML, anchors, unknown fields,
nonregular stores, and invalid records are refused before mutation. Serialization
does not preserve YAML comments or formatting.

## App creation API

The platform exposes `listTags(signal)`, `createTag(name)`, and `onTagCreated`.
Native commands are `list_tags` and `create_tag`; both return tag definitions
with `id` and `name`. Listing reads the current registry, including external
edits, and is allowed in read-only mode. Creation requires the library writer.

Creation atomically looks up or creates a definition in the registry writer lane.
A matching name returns the existing identity and display spelling without
rewriting the store. Active rename/merge aliases return the canonical assignment
identity and name; deleting names are refused. A new definition emits typed
`tag:created` with the committed definition after its atomic write succeeds.
Duplicates and failed writes emit no creation event. Discovery during indexing
uses the same identity lookup but does not emit this command's creation event.

Errors use `tags:invalid-name` for an empty trimmed name, `tags:deleting` for a
name reserved by deletion, and `tags:invalid-store` for registry read/write or
validation failures. Read-only creation uses `library:read-only`.

The Entry assignment picker consumes these APIs; creation alone does not assign
a tag to an Entry. These APIs use the existing 0.14 storage format.

## Entry assignment API

The Article Inspector's Tags view provides searchable toggle chips and creation
with immediate assignment. Assigned chips are filled, carry a check mark, and
expose `aria-pressed`; unassigned chips are outlined. Chip buttons support keyboard
activation. There is no Save action. Read-only inspection shows assignments while
disabling assignment and creation. Annotation inspection does not offer Tags.

`readEntryTags(signal, id)` reads assignments from the Entry file and resolves
them to registry IDs, independently of index readiness. `setEntryTag(id, tagId,
assigned)` adds or removes one identity. Native commands are `read_entry_tags`
and `set_entry_tag`. The writer rereads assignments inside the Entry lane,
resolves aliases, and patches only readable tag labels. Other assignments,
frontmatter, annotations, and body bytes survive. Adding uses the registry's
current assignment name; removing retains the definition. Unknown identities
and assignment to a deleting identity are refused before writing.

After a successful file write, the command carries `EntryTagChange` (`entryId`,
`tagId`, `assigned`) to an Interactive indexing job and the typed
`entry-tags:changed` event. Failed writes emit no assignment event. The index
applies that delta to the existing row without changing content, vectors,
reading/color state, or other assignments. Missing rows remain recovery's
responsibility. The service remembers the latest saved assignment change for each affected
Entry/tag pair and applies it instead of older queued article values, including when such a
snapshot first creates the row. Starting recovery clears ordinary in-memory assignment changes before reading
the source again. Entries affected by a merge retain their markers so older
queued tag IDs still cause the source assignments to be reread. This bookkeeping is not persisted. Entry deletion also
discards its retained deltas.

Inspector instances subscribe before loading assignments and apply successfully saved changes that occurred while the initial read was pending. Navigation discards old target results;
closing the Inspector does not cancel an assignment already requested. External
article edits remain a recovery concern, following the existing reconciliation
contract. This API uses the existing 0.14 storage format.

## Tag query API

`listTagEntries(signal, tagIds, filter)` and `countTagUnread(signal, tagIds)`
call native `list_tag_entries` and `count_tag_unread`. Both are read operations.
Selection resolves through the library's current definitions and aliases before
querying indexed UUIDs. A missing/deleting selected definition produces an empty
result and zero count, preserving the caller's selected set. Query construction
accepts canonical UUID v4 values, not interpolated tag names.

Queries restrict `kind` to Entry and intersect the selected identities. During an
active merge, either participant in the cache satisfies the surviving selection;
each selected group ORs its participant IDs, and groups are ANDed. Ordinary
single-ID selections use `array_has_all(tags, [id])`.
Unread adds `is_unread = true`; Saved adds a nonempty color predicate. Empty
selection omits the tags predicate. Summary reads omit vectors and article
content, and use the existing list ordering. The unread count uses a database
count of the same intersection, without materializing Entries or summing
individual tag counts.

Core index maintenance creates a LabelList index on `tags`, alongside the existing
kind/state bitmap indexes. Existing maintenance updates its indexed fragments.
This changes only a disposable derived index and uses the existing 0.14 format.
The app's shared navigation provider reads definitions from `tags.yaml`, refreshes
them on creation and index notifications, and discards stale count requests when
selection changes. External edits remain visible through the existing recovery
and registry refresh paths. Navigation/membership rules are documented in
[ListView behavior](list_view.md#tag-browsing).

## Mutation and indexing

Runtime registry mutations use one library-writer lane, `tags`, while holding the library write lock. Lookup/creation and discovery reread inside that lane, so concurrent
requests cannot create separate identities for the same comparison value. A
registry mutation atomically replaces the entire YAML file. The CLI's offline
conversion holds the exclusive library lock instead of using runtime lanes.

Source-file Entry values retain readable labels from frontmatter, including an
explicit empty list. Index summaries omit that snapshot; syncing such a summary
preserves existing assignments rather than interpreting omission as removal.
Index producers
resolve those labels through definitions and active transitions. Unknown labels
must be registered before indexing; the indexing consumer cannot invent an ID.
The registry resolver uses a name lookup map. Its cached read-side copy checks
registry file identity before reuse and rebuilds after atomic replacement.

Recovery loads the registry before discovering article labels, registers unknown
names through the library writer, and compares resolved assignments independently
of content checksums. Tag-only differences are repaired without changing existing vectors. Queued Entry vector
completion preserves current cached assignments. Reading/color/pin updates do not
own tag assignments. Index-backed Entry summaries do not substitute indexed UUIDs
for readable labels; article inspection reads the source-file labels.

## Transition records

An optional root `transitions` sequence reserves identities and names while
bounded file/index work is incomplete. Rename, deletion, and merge have runtime
commands and resumable dispatchers.
The user-visible operation rules are in the [organization model](organization_model.md#tags).

A rename updates the definition's `name` and records its old name:

```yaml
transitions:
  - type: rename
    revision: 0
    id: 46fa4a27-1ddf-448a-a5e6-95691a29c602
    old_name: Research
    files:
      complete: false
```

Both names resolve to the unchanged ID. A case-only rename uses the same record.
A rename has no index-conversion checkpoint because assignments retain their IDs.

### Runtime rename

`renameTag(id, name)` invokes native `rename_tag`, requiring the library writer.
The serialized registry operation checks names and transition ownership, commits
the new name and alias, and starts bounded file work. It returns the committed
`TagSnapshot` (`tags`, `renaming`, `deleting`, `merging`). `readTagStatus(signal)` / `read_tag_status`
read that same shape without initializing a store. `tags:changed` carries typed
snapshots after rename acceptance and completion; checkpoints do not emit
unchanged definition snapshots. `tags:collision`, `tags:invalid-name`, and
`tags:busy` support inline dialog errors. Stale checkpoint revisions retry
internally; they cannot advance or retire newer progress.

Sidebar tag rows and responsive header options share one actions menu and
prefilled rename dialog. Actions appear on hover/focus and remain available on
touch devices. Sidebar controls and the responsive menu provide Tab access to
actions. Enter/form submission renames; Cancel/Escape dismiss without mutation.
Dismissal restores focus to the invoking control. Read-only mode disables Rename;
indexed file work shows Renaming…. An unfinished transition blocks another
rename on that identity.
Inspector chips remain assignment controls. Definition events refresh names in
navigation and chips without changing selected IDs or the current list.

Queued Entry upserts carry resolved UUID assignments alongside their Article
snapshot. Native producers obtain those IDs through the registry writer lane,
which reads current source assignments before discovery/resolution. Recovery
carries its resolved IDs to both incremental jobs and fresh-cache bulk inserts;
when discovery is needed, it also uses that writer operation, which rereads the current source assignments. This
prevents a stale source snapshot from recreating an old definition after alias
retirement, and lets queued index jobs finish without depending on retired names.

The dispatcher allows four active tag workers. Each queries the existing LanceDB
read table for Entry IDs carrying the tag UUID, ordered by ID in batches of at
most 1,024. It reads and rewrites only those candidate files. Index false positives
are harmless: each Entry lane rereads current labels and changes only the old
spelling. Missing/deleted Entries are skipped. Other labels, metadata,
annotations, and body bytes are preserved; rename does not rewrite indexed IDs.

Before file writes, the batch is persisted as `files.pending` with the prior
`files.after` cursor. After success, the cursor advances to the batch's last ID.
Restart replays pending IDs idempotently and resumes the indexed scan. Discovery
of old names invalidates the revision/cursor as before. No directory position is
used as an ordered cursor. If no cache exists, source cleanup remains recovery's
responsibility.

When indexed candidates are exhausted and all pending writes are committed, the
worker checkpoints completion and retires the old-name alias immediately. The
Renaming… status clears and another rename is available in the same session.
There is no subsequent verification or cleanup phase for rename.

The index is a candidate source, not an assignment authority. If it missed an
article, that file retains its old spelling. After retirement, ordinary recovery
or discovery handles that spelling as a newly encountered name under the usual
case-insensitive comparison rules. It can create a separate tag identity instead
of attaching the article to the renamed tag. Case-only spellings still resolve
to the existing identity. This is an accepted stale-index consequence; there is
no implicit merge or retained chain of old names. Merge confirmation provides explicit consolidation.

Recovery resumes unfinished indexed batches before discovering article labels.
It does not perform rename-specific full-file rewriting or alias verification.
Missing caches provide no indexed candidates beyond any persisted pending batch;
remaining old labels follow the same discovery rule. Repeated renames retain the
same tag UUID, and pending batches remain crash-safe until indexed work finishes.

Startup resumes indexed work while holding the existing library write lock. Shutdown
stops submissions, drains in-flight writes, waits for workers, and leaves
unfinished checkpoints durable. Background failures use the main window’s existing error-notification handler. Bounds and measurements are in
[index-first rename scaling](design_scale.md#tag-rename-scaling).

### Runtime deletion

`deleteTag(id)` invokes native `delete_tag`, requiring the library writer. It
commits a deletion reservation and returns `TagSnapshot`; `deleting` lists the
reserved identities. Acceptance and retirement emit typed `tags:changed`
snapshots through the same notification path as rename. Checkpoints do not emit
unchanged snapshots. Rename and deletion refuse participants in any unfinished
transition. Unknown IDs are refused; read-only deletion uses `library:read-only`.

Delete appears beside Rename in both tag actions menus. Both action labels include
the current tag name, including while renaming or deleting. One confirmation names
the tag and says its article assignments will be removed while articles remain
intact. Cancel/Escape performs no mutation. Read-only mode disables Delete;
Deleting… identifies unfinished work and both actions are disabled for that tag.
Inspector chips disable assignments involving deleting tags; removing an ordinary
chip assignment still retains its definition.

After committing the reservation, the command enqueues typed `DeleteTag` work at
Interactive priority. Each service turn processes at most 1,024 indexed Entry IDs,
ordered by ID, plus persisted pending work. Both file and index pending batches
are durable before mutation. Up to eight Entry-lane file patches remove matching
labels without replacing other metadata or body bytes. Missing files are skipped;
index false positives do not remove unrelated labels. A single partial-column
cache update removes the UUID from that Entry batch, preserving other assignments,
vectors, content, state, and pins. Only then do both checkpoints advance. Further
batches retain the arriving priority and yield between service turns. Shutdown
may leave work unfinished; the saved transition records the work to retry.

Discovery that checks the current source file and finds a deleting label records its Entry ID as pending file
work and invalidates progress, including when the cache missed that file. The
pending list retains the 1,024-ID bound; if full, discovery refuses the operation
with `tags:busy` rather than dropping cleanup work or overflowing the checkpoint.
Retries and recovery can resolve the source after the batch drains. Older queued
article/assignment jobs also patch a currently deleting source label before
removing its cached ID, so they cannot hide a file from indexed cleanup. The registry resolver checks the current registry file’s identity and filters retired IDs from queued snapshots
and assignment deltas; it does not map them to a new identity with the same name.

Recovery completes unfinished deletion batches before discovering article labels,
using the same cleanup routine and cache writer. Pending files are replayed even
when the cache is empty or rebuilt. Failures preserve reservations and pending
work, surface through the existing error sink, and prevent recovery from proceeding
past unfinished cleanup.

Deletion finishes when indexed candidates and pending work are exhausted and both
checkpoints are complete. It retires the definition and name reservation immediately;
there is no full-source scan, verification pass, or retained tombstone. A file
missed by the index may keep its label. After retirement, recovery or ordinary
discovery can create a new identity from that label, as with missed rename labels.
This accepted stale-index consequence differs from interrupted deletion, whose
active reservation prevents recreation and whose work resumes during recovery.

Deleting a selected tag removes its ID from the current navigation selection
without replacing the displayed list or reading pane selection. Definition events are
retained even if acceptance and completion arrive before a UI render. The next
explicit navigation, filter change, or refresh queries the remaining selection;
an empty selection queries all Entries under the existing filters. Bounds and
scale assumptions are in [deletion scaling](design_scale.md#tag-deletion-scaling). An in-flight
query for the superseded selection cannot replace the retained list.

### Runtime merge

A rename collision offers a second, explicit confirmation naming both tags.
Cancel leaves both definitions and assignments unchanged. `mergeTags(sourceId,
destinationId)` invokes trusted native `merge_tags`, requiring the library write lock. Confirmation is disabled while either participant has an active transition;
the registry also rejects racing conflicts with `tags:busy`.

The command submits a typed merge request to the Interactive indexing queue. That
single consumer counts both indexed Entry populations and persists the survivor
choice before another cache job runs. It returns the committed snapshot and queues
bounded conversion at the same priority. A missing service refuses acceptance.
`merging` contains source/destination IDs, survivor ID, and destination name.
Snapshot `tags` exposes only the survivor under the destination name, while both
original definitions remain reserved in storage. Acceptance and retirement emit
`tags:changed`; checkpoints do not emit redundant snapshots.

Merge and deletion share `tags/conversion.rs`. Each step selects up to 1,024
candidate IDs, persists pending work, and runs up to eight atomic Entry-lane
patches. Merge patches either participant label to the destination name and
collapses duplicate participant labels. It then changes only the cache's `tags`
column on rows carrying the losing ID. Rows already carrying the survivor remove
the loser; remaining rows replace it with the survivor in a second idempotent
update. Unrelated labels, body, metadata, vectors, read/color state, and pins are
preserved. Independently persisted file/index pending lists are replayed without
dropping newly discovered file work, including after a cache reset.

The indexing session remembers participating IDs so delayed pre-merge Entry
snapshots or assignment deltas re-read that Entry's current source assignments.
This prevents an obsolete losing-ID assignment from overriding a later removal
of the survivor. This is session bookkeeping, not a persisted name alias or
tombstone. Recovery remembers active participants before resuming conversion and
then discovers source labels under the resulting registry.

Sidebar/header status shows Merging… on the surviving tag. Selection IDs follow
committed merges, deduplicate, and retain the reading pane and current article list.
Subsequent explicit navigation or refresh includes assignments under either
participating tag ID for the current merged selection. Inspector assignments follow the survivor and refresh from source on
merge events and subsequent assignment events, avoiding late losing-ID deltas.
Bounds and measured batch costs are in [merge scaling](design_scale.md#tag-merge-scaling).

### Merge and deletion records

A merge records `type: merge`, `source_id`, `destination_id`, `survivor_id`,
`destination_name`, `revision`, `files`, and `index`. Both definitions remain
until completion, reserving both names. Both names resolve to the surviving ID;
new assignments use the destination display name. The survivor is selected once
from indexed assignment populations: larger population wins; ties retain the
destination ID. Retries use that persisted selection. Finalization removes the
losing definition and sets the survivor's display name to `destination_name`.

Merge cleanup uses indexed Entry assignments for either participating ID and
persisted pending work. Recovery resumes unfinished batches before discovering
article labels, retaining both name reservations and the persisted survivor
choice. Completion retires the losing definition and transitional name mapping
without a full-source scan, verification pass, or retained tombstone. A missed
file may retain its non-destination label; after retirement, recovery or discovery
can create a new tag identity from it. This accepted stale-index consequence
differs from interruption, whose active reservations prevent recreation.

A deletion records `type: delete`, `id`, `revision`, `files`, and `index`.
Assignment by that identity is refused. File-label discovery recognizes the
reserved name without creating another definition; index reconstruction omits
that deleting assignment. Finalization removes the definition.

A tag can participate in only one transition. Unrelated definitions and
transitions can still change. Names reserved by a rename cannot be taken by
another definition. Merge/delete cannot begin on a participant of an active
rename, merge, or deletion.

## Checkpoints and retirement

Each `files` or `index` checkpoint contains optional `after`, a bounded `pending`
Entry-ID list, and `complete`. `pending` contains at most 1,024 unique safe Entry
IDs. A complete checkpoint cannot retain pending work. `after` is the last
committed position in an ordered Entry-ID scan; it is not an operating-system
directory iterator position.

The producing dispatcher must persist its pending batch before mutation, execute
idempotent work, and advance the cursor only after the batch commits. A restart
retries the persisted batch. Checkpoint writes compare the transition's revision
and increment it, refusing stale workers. Newly observed old/non-destination or
deleting labels invalidate scan completion and cursors for the affected file/index
phases, preserve pending work, and increment the revision. This prevents a worker with an older snapshot
from retiring a name before newly discovered work is finished.

File read–modify–writes belong inside the existing Entry writer lane and use
validated atomic frontmatter patches. Registry replacement alone does not
serialize article changes. File work precedes its checkpoint; cache conversion
precedes its index checkpoint. Rename file completion covers its indexed candidate
batches. Merge and deletion file completion cover indexed candidates and persisted
pending work; merge candidates include either participating ID. Read or write failures in an
indexed rename batch retain the transition and pending work for retry.

Rename finalization requires file completion. Merge and deletion require both
file and index completion. The caller supplies completion evidence through those
checkpoints; storage rejects incomplete or stale finalization. Transition records
live outside LanceDB so a cache rebuild cannot discard their reservations or
progress. Retired names can subsequently create new identities.

## Library upgrade

The explicit 0.14 CLI upgrade initializes definitions from existing article labels
in discovery order and replaces existing Entry cache assignments with resolved
IDs in batches of at most 1,024 file identities. Each batch uses a partial-column
merge keyed by `id` and `kind`, with inserts and deletes disabled; different tag
sets do not create separate commits. Definitions are committed before the
corresponding cache batch. The converter retains one source document and a
batch of names/identities; it does not retain the article corpus.

Conversion preserves article files already using the 0.13 representation,
including organizational tags named `unread`, `bookmark`, or `pinned`. Legacy marker
conversion follows the source version: unread markers before 0.10, pin markers
before 0.11, and bookmark markers before 0.13. Unversioned libraries use the
existing CLI legacy-conversion rules.

Failure retains the pending manifest version. Retrying reuses committed IDs and
reconciles cache batches even when files do not need rewriting. Only successful
conversion, reconciliation, and required index creation completes the version.

The converter reuses core cache maintenance at startup, every 100 batches, and
before completing the manifest. Core compacts and prunes fragments, ensures
scalar indexes, and brings existing indexes up to date. The converter then checks
its direct table handle out to the latest version so it cannot retain a version
that maintenance pruned. Maintenance failure leaves the upgrade pending.

Measured scaling and extrapolation assumptions are recorded in
[Design scale](design_scale.md#tag-migration-scaling).
