# Improvement: animated AVIF support

## Summary

Support animated AVIF (`avis`) in shared image validation. The current AVIF
parser rejects animated containers.

## Requirements

- Accept valid animated AVIF for favicon and image-recovery consumers through
  native downloads and image-document callbacks.
- Preserve original animation bytes for recovered images, persist them locally,
  and animate them in the bundled reader without remote requests. Keep favicon
  transformations separate from validation.
- Reject malformed, truncated, and oversized inputs without committing unusable
  references or overwriting existing content. Preserve support for still AVIF
  and other image formats.

## Remaining work

- Select and implement animated AVIF validation with explicit input-size and
  processing limits. Review licensing and distribution obligations for dependency
  changes.
- Integrate through the shared image validator, existing isolated loader, and
  local-image persistence.
- Verify both acquisition paths and consumers against the requirements above.
- Update durable documentation with supported formats and validation limits.

## References

- [Image validation](../../../src-tauri/src/resource_download.rs)
- [Download consumers](../../../src-tauri/src/loader_download.rs)
- [Image-document acquisition](../../../src-tauri/src/article_loader.js)
- [Article commands](../../../src-tauri/src/commands/articles.rs)
