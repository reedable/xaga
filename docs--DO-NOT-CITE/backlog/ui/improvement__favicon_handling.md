# Improvement: favicon discovery and presentation

## Discovery consistency

The loader and `src/text/entryWorker.ts::extractFavicon` duplicate icon selection
logic. The loader resolves against `document.baseURI` and falls back to
`/favicon.ico`; the worker resolves against the supplied article URL and has no
conventional fallback. Consolidate their contract and test the cases where the
worker supplies the icon reference.

Verify declared and conventional icons, differing base URLs, and relative
references. Selected URLs must continue through the common security controls.

## Normalization consistency

The canvas branch produces a square image, while the download validator
preserves aspect ratio. Decide the intended normalization behavior and make
both branches consistent, including nonsquare icons and the user-visible outcome
for unsupported formats.

## Enhancement investigation

- Broader icon selection: multi-value `sizes`, `sizes="any"`, touch icons,
  manifest icons, and trying another declared candidate after failure.
- Origin-level reuse instead of per-entry storage. Define freshness, negative
  caching, refresh, concurrency, deletion, and read-only behavior before adding
  a cache.
