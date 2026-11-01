# Tag Storage Foundation

## Summary

Implement the tag storage needed by Xaga's six tag stories in one 0.13 → 0.14
library upgrade. Cover definitions, readable article assignments, indexed IDs,
and persisted rename, merge, and deletion state. The dependent stories implement
the app commands, UI, and execution of library-wide transitions.

## Requirements

- Use the [organization model](../../docs/architecture/organization_model.md#tags)
  for tag identity, naming, and transition behavior. Tags organize Entries in the
  selected library; this story adds no other tagging scope.
- Store definitions as a YAML root `tags` sequence with UUID v4 `id` and display
  `name`. Article frontmatter retains readable names; LanceDB assignments contain
  IDs. Do not persist a second display-name authority in the index.
- Compare trimmed Unicode-lowercase names without case folding or normalization.
  `Research` and `RESEARCH` match; `straße` and `strasse` remain distinct. Preserve
  creation/first-discovered display spelling until explicit rename. Reject empty
  names and accept punctuation-only names.
- Make definition lookup/creation atomic, including creation racing with recovery
  discovery. Use the existing library write right and writer serialization;
  read-only access must not create or modify definitions.
- Persist rename state: unchanged tag ID, old/new names, and file-rewrite
  progress. Both names resolve to the same ID until completion.
- Persist merge state: participating IDs/names, destination display name, chosen
  surviving ID, and file/index conversion progress. Use the organization model's
  population/tie rule; retries must not select the survivor again.
- Persist deletion state: deleting identity/name and assignment-cleanup progress.
  Refuse new assignments and retain the deleting name until cleanup completes.
- Preserve transition records through registry mutations. Apply the documented
  operation guards and name reservations. Only completed conversion permits
  retirement of old names or losing/deleted definitions. Store the state needed
  for retry outside the disposable cache.
- Preserve labels through domain conversion. All Entry indexing paths must resolve
  them through current definitions/transitions without clearing assignments or
  restoring stale ones. Tag-only drift must be repairable without re-embedding
  unchanged content.
- Load transitions before recovery discovers labels. Reserved labels resolve to
  their transition rather than creating another definition; deleting labels remain
  pending cleanup. Reconstruct indexed assignments from article files and the
  registry after cache loss.
- Increment the application minor version to 0.14.0-dev and extend the existing
  explicit CLI upgrade. Initialize definitions from existing article labels and
  reconcile indexed IDs. Reuse the manifest's pending/completed protocol; errors
  or interruption must leave a retryable upgrade, without changing unrelated data.
- Include storage support for all six dependent stories before completing this
  story. Their planned functionality must use the 0.14 representation without
  another library upgrade.

## Remaining work

- Add registry parsing, validation, serialization, lookup/creation, and transition
  storage operations in core. Integrate registry writes with `LibraryWriter`;
  reuse the pin store's atomic persistence mechanics.
- Carry article labels through the existing domain conversion and resolve IDs
  in Entry row construction. Update sync, queued embedding, and both recovery
  paths so each preserves current assignments and unrelated row fields.
- Add unknown-label discovery and tag-assignment drift detection to recovery;
  its current content/read/color gates do not detect tag-only changes.
- Extend CLI conversion using its existing bounded-batch approach. Preserve
  previously created IDs on retry and reconcile cache assignments even when
  article files need no rewrite. Update versions through the version script.
- Verify persisted rename/merge/delete fixtures across restart, registry mutation,
  and cache rebuild. Cover mixed names, losing IDs, reserved labels encountered
  externally, permitted/refused operations, and checkpoint/finalization ordering.
  Test storage support here; end-to-end transition execution belongs to the
  dependent stories.
- Verify duplicate creation/discovery, library isolation, read-only refusal,
  concurrent writes, interrupted migration, and preservation of article content,
  annotations, reading/color state, pins, and unrelated metadata.
- Measure migration time, peak memory, and cache commits at the multi-million-entry
  design target. Verify resulting assignment IDs/counts and retry results. Retain
  at most a batch of pending article conversion work; measure registry costs
  separately from corpus size. Use the existing 1,024-item migration batch as the
  starting precedent and report any limit that the measurements do not establish.
- Run affected Rust/frontend checks and production build. Document the concrete
  registry/transition format, write/checkpoint ordering, and recovery behavior;
  update the upgrade guide and source-of-truth assignments.

## References

- [Organization model](../../docs/architecture/organization_model.md)
- [Library version metadata](../../docs/architecture/library_version.md)
- [Library upgrade guide](../../docs/library_upgrade.md)
- [Crash consistency](../../docs/architecture/crash_consistency.md)
- [Design scale](../../docs/architecture/design_scale.md)
- [Layer ownership](../../docs/architecture/layer_ownership.md)
- `crates/xaga-core/src/pins.rs`
- `crates/xaga-core/src/library_writer/mod.rs`
- `crates/xaga-core/src/md.rs`
- `crates/xaga-core/src/embeddings/row.rs`
- `crates/xaga-core/src/embeddings/pipeline.rs`
- `crates/xaga-core/src/recovery/embeddings.rs`
- `crates/xaga-cli/src/cmd/library_upgrade.rs`
- [Tag Identity and Creation](../available/enhancement__tags--identity-and-creation.md)
- [Entry Tag Assignment](../available/enhancement__tags--entry-assignment.md)
- [Browse Entries by Tags](../available/enhancement__tags--browse-entries.md)
- [Rename Tags](../available/enhancement__tags--rename.md)
- [Merge Tags](../available/enhancement__tags--merge.md)
- [Delete Tags](../available/enhancement__tags--delete.md)
