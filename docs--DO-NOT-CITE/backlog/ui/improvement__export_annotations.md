# Export annotations

## Summary

Xaga has no annotation export workflow. The exported scope, representation, and destination are undefined.

## Requirements

- Preserve annotation text, comment, color, timestamps, and source-article identity or URL where available.
- Make export user-initiated and write only to a user-selected destination.
- Define behavior for missing source articles and annotations whose selected text no longer resolves.
- Avoid exporting private data not required by the selected format.
- Produce deterministic output where the format permits it.

## Remaining work

- Define export scope: all annotations, current article, Topic, or a user selection.
- Compare interoperable and Xaga-round-trippable formats, including their lossiness.
- Decide whether export is archival, interchange, or both.
- Define native destination selection and overwrite behavior.
- Implement and test Unicode, multiline comments, missing sources, and empty exports.

## References

- `crates/xaga-core/src/domain.rs`
- `crates/xaga-core/src/persistence/articles.rs`
- `src-tauri/src/native`
- `docs/architecture/security/command_surface.md`
