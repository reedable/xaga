# Preserve and recover responsive images

## Summary

Images can supply their URLs through `srcset` or `<picture><source>` rather
than a single `<img src>`. Readability preserves these elements and attributes,
but Xaga's subsequent HTML-to-Markdown conversion only uses the image's `src`
and ignores `<source>` elements.

- An image with only `srcset` becomes `![alt]()`, losing its source URLs.
- A `<picture>` retains its fallback `<img src>` URL but loses the alternative
  source candidates.
- Successful article-loader capture can avoid this loss by fetching the
  browser-selected `currentSrc` and supplying a `src` before conversion.
  Failed or disabled capture can leave the responsive references to be lost
  during conversion.
- Feed summaries bypass this conversion and can retain responsive-image HTML.
  Current recovery discovers `img src` but not responsive-only sources. Reader
  preparation removes responsive candidates so they cannot trigger remote
  acquisition from the reader.

## Requirements

- Establish candidate selection and persistence behavior for `srcset`, `sizes`,
  and `<picture>` media/type alternatives, including when no `src` exists.
- Preserve a recoverable image reference through article conversion when image
  capture fails or is disabled. Preserve alternative text and titles.
- Extend the shared image-recovery path to responsive references in summaries
  and manually edited body HTML. Keep feed ingestion unchanged.
- Reuse the resolved **Load remote image** setting, isolated downloader, shared
  image validator, and existing local persistence. Keep acquisition outside the
  main webview and reader; save resources before rewriting stored references.
- Preserve unrelated content and concurrent edits. When acquisition fails,
  retain recoverable references and use the common unavailable-image display.

## Remaining work

- Choose the candidate-selection policy for density, width, media, and type alternatives.
- Implement conversion fallback and recovery discovery for the selected candidates.
- Test `srcset`-only images, relative URLs, `<picture>` with and without fallback `src`, and successful, failed, and disabled capture.

## References

- `src-tauri/src/article_loader.js`
- `src/text/entryWorker.ts`
- `src/text/imageReferences.ts`
- `src/text/prepareEntryImages.ts`
- `docs/architecture/security/handling_remote_content.md`
- `docs/architecture/security/handling_persisted_content.md`
