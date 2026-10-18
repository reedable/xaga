# Compliance: persisted resource lifecycle

## Summary

Verify staging, reuse, recovery, refresh, and read-only behavior for downloaded
article resources and favicons.

## Requirements

- Private staging must remain safe when macOS Downloads permission is undecided
  or denied.
- Persisted resources must be reusable and recover when their file is missing.
- Refresh must handle new, legacy, and edited favicon entries consistently.
- Read-only operation must not attempt storage mutation.
- Main-window rendering must remain local-only.
- Failures must preserve captured article content and clean up staged files.

## Remaining work

- Exercise staging with macOS Downloads permission undecided and denied.
- Test reuse, missing-file recovery, refresh, and read-only behavior for new,
  legacy, and edited favicon entries.
- Fix persistence, recovery, or cleanup failures found by those cases.
- Update durable persisted-content and remote-content documentation.

## References

- [Download consumers](../../src-tauri/src/loader_download.rs)
- [Validators and staging](../../src-tauri/src/resource_download.rs)
- [Native tests](../../src-tauri/tests/resource_download_webview.rs)
- [Remote content policy](../../docs/architecture/security/fetching_remote_content.md)
- [Persisted-content policy](../../docs/architecture/security/handling_persisted_content.md)
