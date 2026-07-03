# CONNECT-to-port-80: plaintext tunneling rests on client convention

Status (2026-07-03): no decision made. Recorded so the gap is tracked; the
architecture doc (`docs/architecture/security/fetching_untrusted_content.md`, "Limits")
states the current behavior.

Facts known at time of writing:

- The `http_allowed` setting (default `false`) is enforced in the SSRF proxy
  against **absolute-form** `http://` requests only
  (`src-tauri/src/ssrf_proxy.rs`).
- A `CONNECT host:80` request would be address-validated like any CONNECT,
  then tunneled opaquely — the proxy cannot see that the bytes inside are
  plaintext HTTP. The plaintext-HTTP refusal would not fire.
- Today no consumer does this: reqwest uses CONNECT only for https targets,
  and the loader webview sends plain http as absolute-form. Enforcement
  therefore rests on client convention, not on the proxy.
- The proxy does not currently restrict which ports CONNECT may target at
  all; any port on an allowed address can be tunneled.

Options:

- **Refuse CONNECT to port 80 when `http_allowed` is off.** Closes the
  specific port-80 hole at the single enforcement point. Does not cover
  plaintext on any other port; TLS-on-port-80 (nonstandard but legal) would
  be refused.
- **Port allowlist for CONNECT (e.g. 443 only, or 443 plus a setting).**
  Broader: bounds the tunnel surface to ports where TLS is expected.
  Consequence: feeds served on nonstandard https ports stop working unless
  the allowlist is user-extensible.
- **Record as accepted limit, no enforcement.** Zero code; the gap remains
  convention-dependent and must be re-examined whenever a new consumer is
  pointed at the proxy.

The choice among these, and any port values, are open decisions.
