# Replace manual OPML XML construction

## Summary

`build_opml` constructs document structure with string concatenation and uses `quick_xml` only to escape attribute values. Replace the manual construction with an XML writer or serializer.

## Requirements

- Preserve OPML 2.0 output containing the `Xaga Feeds` document title and one outline per feed in input order.
- Preserve each outline's `text`, `title`, `type="rss"`, and `xmlUrl` attributes.
- Delegate element and attribute encoding to the selected XML API without pre-escaping or double-escaping values.
- Produce well-formed output for empty input and for titles and URLs containing XML metacharacters or Unicode.

## Remaining work

- Compare the `quick_xml` writer API with serializer alternatives and select the dependency approach.
- Replace `build_opml`'s string-built XML structure.
- Parse generated documents in tests and verify document metadata, feed order, and attribute values.
- Cover empty input, ampersands, quotes, angle brackets, and Unicode.

## References

- `crates/xaga-core/src/persistence/opml.rs`
- `src-tauri/src/native/feeds.rs`
