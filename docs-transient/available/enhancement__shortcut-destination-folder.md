# Choose a destination folder when adding a shortcut

## Summary

Choose an existing shortcut group or the top level when adding an Entity to
Shortcuts. New shortcuts default to the top level; group creation stays in the
existing group controls.

## Requirements

- Support the article Share action and Add to Shortcuts controls in Feed, Topic,
  and Site forms, including inline Topic creation in Inspector. Commit with the
  add action or form submission; cancellation creates no shortcut.
- Offer all nested groups independently of sidebar filtering or expansion. Use
  stable group IDs and display hierarchy to distinguish duplicate group titles.
- Append directly into the selected destination in one atomic pin-store write
  through the existing writer lane and library write lock. If the destination
  became unavailable, report failure and allow reselection without writing.
- Preserve existing pin locations when editing without changing pin state,
  target uniqueness, stored order, unpin/Move to… behavior, and reading context.
  Use the existing nested-group storage format.
- Refresh shared pin state and sidebar, responsive, and native-menu projections
  through existing typed notifications. Prevent duplicate submissions, retain
  failed input for retry, and respect read-only restrictions.
- Distinguish entity-save failures from pin-save failures so retry cannot create
  duplicate entities or imply a saved entity was not saved.
- Provide keyboard access, an accessible destination label, and existing dialog
  focus behavior.

## Remaining work

- Add destination support to pin insertion across persistence, writer, native,
  platform, and provider APIs; connect the add surfaces above.
- Reuse destination-selection logic from Move to… and extend capture fixtures.
- Add backend regressions for root/nested insertion, duplicate targets, stale
  destinations, concurrent tree edits, read-only refusal, write failures, and
  preservation of existing locations and order.
- Add component/browser workflows for each add surface, duplicate-name groups,
  cancellation, partial-save retry, keyboard access, and preserved reading context.
  Reuse capture reset/readiness; run `npm run test:browser`, relevant Rust/frontend
  checks, and workspace formatting checks. Report native-menu and persistence
  behavior not verified by browser mocks.
- Update the pin contract, User Guide, and affected training.

## References

- [Pin storage and groups](../../docs/architecture/pins.md)
- [Pin persistence](../../crates/xaga-core/src/pins.rs)
- [Native pin commands](../../src-tauri/src/commands/pins.rs)
- [Pin provider](../../src/providers/PinsProvider.tsx)
- [Existing group destination selection](../../src/components/sidebar/ShortcutTree.tsx)
- [Article toolbar](../../src/components/content/Toolbar.tsx)
- [Feed forms](../../src/components/dialogs/EditFeedDialog.tsx)
- [Topic forms](../../src/components/dialogs/TopicFormFields.tsx)
- [Inline Topic creation](../../src/components/content/TopicMembership.tsx)
- [Site Settings](../../src/components/dialogs/EditSiteDialog.tsx)
- [UI testing](../../docs/conventions/ui_testing.md)
