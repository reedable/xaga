# Restore the inline Topic creation browser tests

## Summary

Two inline Topic creation cases are temporarily disabled after CI failures.
Both passed five local runs with CI settings.

## Requirements

- Restore coverage that inline confirmation creates a separate Topic with the
  requested article membership and top-level shortcut.
- Restore coverage that changing articles discards a pending inline similarity
  check and that repeated submissions while the check is pending are ignored.
- Verify both restored cases in repeated runs on hosted CI.

## Remaining work

- Inspect each failing CI assertion and its Playwright trace. If a trace is
  unavailable, reproduce the failure on hosted CI with the affected case
  enabled on an investigation branch.
- Fix the causes and re-enable both cases with their behavior assertions intact.

## References

- [Topic creation browser spec](../../e2e/topic-creation.spec.ts)
- [Browser failure diagnostics](../../docs/development/browser_tests.md#failures-and-ci)
