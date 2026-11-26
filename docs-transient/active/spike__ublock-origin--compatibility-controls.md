# Spike: article filtering compatibility and user controls

## Summary

Define how users understand and recover from article-filtering failures, using
the mechanisms investigated by the feasibility spike.

## Requirements

- Compare defaults, per-attempt retry, scoped persistent exceptions, and global
  controls without choosing their semantics for the user. Explain consequences
  before any relaxation; filtering exceptions must not disable network isolation.
- Distinguish tracker identification from a decision to block a request. Do not
  infer filtering success or the cause of extraction failure from missing content.
- Keep diagnostics bounded and honest about native, proxy, and page-reported
  evidence. Evaluate privacy costs before proposing request or rule details.

## Remaining work

- Define controlled fixtures for blocked ads/trackers, necessary first-party and
  third-party resources, login/subscription pages, cosmetic changes, script rules,
  false positives, unsupported rules, and extraction failure unrelated to filtering.
- Compare filtered and unfiltered loading/extraction, preserving the saved article
  on failure. Include hidden and visible loaders, redirects, favicon acquisition,
  body-image recovery, and PDF downloads; identify any untested behavior.
- Propose exception matching scope, lifetime, persistence location, redirect
  behavior, retry authorization, and effects on concurrent/in-flight loads.
- Define evidence available to explain blocked resources and active artifact
  versions, separating observation from inferred causation. Propose accessible
  controls, error/recovery behavior, and verification criteria.
- Present alternatives and evidence for the user's decision. Record approved UX
  and diagnostic contracts durably and refine the dependent controls story.
  Completion requires the comparison and reproducible compatibility findings;
  unresolved choices must remain explicit.

## References

- [Source idea](../ideas/ublock-origin-integration.md)
- [Integration feasibility](spike__ublock-origin--integration-feasibility.md)
- [Controls implementation](enhancement__ublock-origin--controls-diagnostics.md)
- [Article diagnostics](../../docs/architecture/article_loading_diagnostics.md)
- [Secure defaults](../../docs/principles/secure_by_default.md)
- [UI and UX verification](../../docs/conventions/ui_testing.md)
