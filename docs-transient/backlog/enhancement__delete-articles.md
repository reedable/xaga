# Delete Articles

## Summary

Let users delete articles from their library. Define deletion's effect on related
data and on the list and reader currently open.

## Requirements

- Define the deletion affordance and whether deletion is recoverable.
- Define what happens to the article's annotations, pins, and references from
  Feeds and Topics.
- Define navigation after deleting the article that currently scopes the
  ListView: what replaces the article-and-annotations list, and what remains in
  the reader?
- Distinguish deletion of underlying content from unpinning, which preserves
  the open list and reading context.
- Apply library writer serialization and read-only gating; keep derived stores
  consistent with the authoritative deletion, including after recovery.

## Remaining work

- Trace article ownership, membership references, annotation storage, cache
  removal, and navigation to identify the changes required for deletion.
- Present deletion and recovery options, including the post-deletion list and
  reader behavior, for decision.
- Refine the implementation and verification work from the chosen behavior.

## References

- [Pin Articles to Shortcuts](../available/enhancement__pin-articles-to-shortcuts.md)
- [ListView behavior](../../docs/architecture/list_view.md)
- `docs/principles/single_source_of_truth.md`
- `docs/architecture/crash_consistency.md`
- `crates/xaga-core/src/persistence/articles.rs`
- `crates/xaga-core/src/library_writer/mod.rs`
- `src/components/list/ItemList.tsx`
- `src/hooks/useRouter.tsx`
