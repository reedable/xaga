# Improvement: reconsider favicon persistence

## Summary

Favicon acquisition has a separate phase within the article loader, but its
persistence is tied to article saving and per-entry image storage. Reconsider
whether that coupling is useful, or whether favicons would fit better as shared
site metadata.

## Requirements

- Identify the friction caused by passing favicon results through article saving,
  including duplicate storage and retries that require article reloads.
- Decide whether to keep per-entry storage or separate favicon persistence.
  Consider cleanup: shared durable files require reference tracking, while a
  disposable cache could use eviction and show a placeholder when an icon is absent.
- Document the rationale. This task does not assume that shared storage is the answer
  or propose changing article-image storage.

## Remaining work

- Review the current favicon acquisition and persistence flow.
- Choose an approach and identify the concrete changes needed.
- Record the decision in durable architecture documentation.

## Reference

- `src-tauri/src/article_loader.js` and `src-tauri/src/commands/articles.rs`:
  favicon acquisition and capture delivery.
- `crates/xaga-core/src/persistence/articles.rs`: favicon storage during article saving.
- `docs/architecture/crash_consistency.md`: per-entry image cleanup and deletion.
- `docs/architecture/security/handling_persisted_content.md`: local favicon
  rendering and reload-based retries.

The existing docs describe the storage model but do not explicitly explain why
favicons are stored per entry rather than shared.
