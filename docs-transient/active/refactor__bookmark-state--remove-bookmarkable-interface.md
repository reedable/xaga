# Remove Bookmarkable Interface

## Summary

Remove the unused `Bookmarkable` capability abstraction from the Rust and
TypeScript domain models. Keep bookmark state on the concrete types without
changing behavior.

## Requirements

- Remove the Rust `Bookmarkable` trait, its Entry/Article/Note implementations,
  and its re-export from `types.rs`.
- Remove the TypeScript `Bookmarkable` type while preserving the concrete
  representation present at implementation time. Do not introduce or restore
  bookmark booleans if the color-marking story has already removed them.
- Preserve concrete bookmark fields, serialized payloads, persistence, filtering,
  toolbar actions, and bookmark import/export.
- Do not introduce a reading-state interface or change other capabilities.
- Do not change the meaning of Bookmark or its relationship to Tags.

## Remaining work

- Update the domain declarations and any accessor calls that depend on the
  removed trait; Article's existing delegation must not require retaining it.
- Check for remaining imports, re-exports, and references to `Bookmarkable`.
- Run `cargo fmt --all`, `cargo fmt --all --check`, affected Rust compilation
  checks, and frontend type checking and lint.

## References

- [Bookmark Articles with Colors](../available/enhancement__bookmark-state--color-markings.md)
  — independent color-marking feature; either story can be implemented first.
- `crates/xaga-core/src/domain.rs`
- `crates/xaga-core/src/types.rs`
- `src/domain.ts`
- [Domain-definition investigation](../active/spike__define-tag-collection-bookmark-pin-shortcut-site.md)
