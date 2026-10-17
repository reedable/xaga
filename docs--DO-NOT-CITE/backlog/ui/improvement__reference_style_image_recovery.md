# Recover reference-style Markdown images

## Summary

Reference-style Markdown separates image placement from its URL:

```markdown
![A cat][cat-photo]

[cat-photo]: https://example.com/cat.jpg "Photo caption"
```

Xaga's regular body-capture pipeline generates inline images (`![alt](URL)`).
Capture failure preserves that inline remote reference; it does not create
reference-style images. This task addresses manually edited or externally
authored Markdown.

Image recovery does not resolve these references through their definitions. Manually edited or externally authored entries therefore cannot recover their remote images through the existing setting-gated pipeline.

## Requirements

- Resolve full, collapsed, and shortcut Markdown image references through their
  definitions using Markdown parsing rules.
- Reuse the shared setting-gated, isolated acquisition and local persistence
  pipeline for summaries and bodies. Do not fetch through the reader webview.
- Save resources before rewriting successful references, preserving alternative
  text, titles, unrelated content, and concurrent edits. Failed recovery leaves
  persisted references unchanged.
- Account for definitions shared by multiple images or ordinary hyperlinks.
  Preserve ordinary hyperlink destinations; determine the rewrite strategy
  before implementation rather than blindly replacing shared definitions.

## Remaining work

- Extend discovery to return reference uses and their definitions.
- Define rewriting when one definition is shared by images and ordinary links.
- Integrate resolved targets with the existing recovery operation.
- Test full, collapsed, shortcut, shared, missing, and already-local references with recovery enabled and disabled.

## References

- `src/text/imageReferences.ts`
- `src/text/prepareEntryImages.ts`
- `src/text/entryWorker.ts`
- `crates/xaga-core/src/persistence/image_recovery.rs`
- `docs/architecture/security/handling_persisted_content.md`
