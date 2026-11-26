# Article filtering artifacts

## Summary

Implement the approved delivery and lifecycle of filtering code and data.
This dependent story is selected in active at the user's request, but is blocked
on the feasibility and supply-policy decisions. It is not implementation-ready.

## Requirements

- Implement only the selected engine/list combination, licensing obligations,
  executable-rule policy, and delivery model recorded after the spikes.
- Artifacts must be identifiable and reproducible by revision. Apply the approved
  integrity, validation, resource-limit, offline, and failure policies before use.
- For third-party retrieval, implement the approved publisher terms and protocol
  contract, including applicable caching directives, conditional requests,
  unchanged responses, rate limits, and retry delays. If runtime checks are
  selected, honor saved user frequency choices within publisher/server limits.
- Preserve existing remote-content protections and privacy behavior. If artifacts
  are account resources, coordinate writes through the existing account lock.
- If library storage changes are approved, require a minor-version increment
  while the app remains at major version 0 and the explicit upgrade contract.

## Remaining work

- Resolve prerequisites through the linked spikes, then replace conditional
  requirements with the approved artifact locations, sources, limits, notices,
  failure behavior, and acceptance criteria. Obtain a fresh readiness review
  before implementation.
- Implement packaging and loading for release-bundled artifacts, or the approved
  runtime staging/verification/atomic installation and recovery path if selected.
  Do not add a runtime updater merely because it was investigated.
- Verify the selected lifecycle, including missing, corrupt, malformed,
  unsupported, and oversized artifacts, offline startup, and concurrent access.
  For runtime installation, also verify interrupted updates, rollback, and
  contention. Document trust assumptions and remaining limitations.
- Verify retrieval using controlled responses for validators and `304`, freshness
  and no-store directives, missing/invalid metadata, redirects, rate limits,
  `Retry-After`, and transient failures according to the selected contract. For
  scheduled checks, verify user intervals, automatic-check disablement, manual
  checks, restart/offline recovery, backoff, and cross-instance coordination.

## References

- [Source idea](../ideas/ublock-origin-integration.md)
- [Feasibility prerequisite](spike__ublock-origin--integration-feasibility.md)
- [Supply-policy prerequisite](spike__ublock-origin--supply-policy.md)
- [Loader consumer](enhancement__ublock-origin--loader-filtering.md)
- [Account lock](../../docs/architecture/single_writer_account_lock.md)
- [Layer ownership](../../docs/architecture/layer_ownership.md)
