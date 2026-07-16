# CLI network-test harness (top-level `test` command)

Owner direction 2026-07-11 (re-labeled improvement, not compliance, per owner): unit tests make no network calls — network-involving tests belong in a CLI harness, runnable independently of CI.

Decided so far:

- Top-level `test` command in xaga-cli (not under `embeddings`).
- Unit tests stay network-free (the cap/checksum logic is pinned by synthetic-chunk tests in `crates/xaga-core/src/embeddings/model.rs`).

Open (owner to rule when picked up):

- **Scope**: plain unproxied client, or the full proxied chain (`ProxyConfig.is_blocked_fn` is injectable; a loopback-permitting constructor in core would exercise the production path minus the address filter).
- **Gating**: always compiled (user-runnable diagnostic; ships an arbitrary-URL fetch surface), cargo feature (CI-only; build-matrix split), or `debug_assertions` (no release presence, no feature plumbing).
- Which network activities it covers beyond the download cap (feed fetches, proxy behavior, …).

Facts that constrain the design: the SSRF proxy refuses loopback by policy, so a proxied test cannot reach a 127.0.0.1 server without the permissive constructor; the production download path's URL and cap are compile-time constants — the harness needs a core seam (`pub` diagnostic fn) that takes base + cap as parameters.
