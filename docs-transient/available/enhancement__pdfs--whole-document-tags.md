# Assign tags to whole PDFs

## Summary

Assign and remove tags on a whole stored PDF, and find it through existing Tags
browsing. Reuse Entry tags; PDF passage and annotation tagging are outside this
story.

## Requirements

- Depends on [shared Entry location resolution](../available/defect__entry-location-resolution.md)
  providing a discoverable, writable Entry for each supported stored PDF. The
  PDF's existing URL-derived Entry identity owns its assignments. Do not create
  a separate PDF tag store or require text extraction for tagging.
- Make the shared Entry tag picker available for the selected PDF through the
  existing wide Inspector and narrow drawer. Support assigning existing tags,
  creating and assigning a tag, and removing an assignment with the same
  immediate-save behavior as article tags.
- Tagging affects the whole document only. Preserve existing metadata, color,
  annotations, and PDF bytes. Assignments survive reopening, PDF refresh, and
  cache recovery and participate in existing tag rename, merge, and deletion.
- Include PDF Entries in existing tag intersections, counts, and All/Saved
  filtering without requiring semantic embeddings. Selecting a result opens
  the stored PDF using the existing inline/external presentation preference.
- Preserve the open PDF, reading position, and current list while assignments
  change. Show successful writes only after saving; report errors and allow
  retry. Read-only libraries allow inspection and browsing but refuse writes.

## Remaining work

- Verify the prerequisite's Entry resolution, discovery, tag mutation, and
  recovery paths support both feed-backed PDFs and activated PDF companions.
- Connect PDF inspection to the shared tag picker and existing Entry assignment
  APIs; extend capture fixtures and tag-result navigation for PDFs.
- Add backend coverage for assignment persistence, refresh, recovery, and tag
  lifecycle operations. Add browser coverage for wide/narrow assignment,
  creation, removal, tag browsing, save failure, and read-only behavior. Verify
  PDF presentation and preserved reading position natively where browser
  fixtures cannot establish them.
- Update the Inspector/tag contracts, User Guide, and affected training.

## References

- [PDF identity and presentation](../../docs/architecture/wry_pdf_routing.md)
- [Tag storage and assignment](../../docs/architecture/tags.md)
- [Inspector](../../docs/architecture/article_inspector.md)
- [ListView](../../docs/architecture/list_view.md)
- [Reader and Inspector layout](../../src/components/content/EntityView.tsx)
- [Shared tag picker](../../src/components/content/EntityTags.tsx)
- [Entry persistence](../../crates/xaga-core/src/persistence/entries.rs)
- [PDF persistence](../../crates/xaga-core/src/persistence/pdfs.rs)
- [UI testing](../../docs/conventions/ui_testing.md)
