# Configure stable macOS code signing

## Summary

Bundled macOS builds have no configured signing identity. WKWebView can request access to its per-application WebCrypto master key, and an unstable ad-hoc signature can cause the keychain authorization prompt to recur across builds.

## Requirements

- Use a stable signing identity for macOS bundles that share persistent WebKit data.
- Keep private keys and machine-specific identity names out of the repository.
- If builds are distributed to other Macs, satisfy the applicable Developer ID, hardened-runtime, notarization, and release-verification requirements.
- Verify the signed bundle's designated requirement and behavior rather than assuming configuration implies a stable identity.

## Remaining work

- Decide whether the target is local-only development signing or external macOS distribution.
- Provision the corresponding self-signed or Apple-issued identity outside the repository.
- Configure the build environment to select the identity and fail when a required release identity is unavailable.
- Build and install two successive bundles with the same identity and verify WebCrypto keychain access across them.
- Document the release signing and verification commands without recording secrets or a developer-specific certificate name.

## References

- `src-tauri/tauri.conf.json`
- `package.json`
- `docs/architecture/security/handling_persisted_content.md`
