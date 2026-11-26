# Article filtering controls and diagnostics

## Summary

Expose the approved filtering controls, compatibility recovery, and diagnostics.
This dependent story is selected in active at the user's request, but is blocked
on the compatibility spike and the approved loader/artifact contracts. It is not
implementation-ready.

## Requirements

- Implement the approved default, exception scope/lifetime, persistence, and
  retry behavior. Explain the consequences of relaxing filtering before a user
  decides, and preserve the separate network-security controls.
- Display saved settings only after a successful write, using the existing
  settings funnel and conflict handling where settings are account-owned.
- If runtime list checks are selected, let users control check frequency and
  disable automatic checks under the approved supply policy. Define manual-check
  behavior and explain publisher/server timing limits. Distinguish last check,
  installed revision, next eligible check, and deferred/failed checks without
  suggesting that a check necessarily downloads or activates an update.
- Extend the existing bounded diagnostic system with approved observations and
  artifact identity. Preserve attempt attribution, redaction, and observation
  limits; do not claim that filtering caused an otherwise unexplained failure.
- If library storage changes are approved, require a minor-version increment
  while the app remains at major version 0 and an explicit library upgrade.

## Remaining work

- Resolve prerequisites and specify exact controls, accessible state, exception
  matching, save failures, retries, diagnostic fields, and acceptance criteria.
  Obtain a fresh readiness review before implementation.
- Resolve update-frequency choices with the supply-policy spike, including
  defaults, allowed intervals, manual checks, retry delays, and saved scheduling
  behavior. Verify the controls and timing feedback against backend enforcement.
- Implement controls and recovery through the Platform boundary and native
  coordination. Verify scope across redirects, concurrent loads, and in-flight
  attempts against the approved contract.
- Add meaningful browser workflow coverage without hardcoded product copy and
  run the browser suite. Verify persistence, policy enforcement, cancellation,
  diagnostic attribution/redaction, and actual WebKit behavior separately.
- Update affected architecture documentation, User Guide, and training to match
  implemented controls and consequences. Report native or bundled verification
  gaps through the required verification handoff.

## References

- [Source idea](../ideas/ublock-origin-integration.md)
- [Compatibility prerequisite](spike__ublock-origin--compatibility-controls.md)
- [Update-policy prerequisite](spike__ublock-origin--supply-policy.md)
- [Loader contract](enhancement__ublock-origin--loader-filtering.md)
- [Artifact contract](enhancement__ublock-origin--filter-artifacts.md)
- [Diagnostics contract](../../docs/architecture/article_loading_diagnostics.md)
- [Settings coordination](../../docs/architecture/single_writer_account_lock.md)
- [UI testing](../../docs/conventions/ui_testing.md)
- [Verification handoff](../../docs/development/verification.md#verification-handoff)
