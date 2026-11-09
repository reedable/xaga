# Expose fetch settings and clarify Settings organization

## Summary

Expose existing fetch-policy settings and organize controls by their consequences
for the user.

## Requirements

- Add the following controls, retaining current defaults and enforcement:

  | Panel | Settings | Defaults and limits |
  | --- | --- | --- |
  | Privacy | `referer_allowed`, `gzip_allowed` | Both false |
  | Security (new) | `http_allowed`, `connect_nonstandard_ports_allowed` | Both false; CONNECT restricted to port 443 when the latter is off |
  | Security | `max_redirects` | 3; range 0–10; 0 refuses the first redirect |
  | General | `loader_timeout_secs`, `feed_timeout_secs` | Both 10 seconds |
  | General | `feed_max_body_bytes` | 10 MiB |

- Use user-facing labels and explain each control's privacy, security,
  compatibility, or resource consequence, actual retrieval scope, and when a
  change takes effect. Do not imply feed-client redirect/header controls govern
  WebKit article loads.
- Keep the existing panel placements, with these copy and grouping changes:
  - Rename **Switch** to **Switch library**.
  - Explain that startup feed refresh contacts subscribed websites automatically.
  - Put **Summary / Reader** under Appearance's **Reading** subsection and label
    it as the default article view.
  - Rename **Allowed Domains** to **Keep website sessions**, explaining that
    cookies and site data persist for the listed websites.
  - Group feed-size and import-size limits together in General.
- Follow existing Settings save and validation behavior: immediate switches;
  numeric saves on blur, Enter, and window close, with inline validation. Failed
  writes retain the last saved setting and the unsaved input for retry.

## Remaining work

- Preserve existing numeric semantics: timeouts and feed-size limits are `u64`
  values loaded without range clamping. Validate whole-number input and lossless
  IPC conversion; do not introduce new policy limits in this UI change.
- Extend settings commands, platform/provider APIs, and panels; update the User
  Guide and affected training to match the final labels and behavior.
- Test persistence, runtime application, validation, save failures, and Settings
  close behavior. Run relevant frontend/backend checks and `npm run test:browser`;
  report retrieval behavior not verified by browser mocks.

## References

- [Settings panels](../../src/components/settings/Settings.tsx)
- [Settings commands](../../src-tauri/src/commands/settings.rs)
- [Settings provider](../../src/providers/SettingsProvider.tsx)
- [Remote-content policy](../../docs/architecture/security/fetching_remote_content.md)
- [Secure by default](../../docs/principles/secure_by_default.md)
