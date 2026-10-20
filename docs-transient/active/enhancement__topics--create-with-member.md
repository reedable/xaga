# Create a Topic with an initial member atomically

## Summary

Add one Topic-creation operation that persists a new Topic and an existing
article or annotation as its first manual member in a single library write.

## Requirements

- Accept the same Topic metadata as ordinary Topic creation together with one
  existing Entry or Annotation.
- Mint the Topic identity and persist the Topic metadata and initial member in
  one writer-lane operation and one Topic-document write.
- Return the created Topic with its initial member present.
- Do not leave an empty Topic behind if validation or persistence of the initial
  member fails.
- Keep the Topic document as the source of truth for manual membership; do not
  introduce a second persisted membership source.
- After the write succeeds, enqueue the embedding work needed to represent both
  the Topic and its initial membership consistently.
- Carry the operation through the Tauri and TypeScript platform boundary for use
  by the **Add to Topic** panel.
- Preserve the existing empty-Topic creation operation for workflows that do
  not have an initial member.
- Reject an attempt to use this operation without a persisted Entry or
  Annotation identity; an annotation still being composed uses the existing
  staged-membership workflow instead.

## Remaining work

- Add a library-writer operation that creates a Topic whose manual-member list
  initially contains the supplied item.
- Add the Tauri command and enqueue the Topic upsert and membership indexing
  work only after persistence succeeds.
- Add the corresponding `Platform` method, Tauri implementation, and Topic hook
  operation.
- Cover successful Entry and Annotation creation, write or validation failure
  without an orphan empty Topic, returned membership, and downstream embedding
  jobs in focused tests.
- Run Rust and frontend formatting, lint, tests, and the production build.

## References

- `crates/xaga-core/src/library_writer/mod.rs`
- `crates/xaga-core/src/persistence/topics.rs`
- `src-tauri/src/commands/topics.rs`
- `src/platform/Platform.ts`
- `src/platform/TauriPlatform.ts`
- `src/hooks/useTopics.tsx`
- `docs/principles/single_source_of_truth.md`

