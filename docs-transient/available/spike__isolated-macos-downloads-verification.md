# Plan isolated macOS Downloads verification

## Summary

Determine how to provide an isolated macOS test account for bundled Xaga Downloads
verification under independently established undecided and denied access states,
and for desktop startup/library-upgrade smoke checks.

## Requirements

- Follow the bundled Downloads procedure in resource persistence verification.
- Preserve the primary user's TCC permissions and account settings. Keep bundle identity stable between
  establishing permission states and running each check.
- Present setup options, access requirements, and trade-offs for the user's decision.
  Complete when the user selects a setup and the subsequent verification is actionable.

## Remaining work

- Determine the account setup and access needed to establish both permission states.
- Agree on a setup and define the follow-up checks for persistence, retained content,
  staging cleanup, and startup/library-upgrade gates, including the required
  environment, disposable libraries, and bundle records.

## References

- [Bundled Downloads verification](../../docs/development/resource_verification.md#bundled-macos-downloads-authorization-check)
- [Desktop library-upgrade smoke checks](../../docs/development/library_upgrade_verification.md)
- [Decision ownership](../../docs/principles/decision_ownership.md)
