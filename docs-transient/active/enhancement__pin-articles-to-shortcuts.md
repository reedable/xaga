# Pin Articles to Shortcuts

## Summary

Let users pin an article to Shortcuts. Selecting its shortcut scopes the ListView
to that article: the article appears first, followed by its annotations. Deliver
persistence and the UI together as one story.

## Requirements

- Persist article pins within their library and restore them across sessions.
- Store pin state as the `pinned` tag in the article's `.md`, following the
  existing Topic pin pattern; derive cached pin state from that file.
- Include pinned articles in Shortcuts alongside existing pinned targets.
- Provide pin and unpin controls in the article toolbar.
- Apply the existing Shortcuts sorting modes to article shortcuts; show no count
  on article shortcuts.
- Selecting an article shortcut shows the article first and its annotations
  below it in passage order in the ListView.
- Place annotations with unresolved anchors at the bottom in arbitrary order.
- Retain text filtering.
- Omit Unread/Saved/All controls and the Mark all read action in this scope.
- Preserve the open list and reading context when its article is unpinned or its
  read state changes. Unpinning removes the shortcut, not the open list.
- Selecting the article row opens the article; selecting an annotation row opens
  the parent article and scrolls to that annotation.
- Preserve the article scope while selecting rows within it; distinguish the
  list target from the selected article or annotation.
- Support keyboard navigation and correct selected-row indication for both kinds
  of row.
- Reflect annotation creation, editing, replacement, and removal in the scoped
  list.
- Route pin mutations through the library writer and existing read-only gating.
- Keep this story independent of the Tags and Inspector redesign.
- Implement article pinning using the existing pin semantics and persistence
  pattern. Completion of the broader domain-definition spike is not a
  prerequisite for this story.
- Article deletion is outside this story.

## Remaining work

- Use the writer's existing entry-tag patching lane for pin mutations.
- Implement passage ordering from annotation anchors, including multi-segment
  annotations, with unresolved anchors placed last.
- Integrate article shortcuts with the selectable ListView header used when
  the sidebar is hidden.
- Implement authoritative pin writes, pin discovery, platform commands, and live
  state propagation. If pins are mirrored in LanceDB, include recovery: Entry
  currently has no pin field and article row construction sets `is_pinned` false.
- Add article shortcut navigation and article-scoped list loading. Existing
  article reads include annotations, and mixed article/annotation rows already
  render in the ListView.
- Update row selection and keyboard navigation to account for `annotationId`;
  they currently use only `entryId`. Handle annotation additions in the new scope
  as well as edits, replacements, and removals.
- Update durable documentation for the chosen ownership and implemented behavior.
- Verify persistence across restart and recovery, library isolation, concurrent
  state changes, read-only refusal, shortcut navigation, list ordering and
  selection, keyboard operation, annotation updates, filtering, and pin lifecycle.
- Run formatting and checks appropriate to the affected Rust and frontend code,
  including `cargo fmt --all --check` and the frontend production build.

## References

- [Pin and domain definitions](../active/spike__define-tag-collection-bookmark-pin-shortcut-site.md)
- `docs/principles/single_source_of_truth.md`
- `docs/architecture/crash_consistency.md`
- [ListView behavior](../../docs/architecture/list_view.md)
- `docs/architecture/single_writer_library_lock.md`
- `crates/xaga-core/src/domain.rs`
- `crates/xaga-core/src/library_writer/mod.rs`
- `crates/xaga-core/src/persistence/articles.rs`
- `crates/xaga-core/src/embeddings/row.rs`
- `src-tauri/src/commands/topics.rs`
- `src/platform/Platform.ts`
- `src/types.ts`
- `src/hooks/useRouter.tsx`
- `src/components/sidebar/Sidebar.tsx`
- `src/components/sidebar/ShortcutList.tsx`
- `src/components/list/ItemList.tsx`
