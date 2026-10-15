# Improvement: reference-style Markdown image recovery

## Problem

Reference-style Markdown separates image placement from its URL:

```markdown
![A cat][cat-photo]

[cat-photo]: https://example.com/cat.jpg "Photo caption"
```

Xaga's regular body-capture pipeline generates inline images (`![alt](URL)`).
Capture failure preserves that inline remote reference; it does not create
reference-style images. This task addresses manually edited or externally
authored Markdown.

The current recovery discovery does not resolve image references through their
definitions. A remote image in this form therefore receives unavailable-image
presentation without recovery, even when **Load remote image** is enabled.

## Work

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

## Completion criteria

Cover full, collapsed, and shortcut references; shared image/link definitions;
missing definitions; already-local resources; capture and persistence failures;
and recovery enabled and disabled. Verify preservation of alt text, titles,
ordinary links, and unrelated document content.

Relevant code: `src/text/imageReferences.ts`, `src/text/prepareEntryImages.ts`,
`src/text/entryWorker.ts`, and
`crates/xaga-core/src/persistence/image_recovery.rs`.
