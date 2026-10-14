# Improvement: unavailable images and feed-summary image handling

Implement one shared unavailable-image handling path for summaries and article
bodies. Summary images enter this path naturally because summaries are initially
persisted with remote image references. Body images enter the same condition
after capture failure, capture being disabled, or a manual Markdown edit. Users
can also insert remote images into summaries manually. Handling depends on the
available image resource and setting, not on the reference's provenance.

## Agreed behavior

- Keep feed ingestion unchanged. Continue persisting the feed-supplied summary
  in the `.md` frontmatter with its remote image references. Do not fetch images
  or rewrite those references during feed ingestion.
- Viewing the entry is the timely opportunity for image recovery. Resolve
  already-available local images first; for unavailable remote images, gate
  recovery on the future user-controllable **Load remote image** setting.
- Use the existing Xaga settings mechanism. Define the default in one place in
  the settings layer, initially `true`, and have consumers use the resolved
  value. An absent persisted value uses that default; an explicitly stored
  value takes precedence. Do not eagerly write `true` or scatter fallback
  defaults through consumers. The default may later change to `false`.
  Building the settings UI is outside this task.
- Generalize the existing isolated resource-downloader pipeline used by the
  article loader for PDFs and favicons so it supports PDFs and general images.
  Use that shared pipeline for remote-image recovery.
  Viewing triggers recovery; it does not authorize remote fetching from the
  main webview or reader. Do not add a render-time remote-fetch exception.
- Save recovered resources locally before committing any updated content
  references, then render the locally resolved content. Update the summary
  where appropriate without losing its content or unrelated frontmatter.
  Capture or persistence failure must preserve the existing content and
  references rather than inventing a successfully persisted local resource.
- Apply the same recovery and unavailable-image presentation to summaries and
  bodies. With recovery disabled, already-persisted images remain usable.
  When an image remains unavailable, present useful preserved information
  without requiring separate treatment for manual edits and capture failures.

Successful recovery rewrites the stored summary or body references to local
paths, only after the corresponding resources are saved. This persistence
choice is agreed. SVG and AVIF recovery validation remain an open format-coverage
choice; the existing native decoder covers PNG, JPEG, GIF, WebP, and ICO.

## Resource-downloader generalization

The existing staging/validation boundary accepts consumer-provided validators
and byte limits, but its loader adapter currently selects PDF handling before
article HTML capture and favicon handling while captured HTML awaits its icon.
Image handling is therefore coupled to the favicon phase rather than usable
directly for an arbitrary summary or body image reference.

- Make the shared resource downloader usable for PDFs and general images,
  including direct image recovery without requiring a preceding article HTML
  capture. Preserve existing PDF and favicon behavior.
- Reuse the existing isolated loader network/session policies, request
  ownership, timeout/cancellation handling, temporary staging, and explicit
  validation boundary. Keep remote acquisition outside the main webview and
  reader.
- Cover both responses WebKit displays as image documents and responses it
  classifies as native downloads. The current image-document path uses canvas
  capture; the native-download path uses a Rust image validator.
- Separate favicon transformations from general image acquisition. Both
  existing favicon paths produce small static PNGs; those transformations must
  not silently resize, distort, or discard animation from recovered content
  images. Establish format coverage and resource limits explicitly, including
  differences between the two acquisition paths, and surface any fidelity
  trade-offs for a decision.
- Keep acquisition and validation separate from library persistence. Accepted
  image resources must join the existing local-image storage and writer
  ordering rather than introduce a second persistence scheme.

## Implementation and verification status

Implemented:

- Centralized, default-true setting resolution, honoring explicitly persisted
  values without adding UI or eagerly writing a default.
- Shared isolated downloader support for direct image documents and native
  image downloads, preserving original bytes and keeping favicon transformations
  separate. PDF and favicon consumers remain supported.
- Recovery on viewing either summary or body, local resolution first, cancellation,
  and writer-ordered persistence. The writer checks the expected content before
  replacing references, preserving concurrent edits and unrelated metadata.
- Source-preserving recovery for inline Markdown images, HTML `img src`, inline
  CSS images, and images in embedded stylesheets. Font resources and stylesheet
  imports are not acquired as images.
- Shared unavailable-image presentation using preserved alt/title information;
  reader remote image fetching remains blocked. Failed recovery leaves persisted
  references unchanged.

Verification completed: native compilation, persistence/image unit tests, native
loader tests, command ACL tests, reader/recovery frontend tests, TypeScript and
production build. Native WebKit smoke tests passed for image documents, image
downloads, PDFs, and favicons. These checks do not establish a completed visual
review of the bundled reader.

Remaining coverage and decisions:

- SVG and AVIF validation remain open, as described above.
- Reference-style Markdown images and `srcset`-only images are not yet discovered
  for recovery. Responsive sources are removed for rendering so they cannot
  bypass isolated acquisition; an available `src` is used. These are current
  implementation gaps, not agreed scope exclusions.
- Complete bundled-reader visual verification of unavailable presentation and
  image-centric summaries with recovery enabled and disabled.

Relevant code: `src/text/markdownPipeline.ts`, `src/text/entryWorker.ts`,
`src/components/content/ArticleReader.tsx`, `src-tauri/src/resource_download.rs`,
`src-tauri/src/loader_download.rs`, `src-tauri/src/article_loader.rs`,
`src-tauri/src/article_loader.js`, and `src-tauri/src/commands/articles.rs`.

Establish any needed summary-image baseline when working on this task. Update
durable documentation with the shared resource-downloader behavior, reader
presentation, and summary-image capture, persistence, and rendering behavior.
