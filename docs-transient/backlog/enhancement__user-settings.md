# Improvement: fetch-policy settings UI

## Summary

Expose existing fetch-policy settings that currently require manual edits to
`settings.json`.

## Requirements

- Support `http_allowed` (default `false`).
- Support `max_redirects` (range 0–10, default `3`; `0` refuses the first
  redirect).
- Support `referer_allowed` and `gzip_allowed` (both default `false`).
- Support `connect_nonstandard_ports_allowed` (default `false`; HTTPS `CONNECT`
  remains limited to port 443 when disabled).
- Support `loader_timeout_secs` and `feed_timeout_secs` (both default 10 seconds).
- Support `feed_max_body_bytes` (default 10 MiB).
- Preserve the existing defaults, ranges, and enforcement. Explain the privacy,
  security, and compatibility consequence of changing each setting. The default
  redirect limit of 3 is an existing secure-by-default exception.

## Remaining work

- Add get/set commands and settings UI controls following the existing settings
  command and provider patterns.
- Verify persistence, runtime application, validation, and displayed consequences
  for each exposed setting.

## References

- [Settings commands](../../../src-tauri/src/commands/settings.rs)
- [Settings provider](../../../src/providers/SettingsProvider.tsx)
- [Privacy settings UI](../../../src/components/settings/PrivacySettings.tsx)
- [Remote-content policy](../../../docs/architecture/security/fetching_remote_content.md)
- [Secure by default](../../../docs/principles/secure_by_default.md)
