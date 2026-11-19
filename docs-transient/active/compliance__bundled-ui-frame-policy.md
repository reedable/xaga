# Restrict the bundled UI frame policy

## Summary

The network policy for bundled main and settings is to deny remote connections.
Live publisher pages load in the isolated article loader. This story closes
the HTTPS frame path in the shared bundled CSP as a separate layer of
enforcement. The article reader and inline PDF viewer use Blob URLs.

## Requirements

- Remove the HTTPS allowance from `frame-src`. Keep frame sources local and
  preserve the existing app-local IPC endpoints.
- Preserve app resources, Tauri IPC in both windows, the sandboxed article
  reader, and inline Blob PDF viewing.

## Remaining work

- Remove `https:` from `frame-src` in the shared bundled policy.
- In a bundled macOS build, attempt to load a controlled HTTPS frame from
  each window and observe a CSP violation for the attempted URL with
  `effectiveDirective` equal to `frame-src`. A failed load alone could come
  from the deny-all proxy. This check establishes enforcement for the frame
  path. Broader network containment is covered by the loopback remediation.
- Verify normal IPC in both windows, article rendering, and inline PDF
  viewing. Record any platform coverage limits.

## References

- [Bundled app CSP](../../src-tauri/tauri.conf.json)
- [Reader isolation and bundled CSP](../../docs/architecture/security/handling_persisted_content.md)
- [Article loader investigation](compliance__article-loader-script-policy.md)
- [Build supply-chain strategy](../backlog/compliance__software-supply-chain.md)
- [Loopback proxy bypass remediation](../backlog/compliance__loopback-proxy-bypass.md)
