# Compliance: process SVG favicons without executing active content

## Problem

The article loader navigates directly to the selected favicon URL. Its capture
code then reads `document.images[0]` and draws that image to a canvas for PNG
storage. A standalone SVG document has an `<svg>` root rather than that image
element, so capture fails with `no-image`.

Observed example: `https://htmx.org/essays/code-is-cheap` declares
`https://htmx.org/favicon.svg`. Refresh recorded `article:favicon-unavailable`
and `no-image`; the saved article contained a local favicon path without an
image file. Repeated favicon warnings are suppressed per article per session,
so disappearance of the toast does not establish successful capture.

The failure currently prevents this SVG from being persisted and displayed as
a favicon. It must not be fixed merely by teaching the canvas code to handle an
SVG document. SVG can contain scripts, event handlers, and external resources.
Navigating to it as a document can enable active content, unlike loading it in
an image context. The later `no-image` failure does not establish that scripts
were prevented from executing during navigation. Script execution has not been
tested in this Xaga path.

The [SVG processing rules](https://www.w3.org/TR/SVG/conform.html) distinguish
document processing from image modes that disable scripting and external
resource references.

## Work

- Establish the current loader behavior using controlled SVG fixtures containing
  script elements, event handlers, and external resource references. Observe
  execution and requests, not only the final capture result.
- Choose an acquisition and conversion path that prevents SVG active content
  from executing before or during conversion. Do not navigate to an active SVG
  document and rely on PNG conversion afterward for protection.
- Keep acquisition in the untrusted-content pipeline, with existing URL,
  redirect, and network controls. Do not fetch or execute SVG in the main or
  settings webview, or introduce a remote favicon rendering fallback.
- Validate downloaded content and enforce input-size, decoded-dimension, and
  processing limits. Disable external resource loading during conversion.
  Reuse the shared resource pipeline and its injectable validation where
  applicable; do not introduce a separate favicon download infrastructure.
- Persist only successfully validated and converted image bytes using the
  existing local URL-hash storage pattern. Do not create a local favicon path
  when no file was saved. Main UI rendering must consume the local result.
- Report unsupported or rejected SVG through existing favicon diagnostics.
  Make explicit reload outcomes observable even after an earlier favicon
  warning for the same article.
- Update durable security documentation with the enforced acquisition,
  validation, conversion, and rendering boundaries.

## Acceptance

- A supported, static SVG favicon produces a locally stored image and renders
  through the existing favicon UI path.
- Script elements and event handlers never execute during favicon processing;
  external references do not cause conversion-time network requests.
- Malformed, oversized, or unsupported SVG fails without persisting an unusable
  local path, exposing active content to the trusted UI, or preventing article
  loading. Failure remains diagnosable after an explicit retry.
- Existing raster favicon capture, validation, local storage, and rendering
  continue working.

## Relevant implementation

- `src-tauri/src/article_loader.js`: `__xagaIsImageDocument`,
  `__xagaCaptureFavicon`.
- `src-tauri/src/commands/articles.rs`: favicon navigation and receipt.
- `src-tauri/src/loader_download.rs`, `src-tauri/src/resource_download.rs`:
  shared download handling and validation.
- `crates/xaga-core/src/persistence/articles.rs` and `images.rs`: favicon
  persistence.
- `src/components/content/Toolbar.tsx` and
  `src/providers/ErrorSinkProvider.tsx`: local display and warning suppression.
