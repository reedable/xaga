# Spike: filtering licensing, trust, and update policy

## Summary

Establish the licensing obligations and trust/update choices for article-filtering
code and data before selecting a production combination.

## Requirements

- Evaluate code, dependencies, filter lists, and tracker datasets separately.
  Cover uBlock Origin, adblock-rust, SafariConverterLib, Tracker Radar, Disconnect,
  EasyPrivacy, and AdGuard filters from the source idea. Record exact revisions
  and authoritative license/terms sources; flag unresolved interpretation.
- Identify maintainers, publishers, distribution/update channels, transitive
  dependencies, and the trust remaining after integrity checks. Matching a hash
  does not establish that an artifact or its parser is safe.
- Compare release-bundled reviewed artifacts with runtime updates, distinguishing
  executable code, script-based rules, and declarative data. No runtime update
  channel or executable-rule policy is approved by this story.
- Research publisher-friendly retrieval for every third-party list source,
  including its terms of service, distribution guidance, permitted endpoints,
  attribution/client-identification requirements, and request-rate limits. Cover
  build/release retrieval as well as runtime checks when applicable.

## Remaining work

- Produce a candidate/combinations matrix covering redistribution, modification,
  notices, source obligations, list-specific terms, and unresolved legal questions.
- Assess parser/compiler attack surfaces, malicious lists, resource exhaustion,
  dependency and publisher compromise, and the authority of script-based rules.
- Compare freshness, reproducibility, offline operation, authentication of
  expected revisions, review, update-request privacy, rollback, revocation,
  interrupted installs, and last-known-good behavior for each delivery model.
- Research HTTP retrieval and caching against current primary protocol sources
  and each publisher's guidance: `Last-Modified`/`If-Modified-Since`,
  `ETag`/`If-None-Match`, `304 Not Modified`, `Cache-Control`, `Expires`, `Date`,
  `Age`, `Vary`, and `Retry-After`, including rate-limit and temporary-error
  responses. Define which headers and list-supplied expiry/update metadata apply,
  their precedence, validator persistence, and behavior when values are absent,
  invalid, contradictory, or change across redirects. Respect applicable response
  directives and publisher restrictions rather than treating them as hints to
  poll more often.
- Compare user-controlled check intervals, disabling automatic checks, and manual
  checks with publisher minimum intervals, freshness, retry deadlines, backoff,
  and jitter. Specify how scheduling survives restart, offline periods, and
  concurrent instances without bursts or duplicate checks. Distinguish checking
  for an update from downloading and activating one; user controls must not
  bypass publisher restrictions or server-directed retry delays.
- Evaluate validator tracking, retained metadata, update-request disclosure, and
  any publisher-required identification against Xaga's current no-validator and
  truthful-disclosure policies. Present any list-specific policy exception for
  approval and durable documentation; do not silently change feed-fetch behavior.
- Present two or three viable policies with costs, risks, and evidence for the
  user's choice, coordinated with the feasibility spike. Record approved choices
  in durable documentation and refine the artifact implementation story. If no
  candidate is acceptable or a decision remains pending, report that explicitly.
  Completion is the evidence-backed policy comparison, not assumed approval.

## References

- [Source idea and upstream candidates](../ideas/ublock-origin-integration.md)
- [Integration feasibility](spike__ublock-origin--integration-feasibility.md)
- [Artifact implementation](enhancement__ublock-origin--filter-artifacts.md)
- [Update-frequency controls](enhancement__ublock-origin--controls-diagnostics.md)
- [Secure defaults](../../docs/principles/secure_by_default.md)
- [Network disclosure policy](../../docs/architecture/security/fetching_remote_content.md)
- [Conditional-request privacy](../../docs/architecture/security/performance_tradeoff.md)
- [Existing model trust analysis](../../docs/architecture/security/embedding_model_trust.md)
- [Account resource coordination](../../docs/architecture/single_writer_account_lock.md)
