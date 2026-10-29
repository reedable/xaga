# Store Unread Outside Tags

## Summary

Move article reading state from the reserved `unread` tag to a dedicated
frontmatter field. Deliver the shared core change, app integration, and xaga-cli
library upgrade in one story, preserving existing read/unread behavior and library
state.

## Requirements

- Make a dedicated article frontmatter field authoritative for unread state.
  Reading state applies to Entries, not to Feeds, Topics, or annotations; collection
  unread counts remain aggregates of member Entry state.
- Name the field `is_unread`, matching the domain boolean and LanceDB column.
  Store `is_unread: true` for unread articles. Writers must omit the field for
  read articles and must never write `is_unread: false`. Readers accept an
  explicit false as read; its presence is not an error. Manually supplied
  articles without the field are treated as already read. Newly fetched articles
  are explicitly marked `is_unread: true`. Marking read removes the field.
  The domain and LanceDB boolean remain true/false.
- Separate reading state from organizational tags. Marking read or unread must
  change the dedicated field without modifying organizational tags.
- Preserve existing libraries' reading state when transitioning from the legacy
  tag representation, including entries without the legacy tag.
  During CLI conversion, preserve the state of an existing boolean `is_unread`; otherwise
  derive state from the legacy `unread` tag: presence means unread and absence
  means read. Remove the legacy marker in the same atomic file update that
  preserves or sets the dedicated state. Represent read state by omitting the
  field, including when converting an existing explicit false. Migration must
  not use `is_unread: false` as a completion marker. Files with neither the field nor the
  marker are already read and need no rewrite.
- Provide the explicit library upgrade through xaga-cli for users of the 0.x
  app. The app does not perform the library upgrade.
- Keep app and CLI reads, writes, feed counts, filters, and recovery consistent
  with the new source of truth. LanceDB's `is_unread` remains derived and rebuildable
  from article files; active cache readers must not derive reading state from tags.
- Stop synthesizing the reserved `unread` marker in LanceDB's `tags` column.
  Read and update the `is_unread` column directly for reading-state queries and
  events. Reconcile legacy cached markers during the CLI upgrade. Report success
  only after file conversion and cache reconciliation complete; report file or
  cache failures with a nonzero exit status and enough detail to retry.
- Preserve automatic mark-read behavior, explicit read/unread actions, typed state
  propagation, current-list stability, and read-only library behavior.
- Serialize authoritative writes and any library conversion through the existing
  library writer and write right. Conversion must preserve article content,
  annotations, unrelated metadata, and timestamps unrelated to reading state.
- Keep bookmark, pin, color, and general-purpose tag redesign outside this story.
  The linked domain investigation and Inspector idea provide context, not
  prerequisites for implementation.
- Release this storage change with a minor version increment. While the major
  version is 0, minor versions signal significant compatibility changes: for
  example, a 0.10 library is incompatible with 0.9 code, and 0.10 code is
  incompatible with a 0.9 library until an explicit library conversion.
  This requires the version bump and documented upgrade path, not a new
  mandatory startup rejection mechanism.

## Remaining work

- Implement the xaga-cli upgrade from legacy file tags to the dedicated field,
  removing the migrated reserved marker and preserving unrelated tags. Make
  retries safe: files already in the new representation require no rewrite,
  existing boolean state takes precedence over conflicting legacy tags, and
  reruns retry unfinished cache reconciliation even when no files need changes.
- Increment the minor release version and document the format incompatibility
  and CLI upgrade procedure.
- Update shared frontmatter parsing, serialization, Entry construction, and every
  article creation/import/update path to preserve the dedicated state.
- Replace `TagPatch::MarkRead`/`MarkUnread` with reading-state operations in the
  writer and update app commands and incidental mark-read paths.
- Update database row construction and recovery to populate `is_unread` from the
  article field without synthesizing an unread tag. Switch summary reads and
  change comparisons that currently inspect tags to the boolean column; retain
  queries already using that column. Stop mutating tags when patching unread.
- Verify CLI article operations, bookmark import, feed counts, and embedding
  rebuilds through the same core representation; update CLI integration where
  needed rather than implementing a second conversion path.
- Update durable source-of-truth and crash-consistency documentation with the
  chosen format, transition contract, and write path.
- Test legacy read/unread files, true and missing-field representations,
  normalization of existing false values to omission, absence of persisted false
  values after writes or conversion,
  interrupted and repeated conversion, read-only access, concurrent mutations,
  unrelated-tag preservation, app/CLI interoperability, state events, counts,
  filters, and recovery from files without a cache.
- Run workspace formatting and affected Rust tests, app boundary tests, and
  frontend checks appropriate to any frontend changes.

## References

- [Domain-definition investigation](../active/spike__define-tag-collection-bookmark-pin-shortcut-site.md)
- [Tags and the Inspector](../ideas/tags-and-inspector.md)
- `docs/principles/single_source_of_truth.md`
- `docs/architecture/crash_consistency.md`
- `docs/architecture/single_writer_library_lock.md`
- `docs/architecture/list_view.md`
- `crates/xaga-core/src/md.rs`
- `crates/xaga-core/src/tags.rs`
- `crates/xaga-core/src/persistence/entries.rs`
- `crates/xaga-core/src/persistence/articles.rs`
- `crates/xaga-core/src/library_writer/mod.rs`
- `crates/xaga-core/src/embeddings/row.rs`
- `crates/xaga-core/src/embeddings/db.rs`
- `crates/xaga-core/src/embeddings/pipeline.rs`
- `src-tauri/src/commands/entries.rs`
- `crates/xaga-cli/src/cmd/articles.rs`
- `crates/xaga-cli/src/cmd/bookmarks.rs`
- `crates/xaga-cli/src/cmd/feeds.rs`
- `crates/xaga-cli/src/cmd/embeddings.rs`
