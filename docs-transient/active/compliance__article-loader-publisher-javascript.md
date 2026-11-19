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

- Use WebKit's `WKWebpagePreferences.allowsContentJavaScript` control to disable
  publisher scripts while retaining Xaga's `WKUserScript` injection. The local
  Wry implementation already uses this control for `javascript_disabled`.
  Verify capture, image readback, deadlines, and IPC in the native loader with
  publisher JavaScript disabled.
- Explain the setting's scope, default, and when changes apply in Settings.

## References

- [Article loader privacy controls](compliance__article-loader-script-policy.md)
- [WebKit content JavaScript control](https://developer.apple.com/documentation/webkit/wkwebpagepreferences/allowscontentjavascript)
- [Local Wry WebKit implementation](../../../wry/src/wkwebview/mod.rs)
