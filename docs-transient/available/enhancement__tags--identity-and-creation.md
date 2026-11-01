# Tag Identity and Creation

## Summary

Let users create library-scoped tags for Entry organization through the storage
foundation’s stable-identity and name-resolution APIs.

## Requirements

- Use the [Tag Storage Foundation](../active/enhancement__tags--storage-foundation.md)
  contract and its shared 0.14 library upgrade; do not introduce a separate
  storage format for this functionality.
- Expose library-scoped tag creation using the foundation's atomic lookup/creation
  operation. Names with equal comparison values return the existing identity without
  changing display spelling; invalid names report a validation error.
- Preserve library isolation and refuse creation in read-only mode.
- Provide the app-facing creation operation and carry committed changes to
  consumers. The complete creation/assignment picker UI belongs to Entry
  assignment, not this story.

## Remaining work

- Complete the Tag Storage Foundation first.
- Expose foundation lookup/creation through writer operations and the platform
  creation API; carry committed changes to consumers. Connect the picker in the
  Entry-assignment story.
- Verify creation, duplicates, external edits, restart, concurrent writes,
  read-only access, and library isolation through the app-facing API.
- Run affected Rust/frontend checks and update durable domain/storage docs.

## References

- [Tag Storage Foundation](../active/enhancement__tags--storage-foundation.md) — prerequisite.
- [Organization model](../../docs/architecture/organization_model.md)

- [Pin-store contract](../../docs/architecture/pins.md).
- [Entry Tag Assignment](enhancement__tags--entry-assignment.md)
- [Rename Tags](enhancement__tags--rename.md)
- `crates/xaga-core/src/md.rs`
- `crates/xaga-core/src/embeddings/row.rs`
- `crates/xaga-core/src/library_writer/mod.rs`
- `src/domain.ts`
- `src/platform/Platform.ts`
- `docs/architecture/design_scale.md`
