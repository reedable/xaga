# Exclude a semantic class from a Topic

## Summary

Let a user give a Topic an optional **Exclude** description so articles that
match an unwanted semantic class are omitted from that Topic's automatically
discovered results.

## Requirements

- Add one optional **Exclude** description and one exclusion similarity
  threshold to a Topic definition.
- Continue using the Topic description and its existing threshold to identify
  candidate semantic matches.
- Apply hard-exclusion semantics: omit a candidate when its similarity to the
  **Exclude** description meets or exceeds the exclusion threshold.
- Apply **Exclude** only to automatically discovered semantic matches. Preserve
  manually added articles and annotations because their membership records an
  explicit user decision.
- Keep existing Topics fully backward compatible: a Topic without a non-empty
  **Exclude** description must return the same results it does today.
- Present the field as **Exclude**, not as a generative-model "negative prompt,"
  and explain that it describes articles the Topic should omit.
- Expose an independently adjustable exclusion threshold alongside the existing
  inclusion threshold.
- Persist authored exclusion text and its threshold with the Topic. Keep any
  exclusion embedding derived and recoverable from that durable definition.
- Recompute the derived exclusion embedding when the exclusion text changes,
  remove it when the text is cleared, and rebuild it during embedding recovery.
- Apply the same inclusion and exclusion semantics wherever Topic membership
  and counts are derived so the sidebar count and opened Topic list agree.

## Remaining work

- Extend the Topic domain model, Markdown front matter, parser, writer, Tauri
  commands, TypeScript types, and platform methods with the optional exclusion
  definition and threshold.
- Choose and document a derived-vector representation that keeps the Topic's
  inclusion and exclusion embeddings distinct without treating either as an
  additional source of truth.
- Extend Topic embedding jobs, checksum behavior, deletion, and recovery to
  maintain the exclusion vector.
- Update Topic list and count queries to evaluate the inclusion threshold first
  and then remove matches at or above the exclusion threshold, while retaining
  explicit manual members.
- Add **Exclude** and its threshold to the shared Topic form used by the Topic
  dialog and inline **Add to Topic** creation flow.
- Cover absent and cleared exclusions, positive-only matches, excluded matches,
  matches close to both descriptions, manual members that match the exclusion,
  edits, recovery, counts, filters, and persisted-library compatibility.
- Run Rust and frontend formatting, lint, tests, and the production build.
- Document Topic inclusion, exclusion, manual-membership precedence, and derived
  embedding ownership in the durable architecture documentation.

## References

- `crates/xaga-core/src/domain.rs`
- `crates/xaga-core/src/md.rs`
- `crates/xaga-core/src/persistence/topics.rs`
- `crates/xaga-core/src/embeddings/row.rs`
- `crates/xaga-core/src/embeddings/service.rs`
- `crates/xaga-core/src/query/topics.rs`
- `src-tauri/src/commands/topics.rs`
- `src/components/dialogs/EditTopicDialog.tsx`
- `docs-transient/available/enhancement__add-to-topic--inline-topic-creation.md`
- `docs/principles/single_source_of_truth.md`

