# Centralize Entry locations and scan all Entry directories

## Summary

Make Entry lookup, writes, and discovery use the library's complete location
rules. PDF Markdown companions are Entry records in the existing layout, but
`find_entry_path()` and Entry scans currently look only in `entries/`.

## Requirements

- Follow the [Entry location contract](../../docs/architecture/library_layout.md#entry-location-contract).
  Replace the single-directory `entries_dir()` contract with an ordered
  collection of all Entry scan directories owned by `library.rs`, including
  `entries/` and `pdfs/`. Give the collection API an appropriately plural name.
  Every Entry scan must use it; callers must not assemble their own root lists.
- Provide one shared existing-Entry resolver and one shared discovery path using
  those locations. Prefer an existing `entries/{id}.md`, otherwise resolve
  `pdfs/{id}.md`. Discover each ID once, with the same precedence. Exclude PDF
  bytes and unrelated resources from Markdown parsing. Do not move, merge,
  delete, or overwrite a competing record to resolve it.
- Use the shared resolver for existing-record reads and writes, including tag,
  color, read-state, annotation, and metadata operations. PDF persistence and
  ordinary Entry creation must reuse the central location decision rather than
  create another Markdown record for an already represented ID.
- Update recovery, list/export paths, repairs, CLI operations, and conversion
  scans to use the shared discovery rules. Keep version-specific conversions
  limited to their intended transformations without duplicating location rules.
  Metadata-only PDF Entries must remain discoverable without text extraction;
  do not introduce extraction or fabricate text for semantic embedding.
- Preserve saved fields, IDs, PDF bytes, writer serialization, read-only behavior,
  and file replacement ordering. Missing optional directories are allowed;
  unreadable directories or files are errors, not proof of absence. Recovery
  must withhold absence-based deletion if any Entry directory was incompletely
  read. Existing validation boundaries remain in force.
- Remove hardcoded library paths and private location rules from production
  callers and test fixtures; obtain them from the owning library API. Add an
  automated architectural check to catch their reintroduction outside that API.
  Keep the existing on-disk format and directory layout.

## Remaining work

- Refactor the library location API and shared Entry traversal/resolution, then
  update all workspace consumers. Known single-directory consumers include
  recovery, bookmark export, summary repair, and library upgrades; PDF saving
  also independently chooses between Entry locations.
- Add regressions for Entries in each directory, same-ID precedence, writes to
  the resolved record, no competing file on creation, mixed-directory discovery,
  refresh, and recovery. Cover missing/unreadable directories, error propagation,
  read-only refusal, and incomplete-scan deletion protection. Verify the path
  guard rejects representative hardcoded paths and duplicated scan-root lists.
  Cover PDF-to-HTML refresh so resource cleanup cannot delete the selected
  Markdown record, and metadata-only PDF recovery without fabricated embedding
  input.
- Update comments and PDF documentation that describe companions as dormant,
  and align the PDF representation task's remaining work with this contract.
  Run relevant core/CLI tests and workspace checks; no PDF UI controls or text
  extraction are part of this story.

## References

- [DRY](../../docs/principles/dry.md)
- [Library locations](../../crates/xaga-core/src/library.rs)
- [Entry resolver and writes](../../crates/xaga-core/src/persistence/entries.rs)
- [Markdown traversal](../../crates/xaga-core/src/persistence/walk.rs)
- [PDF persistence](../../crates/xaga-core/src/persistence/pdfs.rs)
- [Recovery](../../crates/xaga-core/src/recovery/embeddings.rs)
- [Bookmark export](../../crates/xaga-core/src/persistence/bookmarks.rs)
- [Summary repair](../../crates/xaga-core/src/persistence/summary_repair.rs)
- [Library upgrades](../../crates/xaga-core/src/library/upgrade.rs)
- [PDF representation task](../backlog/enhancement__pdfs--library-entry-representation.md)
- [Crash consistency](../../docs/architecture/crash_consistency.md)
