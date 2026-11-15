# Separate loader capture and annotation commands

## Summary

Extract loader callbacks/resource capture and annotation operations into separate command modules, retaining article loading, saving, reading, and PDF access in `commands/articles.rs`.

## Requirements

- Split `commands/articles.rs` by moving loader callbacks and resource capture
  into one command module and annotation operations into another. Keep article
  loading, saving, reading, and PDF access in `articles.rs`. Preserve command
  names, payloads, IPC permissions, loader identity checks, validation, cleanup,
  diagnostics, typed notifications, and save-then-enqueue ordering.
- Preserve existing behavior, layer ownership, write authority, and serialization.
  Keep symbol visibility limited to verified consumers.
- Review all affected documentation and update architecture, conventions,
  development, User Guide, and training documents as applicable before
  completion, including moved code references.

## Remaining work

- Update command registration and internal callers for the article-command
  extractions. Verify loader callbacks still pass existing identity and IPC
  checks, capture failures preserve content and clean up resources, and
  annotation operations retain saved content, indexing, and event outcomes.
- Run `npm run lint`, `npm run typecheck`, and `npm run test:headless`.
  Add or adjust meaningful regression coverage where existing tests do not
  establish the preservation requirements; retain test strength when moving
  tests with their implementation.
- Verify native loader/resource capture and annotation operations using the linked resource verification procedure and relevant GUI tests. Report behavior not established by browser mocks, including IPC, filesystem persistence, and loader cleanup.
- Update affected documentation and code references against the final module
  layout. Complete when the specified extractions, documentation updates, and
  required verification pass.

## References

- [Article commands](../../src-tauri/src/commands/articles.rs)
- [Command registration](../../src-tauri/src/lib.rs)
- [Command permissions](../../src-tauri/src/commands/mod.rs)
- [Command surface](../../docs/architecture/security/command_surface.md)
- [Remote content policy](../../docs/architecture/security/fetching_remote_content.md)
- [Resource verification](../../docs/development/resource_verification.md)
- [Loader IPC tests](../../src-tauri/tests/loader_acl.rs)
- [Least visibility](../../docs/principles/least_visibility_principle.md)
- [Layer ownership](../../docs/architecture/layer_ownership.md)
- [Documentation conventions](../../docs/conventions/documentation.md)
- [Verification workflow](../../docs/development/verification.md)
- [Supporting hotspot evidence](../ideas/refactor-hotspots-evidence.md)
