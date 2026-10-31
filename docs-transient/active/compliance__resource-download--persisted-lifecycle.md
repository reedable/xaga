# Compliance: persisted resource lifecycle

## Summary

Verify native resource staging and persistence in the bundled macOS app with
Downloads access independently established as undecided and denied. This task
requires evidence for both permission states; an unbundled native test is not
sufficient.

## Requirements

- Downloads authorization must not be required for app-generated private staging.
  Server filenames and completion paths must not choose the consumed file or
  library destination.
- Validate resources before persisting local references. Preserve available
  article content and existing favicon bytes/references when a resource fails;
  clean up the assigned staging directory after each terminal outcome.
- Record the bundle identity/signing, macOS version, permission-state setup,
  staging destination, persistence result, and cleanup for each state. Do not
  infer permission state from absence of a prompt or a successful Cargo test.
- Use an isolated test account for permission-state setup; do not reset or modify
  the user's Downloads authorization as part of automated testing.

## Remaining work

- Build or obtain the bundle and establish the two permission states.
- Follow the [bundled verification procedure](../../docs/development/resource_verification.md#bundled-macos-downloads-authorization-check)
  using a controlled endpoint accepted by the existing network policy.
- Record the results and any demonstrated limits in the durable security
  documentation. If either state cannot be established or observed, retain the
  specific verification gap rather than treating automated coverage as evidence.
- Repair demonstrated contract violations and run the affected automated suites.

## References

- [Verification guide and automated coverage](../../docs/development/resource_verification.md)
- [Validators and staging](../../src-tauri/src/resource_download.rs)
- [Download adapter](../../src-tauri/src/loader_download.rs)
- [Native persistence command tests](../../src-tauri/tests/persisted_resources_webview.rs)
- [Remote-content policy](../../docs/architecture/security/fetching_remote_content.md)
- [Persisted-content policy](../../docs/architecture/security/handling_persisted_content.md)
