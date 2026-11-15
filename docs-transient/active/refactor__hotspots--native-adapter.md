# Isolate native payload schemas and enforce their import boundary

## Summary

Extract native payload schemas and decoders from `TauriPlatform` and enforce their isolation from UI layers with a local Oxlint rule.

## Requirements

- Extract native payload schemas and decoding helpers from `TauriPlatform.ts`
  into an internal adapter directory. Keep native calls, subscriptions, and the
  `Platform` implementation in `TauriPlatform`. Preserve runtime validation,
  domain conversion, current error behavior, and the existing Platform API.
- Enforce schema/decoder visibility with a local Oxlint rule enabled as an error
  in `npm run lint`. Resolve imports to the internal directory and allow access
  only from the native adapter and its tests. Reject outside imports, including
  type-only and dynamic imports, and re-exports exposing these internals through
  another module. UI layers and their tests must use domain types and Platform
  contracts.
- Preserve existing behavior, layer ownership, write authority, and serialization.
  Keep symbol visibility limited to verified consumers.
- Review all affected documentation and update architecture, conventions,
  development, User Guide, and training documents as applicable before
  completion, including moved code references.

## Remaining work

- Extract adapter schemas and decoders and update the native adapter imports.
  Retain adapter regressions for accepted/rejected payloads, domain kinds,
  event conversion, and errors. Add the import-boundary rule through the
  existing local lint plugin with positive and negative fixtures covering
  allowed adapter/test access and forbidden direct, type-only, dynamic, and
  re-export access; verify the rule runs in the existing release lint check.
- Run `npm run lint`, `npm run typecheck`, and `npm run test:headless`.
  Add or adjust meaningful regression coverage where existing tests do not
  establish the preservation requirements; retain test strength when moving
  tests with their implementation.
- Update affected documentation and code references against the final module
  layout. Complete when the specified extractions, documentation updates, and
  required verification pass.

## References

- [Native adapter](../../src/platform/TauriPlatform.ts)
- [Adapter regressions](../../src/platform/TauriPlatform.test.ts)
- [Platform contract](../../src/platform/Platform.ts)
- [Type identity](../../docs/architecture/type_identity.md)
- [Local lint plugin](../../lint-rules/index.js)
- [Lint configuration](../../.oxlintrc.json)
- [Release lint check](../../.github/workflows/release.yml)
- [Least visibility](../../docs/principles/least_visibility_principle.md)
- [Layer ownership](../../docs/architecture/layer_ownership.md)
- [Documentation conventions](../../docs/conventions/documentation.md)
- [Verification workflow](../../docs/development/verification.md)
- [Supporting hotspot evidence](../ideas/refactor-hotspots-evidence.md)
