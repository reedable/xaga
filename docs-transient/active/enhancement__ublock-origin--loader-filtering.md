# Filter remote article loading

## Summary

Integrate the approved filtering mechanism into remote article loading.
This dependent story is selected in active at the user's request, but is blocked
on all three spikes and the artifact contract. It is not implementation-ready.

## Requirements

- Implement the approved request/content filtering scope and supported syntax.
  Unsupported features must follow the approved handling policy rather than be
  silently presented as equivalent to uBlock Origin.
- Apply the approved policy before the first relevant navigation/request. Preserve
  isolation, secure network defaults, truthful disclosure, download validation,
  capture ownership, cancellation, and per-load teardown.
- Make failures follow the approved default/recovery contract. Do not silently
  retry with filtering relaxed or overwrite saved content on retrieval failure.
- If library storage changes are approved, require a minor-version increment
  while the app remains at major version 0 and an explicit library upgrade.

## Remaining work

- Resolve the linked prerequisites and record the chosen mechanism, coverage,
  failure semantics, exception contract, and measurable acceptance criteria in
  this story. Obtain a fresh readiness review before implementation.
- Integrate the mechanism and artifact consumer at the verified native/core
  boundaries, covering the approved hidden/visible loader, persistent-session,
  image/favicon, redirect, and download behavior.
- Verify request blocking at controlled destinations, supported cosmetic/script
  behavior, exceptions, unsupported syntax, concurrent attempts, cancellation,
  artifact failures, and preserved extraction/persistence behavior as applicable.
- Run relevant native and bundled-build checks, using the spike fixtures and
  recording coverage gaps. Update durable architecture documentation to describe
  the implementation and its demonstrated limits.

## References

- [Source idea](../ideas/ublock-origin-integration.md)
- [Feasibility prerequisite](spike__ublock-origin--integration-feasibility.md)
- [Supply-policy prerequisite](spike__ublock-origin--supply-policy.md)
- [Compatibility prerequisite](spike__ublock-origin--compatibility-controls.md)
- [Artifact prerequisite](enhancement__ublock-origin--filter-artifacts.md)
- [Controls and diagnostics](enhancement__ublock-origin--controls-diagnostics.md)
- [Network architecture and native verification](../../docs/architecture/security/fetching_remote_content.md)
- [Layer ownership](../../docs/architecture/layer_ownership.md)
