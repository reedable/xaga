# Bookmark Articles with Colors

## Summary

Make bookmarking an article the act of assigning it a color. Consolidate article
bookmark colors and annotation colors into the existing annotation swatch palette.
Deliver storage conversion and the toolbar
interaction together.

## Requirements

- An article is bookmarked when it has a color marking. Removing the marking
  removes the bookmark. Remove `is_bookmarked`/`isBookmarked` everywhere: domain
  types, serialized payloads, state patches, events, frontend models, and LanceDB
  schema and indexes. Do not retain a stored or derived bookmark boolean.
- Color remains optional for Entries. Unmarked articles require no color.
- Use the existing annotation swatch colors for article bookmarks. Users select
  a swatch color; do not introduce a separate bookmark-blue palette option.
- Convert legacy bookmarks without a color to Blue. Bookmark imports whose
  source provides no color also receive Blue.
- Clicking the article toolbar bookmark control opens a swatch popup. Selecting
  a color marks the article and updates the toolbar icon to that color; provide
  removal of the marking. Do not offer article comments in this story.
- Preserve Saved membership for marked articles and annotations, bookmark
  import/export, reading state, organizational tags, pins, and open-list context.
- Persist authoritative article color state in the library and derive cache and
  domain projections from it. Store optional `color` in article frontmatter using
  the annotation palette identifiers; omit it when unmarked.
- All articles with a color marking are bookmarked items and are included in
  bookmark export. Export membership even when the destination format cannot
  represent color.
- Convert existing bookmarks through xaga-cli, preserving their marked state and
  unrelated article data. Make migration atomic per file and safe to retry, and
  reconcile cached state.
- Use existing writer serialization and read-only gating for color mutations.
- Extend the existing `bookmark:changed` event, carrying the updated Entry, to
  fire on article color assignment, removal, and changes between colors. A
  color-to-color change must notify subscribers even when derived bookmark
  membership stays true. Keep annotation color notifications on their existing
  annotation event paths.
- Increment the minor release version for the storage-model change and document
  the format and CLI upgrade procedure.
- Keep Tags, article comments, and expansion of marking to other entity types
  outside this story.

## Remaining work

- Implement article color storage and preserve valid existing palette values.
- Replace all bookmark boolean consumers with color-presence checks, including
  filters, counts, import/export, toolbar state, and change detection. Remove the
  old database column and index during upgrade and from newly created schemas.
- Reuse annotation swatch selection/removal, dismissal, and focus behavior for
  article bookmarks, without passage-selection or comment controls.
- Implement core persistence, writer commands, platform boundaries, events,
  database reads/writes, filtering, and recovery for color-backed bookmark state.
- Update backend change detection and state propagation to compare color values,
  and ensure event payload mapping retains optional Entry color. Update toolbar,
  list, and pinned-article subscribers to reflect the event's color state.
- Map an article's color, when present, into its LanceDB Entry row instead of
  always writing an empty string. Retain the empty database representation for
  unmarked articles and restore optional color in Entry summary reads.
- Add a bitmap scalar index on `color` for palette equality and nonempty-color
  predicates. The locked LanceDB 0.33.0 / lance-index 9.0.0 supports string bitmap
  equality; its scalar planner implements `!=` as indexed equality followed by
  negation. Use `color = 'blue'` for a color and `color != ''` for marking
  membership, combined with the appropriate entity-kind predicate. No stored
  bookmark boolean is required.
- Verify index use with query plans for the actual combined kind/color and Saved
  queries. Measure at the multi-million-entry design target, including after
  color assignment, changes, and removal; source-level operator support alone
  does not establish runtime performance.
- Update CLI migration and bookmark import/export with the Blue fallback and
  membership export for formats that cannot carry color.
- Update durable domain, source-of-truth, crash-consistency, and upgrade docs.
- Verify palette consistency, toolbar selection/removal, Saved filtering,
  import/export, migration retries, data preservation, live propagation,
  concurrent changes, read-only behavior, restart, and cache rebuilds.
- Verify marked and unmarked Entry round trips, color changes and removal, and
  query correctness after cache rebuild and index creation.
- Verify `bookmark:changed` delivery and UI updates for assignment, removal, and
  color-to-color changes, including when bookmark membership remains unchanged.
- Run workspace formatting, affected Rust/frontend tests, lint, and production build.

## References

- [Article Bookmark Comments](../ideas/article-bookmark-comments.md) — separate idea.
- `docs/library_upgrade.md`
- `docs/development/release_cycle.md`
- `docs/principles/single_source_of_truth.md`
- `docs/architecture/crash_consistency.md`
- `src/components/content/Toolbar.tsx`
- `src/components/content/AnnotationTooltip.tsx`
- `crates/xaga-core/src/domain.rs`
- `crates/xaga-core/src/md.rs`
- `crates/xaga-core/src/library_writer/mod.rs`
- `crates/xaga-core/src/persistence/bookmarks.rs`
- `crates/xaga-core/src/embeddings/`
- `crates/xaga-core/src/query/filter.rs`
- `crates/xaga-cli/src/cmd/library_upgrade.rs`
