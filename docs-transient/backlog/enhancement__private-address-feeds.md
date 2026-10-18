# Improvement: private-address feeds

## Summary

Decide whether Xaga should support feeds hosted on loopback or private networks.
This task concerns the current RSS/Atom fetch path, which uses reqwest through the
network proxy. The proxy unconditionally blocks loopback addresses, and no setting
provides an exception.

WebKit, on the other hand, behaves differently because it has a known loopback
proxy bypass, but Xaga does not currently use WebKit to fetch RSS/Atom feeds.

## Requirements

Private, loopback, and link-local addresses remain blocked by default. Any opt-in
must identify what it exposes, avoid granting unrelated article content access to
the permitted service, and preserve DNS/address validation across redirects. The
known WebKit loopback bypass is a containment defect, not an opt-in mechanism.

## Remaining work

- Establish whether intranet feeds, locally hosted development feeds, or another
  concrete use case warrants an exception.
- Choose among no exception, a loopback-only opt-in, or an allowlist of specific
  hosts or address ranges. Record the supported use case and exposure of the
  selected option.
- If an exception is selected, scope it to the intended feed retrieval rather
  than the shared article-loader policy where possible. Define hostname and CIDR
  matching, DNS rebinding behavior, redirect handling, settings, and user-facing
  disclosure.
- Implement and verify the selected behavior after feed retrieval moves to the
  article loader. Tests must observe both the proxy and controlled destinations.

## References

- [Network proxy](../../../crates/xaga-core/src/network_proxy.rs)
- [Remote-content policy](../../../docs/architecture/security/fetching_remote_content.md)
- [Secure by default](../../../docs/principles/secure_by_default.md)
- [Loopback containment task](compliance__loopback-proxy-bypass.md)
- [Feed retrieval migration](compliance__eliminate-reqwest.md)
