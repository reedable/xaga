# Restore the inline Topic creation browser test

## Summary

The inline Topic confirmation case intermittently fails on CI and is
temporarily disabled. It passed five local runs with CI settings.

## Requirements

- Restore coverage that inline confirmation creates a separate Topic with the
  requested article membership and top-level shortcut.
- Verify the restored case in repeated runs on hosted CI.

## Remaining work

- Inspect a failing CI assertion and its Playwright trace to identify where
  the flow stalls. If a trace is unavailable, reproduce the failure on hosted
  CI with the case enabled on an investigation branch.
- Fix the cause and re-enable the case with its membership and shortcut
  assertions intact.

## References

- [Topic creation browser spec](../../e2e/topic-creation.spec.ts)
- [Browser failure diagnostics](../../docs/development/browser_tests.md#failures-and-ci)
