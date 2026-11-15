# Establish persisted entity identity authority

## Summary

Make persisted record contents authoritative for identity and treat filenames as
indexes for direct lookup. Formalize the existing storage model, retaining Markdown and Atom formats and
filename indexing without a whole-library migration or version bump. Scope covers
Entry/Article records, PDF Markdown companions, Topics, and locally persisted Feeds.

## Requirements

- Preserve existing local IDs, references, directory layout, formats, and direct
  filename lookup. Do not change Topic storage to Atom or introduce a location
  index. A filename lookup miss does not prove the record is absent elsewhere;
  fallback scanning is outside this story.
- No minor-version increment is required for this work, as explicitly decided
  by the user: it formalizes identity authority in the existing storage model,
  retaining local IDs, references, formats, and filename indexing. Missing IDs
  are repaired incrementally rather than requiring library conversion;
  `xaga:id` uses Atom's existing extension mechanism. This is the authorized
  interpretation of the storage-version rule for this remediation.
- Parse and retain saved identity. A successful lookup must validate that the
  loaded record belongs to the requested identity before returning it, modifying
  it, or using it to update derived data. Never replace a present document ID
  with the requested ID or filename stem.
- When an indexed record lacks local identity, repair that deficiency using the
  indexed identity through the existing serialized writer. Preserve all other
  fields and content. Failed repair must be reported without claiming it was
  saved. Missing identity does not make the indexed identity ambiguous: use that
  identity in memory even when repair cannot be persisted. Read-only access
  must not repair or mutate storage. Discovery uses the filename index to supply
  missing identity under the same rule.
- Reject a saved-ID/filename-ID mismatch as a persisted-record validation
  failure. Preserve the file; do not return or modify it under the requested ID,
  overwrite either identity, retarget references, or create a replacement.
  Recovery treats rejection as unreliable discovery and withholds absence-based
  deletion. Log the file and both IDs without document content. Improved UI
  reporting belongs to the separate backlog story and is not a dependency.
- Apply one shared interpretation across direct reads, discovery, recovery,
  exports, repairs, and app/CLI consumers. Keep authoritative writes ahead of
  derived index updates and preserve writer serialization and read-only behavior.
- Preserve publisher Feed identity independently from local Xaga identity;
  publisher refresh must not redefine local identity. Namespaced `xaga:` fields
  in local `.atom` files hold locally owned information. Store local Feed identity
  in `xaga:id`, retaining the ordinary Atom `id` as publisher reference data.
  Match extension fields by namespace URI and local name, not prefix spelling.
- New Feed files write `xaga:id`. Refresh and every other `.atom` write repair a
  missing `xaga:id` from the filename lookup ID in the same serialized write.
  Preserve existing Xaga-owned fields during publisher refresh. A present ID
  must match the requested identity; never overwrite a mismatch as a repair.
  Read-only reads use the inferred identity without writing. No bulk migration
  is required.
- With implementation, document the Xaga namespace URI, locally owned fields,
  publisher/local ownership distinction, and repair behavior in durable
  architecture documentation. Define one stable namespace URI in the shared
  Atom persistence implementation.

## Remaining work

- Implement shared identity parsing, validation, and lazy repair. Update existing writers so new records retain authoritative local
  identity and ordinary updates preserve it.
- Add regressions for matching, missing, malformed, and conflicting identities;
  repair persistence and failure; read-only refusal of writes; concurrent reads
  and writes; recovery deletion protection; unchanged references; and Feed
  refresh preserving local and publisher identities. Cover namespace-aware
  parsing/serialization, alternate prefixes, and missing-ID repair on refresh
  and other Atom writes. Verify existing error propagation without adding UI presentation in this story.
- Record the authoritative contract and permitted repair paths in durable
  principles/architecture documentation; update the misleading filename-as-ID
  rule and affected comments. Complete the shared verification gate.

## References

- [Error reporting and UI grooming](../backlog/defect__silent-drop-error-reporting.md)
- [Single source of truth](../../docs/principles/single_source_of_truth.md)
- [Crash consistency](../../docs/architecture/crash_consistency.md)
- [Library locations](../../docs/architecture/library_layout.md)
- [Markdown parsing](../../crates/xaga-core/src/md.rs)
- [Markdown discovery](../../crates/xaga-core/src/persistence/walk.rs)
- [Topic persistence](../../crates/xaga-core/src/persistence/topics.rs)
- [Entry persistence](../../crates/xaga-core/src/persistence/entries.rs)
- [PDF persistence](../../crates/xaga-core/src/persistence/pdfs.rs)
- [Feed persistence](../../crates/xaga-core/src/persistence/feeds.rs)
- [Atom conversion](../../crates/xaga-core/src/atom.rs)
- [Recovery](../../crates/xaga-core/src/recovery/embeddings.rs)
