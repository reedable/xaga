# Private/loopback feeds: unreachable, with no opt-in mechanism

Status (2026-07-03): no decision made. Recorded so the judgment itself is
tracked — today the unreachability is a side effect of the address policy,
not a documented decision either way. The architecture doc
(`docs/architecture/security/fetching_untrusted_content.md`, "Limits") states the current
behavior.

Facts known at time of writing:

- The SSRF proxy's address policy (`is_blocked`,
  `src-tauri/src/ssrf_proxy.rs`) passes only globally-routable addresses;
  every consumer dials through the proxy. Feeds or articles resolving to
  private/loopback addresses (intranet feeds, localhost dev servers) are
  therefore unreachable, and there is no setting that changes this.
- The address policy is the security-critical core of the SSRF defense; any
  opt-in necessarily reopens part of that surface for the opted-in targets.
- The proxy resolves and pins addresses itself, so an allowlist checked at
  the same point would not be bypassable by DNS rebinding: a rebinding host
  either resolves into the allowlisted range (reaching only what the user
  allowed) or is refused.

Options:

- **Record as permanent design decision: no mechanism.** Simplest and
  safest; users with intranet feeds are out of scope. The decision should
  then be stated in the architecture doc as intent, not listed as a limit.
- **Opt-in allowlist of specific hosts or CIDRs** (analogous to
  `allowed_domains`), empty by default, enforced inside `validate_target`
  next to the existing policy. Progressive trade-off per
  docs/principles/secure_by_default.md; the UI must state the consequence:
  every allowlisted range is SSRF-reachable by any hostile feed, not only by
  the feed the user had in mind.
- **Loopback-only opt-in** (e.g. localhost dev feeds), which is a narrower
  version of the allowlist with a smaller reopened surface — LAN hosts
  (routers, printers) stay unreachable.

Whether any mechanism is wanted at all is the first decision; scope and
values follow from it.
