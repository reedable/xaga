# Persist tags on annotations

## Summary

Add direct annotation tag storage and assignment APIs using the existing library
tag registry. Assignment controls belong to the separate Inspector story.

## Requirements

- Store readable tag names in each annotation frontmatter record's optional
  `tags` sequence. Missing means untagged; omit the field after the last removal.
  Assignments affect only the targeted annotation, not its article or siblings.
- Use existing registry identity, name comparison, alias, and transition rules.
  Reject unknown tag IDs and assignment to deleting tags before writing. Removing
  an assignment retains the tag definition.
- Read assignments from the article file, independently of index readiness.
  Identify targets by parent Entry ID and Annotation ID. Reject missing or
  malformed targets without writing, falling back to article tags, or recreating
  annotations.
- Serialize assignment read–modify–write in the parent Entry's writer lane under
  the library write lock. Atomically patch only the requested assignment;
  preserve unrelated frontmatter and body bytes.
- Existing annotation writers must preserve the latest saved assignments through
  comment edits, recoloring/replacement, and article refresh. Recoloring transfers
  tags to the replacement ID. Concurrent writes in one article must not lose tags.
- Expose native and platform read/toggle APIs. Successful writes emit typed changes
  containing parent ID, annotation ID, tag ID, and assigned state; failed writes
  emit none. Reads work read-only; mutations require the library writer.
- Increment the minor version while major is 0 because this changes the storage
  model. Extend the explicit library upgrade, including unfinished tag transitions,
  preserving data, pending-target checks, exclusive locking, and interruption/retry
  behavior. Older incompatible applications must remain read-only.

## Remaining work

- Extend annotation domain/frontmatter adapters and add assignment persistence,
  writer operations, native commands, platform adapters, and capture support.
  Reuse existing registry resolution and tag mutation logic.
- Update existing annotation writers to preserve source assignments rather than
  overwrite them with stale request data.
- Implement the version increment and upgrade handling.
- Add regression tests for missing/empty tags, last removal, invalid targets,
  read-only and write failures, concurrent same-file mutations, stale comment
  saves, replacement, article refresh, and interrupted upgrades with pending tag
  transitions. Run relevant Rust/frontend checks and workspace formatting checks.
- Update tag storage, organization, source-of-truth, crash-consistency, and upgrade
  documentation with the format, assignment APIs/events, and permitted writers.

## References

- [Tag storage](../../docs/architecture/tags.md)
- [Organization model](../../docs/architecture/organization_model.md)
- [Library upgrade](../../docs/library_upgrade.md)
- [Annotation domain](../../crates/xaga-core/src/domain.rs)
- [Frontmatter adapters](../../crates/xaga-core/src/md.rs)
- [Article persistence](../../crates/xaga-core/src/persistence/articles.rs)
- [Library writer](../../crates/xaga-core/src/library_writer/mod.rs)
- [Native annotation commands](../../src-tauri/src/commands/articles.rs)
- [Platform interface](../../src/platform/Platform.ts)
