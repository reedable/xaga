# Preserve responsive-image URLs during article ingestion

## Summary

Failed or disabled image capture can leave responsive-only image URLs out of the
saved Markdown. Preserve the browser-selected URL through article conversion.

## Requirements

- Before HTML leaves the loader, retain each image's available browser-selected
  `currentSrc` as the conversion fallback, including `srcset` and `<picture>`.
  Use existing `src` when no selected URL is available; do not invent a candidate
  selection policy when neither exists.
- Preserve successful capture's URL/byte pairing, alternative text, and titles.
- Do not add network requests or change **Save article images for offline
  viewing** behavior. Unfetched images retain remote references and use the
  existing unavailable presentation; existing setting-gated recovery is unchanged.
- Scope changes to article ingestion; leave feed summaries and externally edited
  Markdown handling unchanged.

## Remaining work

- Carry the selected URL through loader HTML and Markdown conversion without
  relying on successful image capture.
- Test relative URLs, `srcset`-only images, `<picture>` with/without fallback
  `src`, unavailable `currentSrc`, and successful/failed/disabled capture.
- Run relevant conversion/component tests and `npm run test:browser`; verify
  browser-selected URL capture in the native loader. Update durable image docs.

## References

- [Loader](../../src-tauri/src/article_loader.js)
- [Article conversion](../../src/text/entryWorker.ts)
- [Ingestion contract](../../docs/architecture/security/handling_remote_content.md)
- [Image presentation](../../docs/architecture/security/handling_persisted_content.md)
