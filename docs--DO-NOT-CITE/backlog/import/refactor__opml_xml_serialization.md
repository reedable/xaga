# Replace manual OPML XML construction

`crates/xaga-core/src/persistence/opml.rs::build_opml` constructs XML tags and
attributes with string concatenation and formatting. It uses `quick_xml` for
escaping, but not for writing the document structure.

Replace manual XML construction with a library XML writer or serializer.
Preserve the existing OPML 2.0 export content: document title, feed order, and
outline `text`, `title`, `type`, and `xmlUrl` attributes. The specific API choice
remains to be reviewed before implementation.

Verify exported documents by parsing them and checking feed titles and URLs,
including ampersands, quotes, angle brackets, Unicode, and an empty feed list.
The serializer must handle escaping without pre-escaping or double-escaping.

This item records requested follow-up work; it does not change OPML export code.
