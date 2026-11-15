# Warn about similar Topics during creation

## Summary

Before creating a Topic, warn about similar existing Topics in the Topic form
and inline creation in Inspector. Similarity is advisory: the user decides
whether to open an existing Topic, revise the input, or create the new Topic
anyway.

## Requirements

- On creation submission, search existing Topics by cosine similarity using the
  proposed title and description. Reuse the existing semantic search default
  similarity cutoff and show up to five matches, most similar first. The Topic
  form's similarity threshold still controls article matching, not this warning.
- If matches are found, show their titles and enough description to distinguish
  them before saving. Offer opening an existing Topic, returning to revise the
  input, and explicitly creating anyway. Describe matches as similar Topics,
  not proven duplicates. Do not impose title uniqueness or alter character
  handling.
- Opening a match navigates to that Topic without creating a Topic or changing
  its content, membership, or pins. Revising or dismissing the warning preserves
  the draft and creates nothing. Creating anyway uses the existing creation
  operation, including the requested member for inline creation and the chosen
  pin state for the new Topic. Never silently reuse or merge an existing Topic.
- If no matches are found, continue normal creation. If similarity search fails
  or the model is unavailable, explain that the check could not be completed
  and allow creation. Missing or pending embeddings can omit Topics; do not
  claim that no duplicates exist or block creation until indexing completes.
- A check and its confirmation apply to the submitted draft. Editing the title
  or description requires checking the revised draft; stale results must not
  replace newer results. Prevent repeated submission while checking or saving,
  and save only once after confirmation.
- Keep creation errors distinct from search failures, preserve failed input for
  retry, and retain read-only restrictions. Keep the current Topic IDs, storage
  format, and native creation APIs; similarity is not a persistence constraint.

## Remaining work

- Connect both creation surfaces to Topic-only similarity search and the warning
  actions. Reuse existing search and creation APIs where suitable; match behavior
  in the capture platform.
- Add focused search coverage for Topic filtering, ranking, and result limits.
  Add browser workflows for matches, no matches, opening a match, revising,
  dismissing, creating anyway, unavailable search, stale results, repeated
  submission, and save failure. Verify inline membership and new-Topic pin state
  without modifying existing Topics, plus read-only refusal.
- Update the User Guide and affected training to explain the advisory check and
  its indexing limitations.

## References

- [Organization model](../../docs/architecture/organization_model.md)
- [The user owns the data](../../docs/conventions/user_owns_data.md)
- [Semantic search commands](../../src-tauri/src/commands/embeddings.rs)
- [Similarity search defaults](../../crates/xaga-core/src/embeddings/db.rs)
- [Topic embeddings](../../crates/xaga-core/src/embeddings/pipeline.rs)
- [Topic creation commands](../../src-tauri/src/commands/topics.rs)
- [Topic form coordination](../../src/AppLayout.tsx)
- [Topic dialog](../../src/components/dialogs/EditTopicDialog.tsx)
- [Inline Topic creation](../../src/components/content/TopicMembership.tsx)
- [Capture platform](../../src/platform/capture/MockPlatform.ts)
- [UI testing](../../docs/conventions/ui_testing.md)
