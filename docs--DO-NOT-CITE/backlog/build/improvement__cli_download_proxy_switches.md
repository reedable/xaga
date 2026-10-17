# Decide CLI model-download proxy overrides

## Summary

`xaga-cli embeddings download-model` uses the network proxy with plaintext HTTP and nonstandard CONNECT ports disabled. Its fixed model URL uses HTTPS on port 443, so overrides matter only if a redirect or future endpoint requires a weaker policy. The app's settings do not currently govern the CLI invocation.

## Requirements

- Keep plaintext HTTP and nonstandard CONNECT ports disabled by default.
- Do not weaken the CLI download policy without an explicit invocation and a defined use case.
- Make any override visible in CLI help and diagnostics.
- Preserve address validation and redirect validation regardless of overrides.

## Remaining work

- Decide whether model download needs proxy-policy overrides at all.
- If required, define flag scope and names and whether they apply only to `download-model` or a shared CLI network policy.
- Implement the chosen flags without reading mutable GUI settings implicitly.
- Verify default refusal and each explicit override through proxy integration tests.

## References

- `crates/xaga-cli/src/cmd/embeddings.rs`
- `crates/xaga-core/src/network_proxy.rs`
- `docs/architecture/security/fetching_remote_content.md`
