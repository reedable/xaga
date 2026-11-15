# Prevent duplicate shortcut group paths

## Summary

Require unique full paths for shortcut groups while retaining the existing
requirement that group titles may repeat. For example, `Work / Research` and
`Personal / Research` are allowed; two groups at `Work / Research` are not.

## Requirements

- A path is the sequence of group titles from the top level through the group
  itself. Compare titles exactly after existing trimming; keep existing
  character handling. A slash within a title is part of that title.
- Reject creating, renaming, or moving a group to a path used by another group.
  Exclude the group itself when checking. Saving an existing duplicate unchanged
  fails; changing its path to a unique one succeeds.
- Reject new conflicts caused by descendant path changes or by removing a group
  and promoting its children. Allow renaming or moving a parent when it merely
  preserves existing conflicts among descendants. For example, renaming `Work`
  containing two `Research` groups to `Projects` succeeds.
- Existing duplicates remain readable and do not block unrelated edits. Leave
  their correction to the user.
- Check edits against the current tree through the existing pin writer, for
  both UI and native calls. Two conflicting saves must not both succeed.
- On rejection, save nothing, explain the conflicting path, and keep the user's
  input or selected destination available for correction. A rejected drop leaves
  the group in place.
- Keep the current storage format, group IDs, target uniqueness, operation
  ordering, read-only behavior, and browsing context.

## Remaining work

- Add core validation and conflict reporting to creation, rename, Move to…,
  drag and drop, and removal. Match that behavior in the capture platform.
- Add backend tests for the requirements above at root and nested levels,
  including moving within the same parent, correcting existing duplicates,
  simultaneous conflicting saves, and unchanged storage after rejection.
- Add browser coverage for conflict correction, parent edits with existing
  duplicate children, and preserved browsing context.
- Update the pin contract, User Guide, and affected training to describe the
  agreed naming rule and conflict behavior.

## References

- [Pin storage and groups](../../docs/architecture/pins.md)
- [Pin persistence](../../crates/xaga-core/src/pins.rs)
- [Library writer](../../crates/xaga-core/src/library_writer/mod.rs)
- [Native pin commands](../../src-tauri/src/commands/pins.rs)
- [Group creation and movement](../../src/components/sidebar/ShortcutTree.tsx)
- [Capture platform](../../src/platform/capture/MockPlatform.ts)
- [User Guide](../../docs/user_guide.md)
- [UI testing](../../docs/conventions/ui_testing.md)
