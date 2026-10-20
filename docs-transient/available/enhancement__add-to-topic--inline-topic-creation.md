# Create a Topic within the Add to Topic panel

## Summary

Let a user define a missing Topic inline while mapping an article or annotation
to Topics, then continue that mapping session without leaving the original
content context.

## Requirements

- Keep the **Add to Topic** control available when the library has no Topics.
- When the library has no Topics, show an inline affordance in the panel for
  creating the first Topic without requiring filter text.
- When a non-empty Topic filter has no matches, show a **Create a New Topic**
  action in place of an unexplained empty result.
- Open the Topic form inline within the **Add to Topic** panel; do not open the
  existing Topic dialog or navigate away from the panel.
- Reuse the existing Topic form fields, defaults, validation, and semantics,
  including title, description, similarity threshold, and **Add to Shortcuts**.
  Extract shared form behavior rather than maintaining two independent versions
  of those rules.
- Prefill the inline form's title with the trimmed filter text when creation is
  entered from a no-match result. Leave the title empty when creation is entered
  from the initial no-Topics state without filter text.
- Allow the user to cancel inline creation and return to the same filtered Topic
  list and mapping selections.
- For an existing article or annotation, create the Topic and its initial
  membership through the atomic create-with-member operation.
- After successful creation for an existing item, add the Topic to the panel's
  list with its checkbox checked. Keep the panel open so the user can map the
  item to additional Topics.
- For an annotation still being composed, create the Topic immediately and mark
  its checkbox as selected in the panel's staged selection. Add the annotation
  to that Topic only after annotation persistence succeeds, following the
  panel's existing staged-membership behavior.
- Applying **Add to Shortcuts** must update the new Topic's shortcut state while
  preserving the panel workflow.
- Creating a Topic from this panel must not select it in the sidebar, change the
  displayed collection, or otherwise navigate. Closing the panel returns the
  user to the article or annotation context from which it was opened.
- Keep prior checkbox selections and the filter state stable if Topic creation
  fails, show the failure through the established error path, and allow retry.
- Keep the inline form and its empty/no-match actions keyboard accessible, with
  clear labels and focus behavior.

## Remaining work

- Extract the reusable Topic form fields and state from `EditTopicDialog` while
  retaining the dialog's existing add/edit/delete behavior.
- Add explicit panel states for no Topics, filtered no-match results, and inline
  Topic creation.
- Integrate atomic create-with-member for existing items and staged selection
  for an annotation that has not yet been persisted.
- Make annotation persistence awaitable at the reader boundary and apply staged
  memberships only after that persistence succeeds.
- Refresh Topic and shortcut provider state after creation without invoking the
  normal create-dialog navigation behavior.
- Cover title prefilling, initial empty-library creation, cancellation, form
  validation, successful checked membership, continued multi-Topic mapping,
  shortcut selection, failure and retry, unsaved-annotation staging, and no
  sidebar or content navigation in component tests.
- Run frontend formatting, lint, tests, and the production build.

## References

- `src/components/content/TopicMenuButton.tsx`
- `src/components/content/TopicMenuButton.module.css`
- `src/components/dialogs/EditTopicDialog.tsx`
- `src/components/content/AnnotationTooltip.tsx`
- `src/components/content/HtmlReader.tsx`
- `src/components/content/Toolbar.tsx`
- `src/AppLayout.tsx`
- `src/hooks/useTopics.tsx`
- `src/hooks/useAnnotations.tsx`
- `crates/xaga-core/src/library_writer/mod.rs`
- `src-tauri/src/commands/topics.rs`
- `src/platform/Platform.ts`
