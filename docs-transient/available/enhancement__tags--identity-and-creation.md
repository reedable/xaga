# Tag Identity and Creation

## Summary

Let users create library-scoped tags for Entry organization through the storage
registry’s stable-identity and name-resolution APIs.

## Requirements

- Use the [Tag storage](../../docs/architecture/tags.md)
  contract and existing 0.14 format.
- Expose library-scoped tag creation using the registry's atomic lookup/creation
  operation. Names with equal comparison values return the existing identity without
  changing display spelling; invalid names report a validation error.
- Preserve library isolation and refuse creation in read-only mode.
- Provide the app-facing creation operation and carry committed changes to
  consumers. The complete creation/assignment picker UI belongs to Entry
  assignment, not this story.

## Remaining work

- Expose registry lookup/creation through writer operations and the platform
  creation API; carry committed changes to consumers. Connect the picker in the
  Entry-assignment story.
- Verify creation, duplicates, external edits, restart, concurrent writes,
  read-only access, and library isolation through the app-facing API.
- Run affected Rust/frontend checks and update durable domain/storage docs.

## References

- [Tag storage](../../docs/architecture/tags.md).
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
