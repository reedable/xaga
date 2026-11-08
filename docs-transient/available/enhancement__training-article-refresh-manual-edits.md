# Explain manual-edit loss on article refresh

## Summary

Teach users that refreshing an article replaces its saved body, including manual
edits, and check understanding with a quiz.

## Requirements

- Explain that refresh captures a new copy from the website without merging
  manual body edits. Added text, changed links, and manually inserted images are
  lost; annotations and tags are preserved. Advise keeping a separate copy of
  edits the user wants to retain.
- Place this explanation in product training about article refresh, using the
  actual control label and distinguishing article refresh from feed refresh.
- Add a quiz scenario involving a manually edited article with an annotation and
  tag. Ask what remains after successful article refresh, with one correct answer
  and explanatory feedback for every option.
- Preserve stable training IDs unless question meaning changes; follow the
  training's saved-progress rules when adding or revising the exercise.

## Remaining work

- Update the relevant lesson or add a focused product lesson and quiz, linking
  to the durable refresh contract.
- Verify wording and answers against implementation; run the training checks in
  its README and visually review the lesson and feedback.

## References

- [Training curriculum](../../docs/training/lessons.js)
- [Training maintenance and checks](../../docs/training/README.md)
- [Manual edits and article refresh](../../docs/architecture/security/handling_persisted_content.md#manual-edits-and-article-refresh)
- [Article persistence](../../crates/xaga-core/src/persistence/articles.rs)
