# Tag storage

`<library>/tags.yaml` owns Entry tag identities and display names. Article
frontmatter owns readable-name assignments; LanceDB's existing `tags` string-list
column stores resolved UUIDs. Definitions are library-scoped. Feed, Topic, Site,
Note, and Annotation tagging are outside this contract.

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

Registry reads and serialization are bounded to 12 MiB, following the pin-store
precedent. This is a bound on definitions and active transition records, not on
the number of article assignments. Malformed YAML, anchors, unknown fields,
nonregular stores, and invalid records are refused before mutation. Serialization
does not preserve YAML comments or formatting.

## Mutation and indexing

Runtime registry mutations use one library-writer lane, `tags`, under the library
write right. Lookup/creation and discovery reread inside that lane, so concurrent
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
of content checksums. Tag-only drift reuses unchanged vectors. Queued Entry vector
completion preserves current cached assignments. Reading/color/pin updates do not
own tag assignments. Index-backed Entry summaries do not substitute indexed UUIDs
for readable labels; article inspection reads the source-file labels.

## Transition records

An optional root `transitions` sequence reserves identities and names while
bounded file/index work is incomplete. Storage supports these records; commands
and dispatchers starting and executing transitions are separate functionality.
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

A merge records `type: merge`, `source_id`, `destination_id`, `survivor_id`,
`destination_name`, `revision`, `files`, and `index`. Both definitions remain
until completion, reserving both names. Both names resolve to the surviving ID;
new assignments use the destination display name. The survivor is selected once
from indexed assignment populations: larger population wins; ties retain the
destination ID. Retries use that persisted selection. Finalization removes the
losing definition and sets the survivor's display name to `destination_name`.

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
from retiring a name past newly discovered work.

File read–modify–writes belong inside the existing Entry writer lane and use
validated atomic frontmatter patches. Registry replacement alone does not
serialize article changes. File work precedes its checkpoint; cache conversion
precedes its index checkpoint. A final successful source scan must account for
newly discovered labels before declaring completion. Read or write errors cannot
be treated as scan completion.

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
