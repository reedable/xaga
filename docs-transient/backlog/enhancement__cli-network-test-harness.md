# Add a CLI network test harness

## Summary

Core tests exercise proxy behavior over loopback and model-download limits with synthetic streams, but there is no independently runnable end-to-end network diagnostic. A top-level `xaga-cli test` command is intended to host network-dependent checks without making ordinary unit tests depend on external services.

## Requirements

- Keep the normal unit-test suite independent of external network availability.
- Exercise the production client and proxy path without weakening production address policy.
- Do not expose an arbitrary-URL fetch capability in release builds unless that surface is explicitly accepted.
- Report each check and failure without mutating library or account state.

## Remaining work

- Define the first diagnostic's target, expected response, timeout, and success criteria.
- Decide whether the harness is release-visible, feature-gated, or debug-only.
- Define a test seam that can use loopback without making the production proxy permissive.
- Decide whether the harness covers model download only or also feed and proxy behavior.
- Implement the top-level command and verify offline, timeout, policy-refusal, and success outcomes.

## References

- `crates/xaga-cli/src/main.rs`
- `crates/xaga-core/src/network_proxy.rs`
- `crates/xaga-core/src/embeddings/model.rs`
