# External-edit last-wins window has no detection

## Summary

The library lock coordinates xaga processes but not external editors. An external edit that lands between a writer lane's read and atomic replacement can be silently overwritten. External changes are otherwise observed only by later reads or startup recovery; read-only instances also retain startup-pinned LanceDB state.

## Requirements

- Detect or visibly handle an external edit that occurs during a read-modify-write operation.
- Preserve per-file writer serialization and atomic file replacement.
- Define how externally changed files update the derived LanceDB mirror and read-only views.
- Do not treat advisory locking as protection from non-participating editors.

## Remaining work

- Compare write-time validation, filesystem observation, and explicit reload approaches, including their race windows and platform behavior.
- Present refuse, merge, and visible-last-wins conflict policies with their data-loss and UX consequences for decision.
- Define reload and recovery behavior for the selected policy before implementation.
- Implement and verify an external edit both outside and inside a writer lane's read-modify-write window.

## References

- `docs/architecture/single_writer_library_lock.md`
- `docs/principles/single_source_of_truth.md`
- `crates/xaga-core/src/library_writer/`
- `crates/xaga-core/src/fs.rs`
