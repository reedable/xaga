# Apply color bookmarks to whole PDFs

## Summary

Apply, change, and remove a color bookmark on a whole stored PDF, and find it in
Saved. Reuse Entry color bookmarks; passage highlighting and PDF annotations
are outside this story.

## Requirements

- Use [shared Entry location resolution](../../docs/architecture/library_layout.md#entry-location-contract)
  to find the writable Entry for each supported stored PDF. Store
  color on that Entry through the existing bookmark API, without a separate PDF
  bookmark store or a text-extraction requirement.
- Enable the existing toolbar color palette for the selected PDF. Use the same
  colors, recoloring, and Remove marking behavior as article bookmarks. The
  bookmark applies to the whole document and does not modify PDF bytes.
- PDF Entries appear in Saved and display the existing list color indicator.
  Selecting one opens its stored PDF according to the inline/external
  presentation preference. Changing or removing a bookmark preserves the
  current list, PDF selection, and reading position; explicit navigation or
  filter changes refresh membership under the existing ListView contract.
- Preserve title, tags, read state, annotations, and other saved Entry fields.
  Color survives reopening, PDF refresh, and cache recovery, including PDFs
  without extracted text or embeddings.
- Update visible state after a successful save. Failed saves preserve the last
  saved color and permit retry. Read-only libraries display bookmarks and Saved
  results but disable changes.

## Remaining work

- Verify shared Entry resolution, discovery, color mutation, and
  recovery paths support both feed-backed PDFs and activated PDF companions.
- Connect the PDF toolbar to existing color-bookmark operations and verify
  Saved queries, list indicators, and PDF navigation; extend capture fixtures.
- Add backend coverage for color persistence, recoloring/removal, preservation
  of other fields, refresh, and recovery. Add browser coverage for palette
  actions, Saved browsing, retained list context, failed writes, and read-only
  behavior. Verify inline/external PDF presentation and reading-position
  preservation natively where browser fixtures cannot establish them.
- Update the organization contract, User Guide, and affected training.

## References

- [PDF identity and presentation](../../docs/architecture/wry_pdf_routing.md)
- [Color bookmark contract](../../docs/architecture/organization_model.md#reading-state-bookmarks-and-color)
- [ListView](../../docs/architecture/list_view.md)
- [Article toolbar](../../src/components/content/Toolbar.tsx)
- [Bookmark operations](../../src/hooks/useBookmarks.tsx)
- [Entry persistence](../../crates/xaga-core/src/persistence/entries.rs)
- [PDF persistence](../../crates/xaga-core/src/persistence/pdfs.rs)
- [UI testing](../../docs/conventions/ui_testing.md)
