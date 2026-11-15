# Separate EntityList data handling and presentation

## Summary

Extract list loading and event reconciliation into a hook, and header and row presentation into components, while retaining navigation and focus coordination in `EntityList`.

## Requirements

- Extract `EntityList.tsx` data loading and event reconciliation together into
  a hook, and header controls and row presentation into separate components.
  Keep selection, history, virtualization, and keyboard/focus coordination owned
  by `EntityList`. The hook must preserve cancellation, request-version guards,
  subscription cleanup, and reconciliation of state events with in-flight reads.
  Preserve list membership and reading context under the ListView contract,
  filters, sorting, responsive controls, accessibility, and read-only behavior.
- Preserve existing behavior, layer ownership, write authority, and serialization.
  Keep symbol visibility limited to verified consumers.
- Review all affected documentation and update architecture, conventions,
  development, User Guide, and training documents as applicable before
  completion, including moved code references.

## Remaining work

- Extract the list data hook and header/row components, updating
  `EntityList.tsx` to compose them. Retain component coverage for stale reads,
  event reconciliation, and subscription cleanup. Run browser workflows for
  filters, history, keyboard navigation, virtualized focus return, responsive
  collection selection, and preserved reading context.
- Run `npm run lint`, `npm run typecheck`, and `npm run test:headless`.
  Add or adjust meaningful regression coverage where existing tests do not
  establish the preservation requirements; retain test strength when moving
  tests with their implementation.
- Run `npm run test:browser` for list workflows, including filters, history, keyboard navigation, virtualized focus return, responsive selection, and reading context. Report native focus behavior not established by browser fixtures.
- Update affected documentation and code references against the final module
  layout. Complete when the specified extractions, documentation updates, and
  required verification pass.

## References

- [EntityList](../../src/components/list/EntityList.tsx)
- [List regressions](../../src/components/list/EntityList.test.tsx)
- [ListView contract](../../docs/architecture/list_view.md)
- [UI testing](../../docs/conventions/ui_testing.md)
- [Browser workflows](../../docs/development/browser_tests.md)
- [Least visibility](../../docs/principles/least_visibility_principle.md)
- [Layer ownership](../../docs/architecture/layer_ownership.md)
- [Documentation conventions](../../docs/conventions/documentation.md)
- [Verification workflow](../../docs/development/verification.md)
- [Supporting hotspot evidence](../ideas/refactor-hotspots-evidence.md)
