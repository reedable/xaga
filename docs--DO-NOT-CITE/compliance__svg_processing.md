# Compliance: SVG handling across acquisition and rendering

## Summary

Define and enforce SVG handling across favicons, summary and body images, and
inline SVG in HTML. Cover remote acquisition, validation, persistence, and
rendering, including manually edited local content. Distinguish SVG processed
as an image from publisher content executing in the isolated article loader.

## Requirements

- Use the existing isolated acquisition pipeline, shared image validator, and
  local resource storage. Do not add a separate SVG or favicon downloader.
- Prevent active SVG content from executing during dedicated image processing
  or in the trusted UI. Do not rely on rejection or conversion after execution
  as protection. Assess ordinary publisher-page execution in the article loader
  separately; it already permits publisher scripts.
- Prevent SVG external references from causing unapproved network requests during
  image processing and local rendering. Cover script requests and declarative
  references, including nested images, styles, fonts, and SVG element references.
- Keep remote acquisition outside the main and settings webviews and the reader.
  Preserve existing URL, redirect, session, and network controls.
- Treat stored SVG as untrusted on read. Cover both SVG image resources and inline
  SVG markup; do not assume their browser execution rules are interchangeable.
- Enforce input-size and processing limits. Keep favicon transformations separate
  from shared validation. Establish any conversion or sanitization requirements
  and their effects on appearance and animation before changing stored content.
- Save resources before replacing references. Preserve existing content on
  failure and use existing unavailable-image presentation and diagnostics.

## Remaining work

- Trace SVG through favicon acquisition, article-body capture, summary/body image
  recovery, local image readback, and inline-HTML rendering. Identify the controls
  at each boundary and any gaps.
- Exercise controlled SVG fixtures containing scripts, event handlers, external
  resources, and nested content. Observe execution and requests in native WebKit,
  rather than inferring protection from the final capture result.
- Implement missing controls within the shared pipeline. Cover image-document
  responses, native downloads, inline SVG, and manually edited local files.
- Verify supported SVG display in favicons, summaries, and bodies; safe handling
  of malformed and oversized input; preserved content on failure; and continued
  operation of raster-image and PDF consumers.
- Update durable security documentation with the enforced boundaries, supported
  SVG behavior, and verified limits.

## Reference

- Acquisition and validation: `src-tauri/src/article_loader.js`,
  `src-tauri/src/article_loader.rs`, `src-tauri/src/commands/articles.rs`,
  `src-tauri/src/loader_download.rs`, `src-tauri/src/resource_download.rs`.
- Capture and rendering: `src/text/entryWorker.ts`,
  `src/text/markdownPipeline.ts`, `src/text/imageReferences.ts`,
  `src/text/prepareEntryImages.ts`, `src/text/buildEntryHtml.ts`,
  `src/components/content/HtmlReader.tsx`, `src/components/content/Toolbar.tsx`,
  `src-tauri/tauri.conf.json`.
- Persistence: `crates/xaga-core/src/persistence/images.rs` and
  `crates/xaga-core/src/persistence/image_recovery.rs`.
- Security boundaries: `docs/architecture/security/fetching_remote_content.md`,
  `docs/architecture/security/handling_remote_content.md`, and
  `docs/architecture/security/handling_persisted_content.md`.
- SVG processing modes: https://www.w3.org/TR/SVG2/conform.html
