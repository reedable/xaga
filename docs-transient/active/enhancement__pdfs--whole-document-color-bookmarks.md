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
- Legacy stored PDFs without an Entry remain viewable with marking unavailable.
  Creating Entries for those files belongs to the separate PDF migration.
- Enable the existing toolbar color palette for the selected PDF. Use the same
  colors, recoloring, and Remove marking behavior as article bookmarks. The
  bookmark applies to the whole document and does not modify PDF bytes.
- PDF Entries appear in Saved and display the existing list color indicator.
  Selecting one opens its stored PDF according to the inline/external
  presentation preference. Changing or removing a bookmark preserves the
  current list, PDF selection, and reading position; explicit navigation or
  filter changes refresh membership under the existing ListView contract.
- The first successful marking on a newly downloaded standalone PDF appears
  in Saved on the next explicit navigation or filter change in the same
  session when the index service is available. Publish the saved marking to
  the index before reporting completion, without waiting for recovery, text
  extraction, or a vector. Saved keeps its existing availability during index
  startup or failure.
- Keep the palette available in Xaga for inline and external PDF presentation.
  Changing the marking leaves the open viewer and its reading position intact.
- Preserve title, tags, read state, annotations, and other saved Entry fields.
  Color survives reopening, PDF refresh, and cache recovery, including PDFs
  without extracted text or embeddings.
- Update the visible color from the committed Entry. A failed Entry write
  preserves the last saved color and permits retry. If index publication fails
  after the write, show the committed color and report that Saved could not be
  updated. Reapplying the same color or removal can retry publication.
  Read-only libraries display bookmarks and Saved results but disable changes.

## Remaining work

- In the macOS app with a disposable library, open a multipage PDF and advance
  past the first page. Apply, change, and remove the bookmark from Xaga’s
  toolbar. Verify that the viewer stays open on the same page. Repeat with
  **Show PDFs in the reading pane** disabled and verify the same behavior in
  the default PDF viewer. Select the marked PDF from Saved in both modes to
  verify that Xaga opens the stored PDF using the selected presentation.

## References

- [PDF identity and presentation](../../docs/architecture/wry_pdf_routing.md)
- [Color bookmark contract](../../docs/architecture/organization_model.md#reading-state-bookmarks-and-color)
- [ListView](../../docs/architecture/list_view.md)
- [Article toolbar](../../src/components/content/Toolbar.tsx)
- [Bookmark operations](../../src/hooks/useBookmarks.tsx)
- [Entry persistence](../../crates/xaga-core/src/persistence/entries.rs)
- [PDF persistence](../../crates/xaga-core/src/persistence/pdfs.rs)
- [UI testing](../../docs/conventions/ui_testing.md)
