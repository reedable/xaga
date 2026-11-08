# Investigate responsive-image recovery in feed summaries

## Summary

Feed summaries can retain `srcset` and `<picture>` HTML, but image recovery only
discovers `img src`. Determine whether and how responsive-only summary images
should participate in the existing setting-gated recovery.

## Requirements

- Use representative feed-summary examples to establish the user-visible gap.
- Compare candidate selection for density, width, media, and type alternatives,
  including relative URLs, absent fallback `src`, and unsupported formats.
- Evaluate preserving responsive markup versus storing one selected local image,
  including compatibility and rewrite consequences.
- Retain the existing local-only reader policy and acquisition boundary. Proposals
  must honor **Save article images for offline viewing**, preserve unrelated
  content, and leave failed acquisitions unchanged.
- Exclude externally edited article bodies and changes to feed ingestion.

## Remaining work

- Reproduce responsive-only summary presentation and compare selection/rewrite
  options, implementation costs, and limits without choosing a product policy.
- Record findings in durable docs and present options for PO decision. Create
  bounded follow-up stories where warranted, then delete this investigation.

## References

- [Image discovery](../../src/text/imageReferences.ts)
- [Reader preparation](../../src/text/prepareEntryImages.ts)
- [Recovery persistence](../../crates/xaga-core/src/persistence/image_recovery.rs)
- [Image contract](../../docs/architecture/security/handling_persisted_content.md)
