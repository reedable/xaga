# Evaluate a publisher JavaScript control for the article loader

## Summary

Consider a user option to disable publisher JavaScript in the article loader.

## Requirements

- Keep live article loading in the article loader.
- Use one app-wide publisher JavaScript setting, not per-page permissions.
  Publisher JavaScript is disabled by default. The user can opt in or opt out
  at any time. Changes apply to newly opened loader windows; do not interrupt
  executing scripts or change the policy of already-open loaders.
- Keep the JavaScript choice distinct from remote image loading and saving
  images for offline viewing.
- Disabling publisher JavaScript must preserve Xaga's injected capture scripts.

## Remaining work

- From a macOS desktop session, run
  `cargo test -p xaga --test loader_lifecycle_webview --test persisted_resources_webview --locked`.
  Verify publisher script execution matches the preference and that the native
  loader still delivers article capture, image readback, deadline results, and
  loader IPC with publisher JavaScript disabled.

## References

- [Article loader privacy controls](compliance__article-loader-script-policy.md)
- [WebKit content JavaScript control](https://developer.apple.com/documentation/webkit/wkwebpagepreferences/allowscontentjavascript)
- [Native loader policy](../../docs/architecture/security/handling_remote_content.md#publisher-javascript)
