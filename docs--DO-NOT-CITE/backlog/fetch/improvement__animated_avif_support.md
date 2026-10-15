# Improvement: animated AVIF support

Support animated AVIF (`avis`) in shared image validation. The current AVIF
parser rejects animated containers.

## Work

- Select and implement validation that accepts animated AVIF, with explicit
  input-size and processing limits. Review dependency licensing and distribution
  obligations before adding or replacing a dependency.
- Use the same image validator for favicon and image-recovery consumers across
  native downloads and image-document callbacks. Keep acquisition in the
  existing isolated loader and reuse local-image persistence.
- Preserve original animation bytes for recovered content images. Keep any
  favicon-specific transformation separate from validation.
- Update durable documentation with supported formats and validation limits.

## Completion criteria

- Valid animated AVIF is accepted through native downloads and image-document
  callbacks for both image recovery and favicon consumers.
- Recovered images persist locally and animate in the bundled reader without
  remote reader requests.
- Malformed, truncated, and oversized inputs fail without committing unusable
  local references or overwriting existing content.
- Still AVIF and the other supported image formats continue to work.

Relevant code: `src-tauri/src/resource_download.rs`,
`src-tauri/src/loader_download.rs`, `src-tauri/src/article_loader.js`, and
`src-tauri/src/commands/articles.rs`.
