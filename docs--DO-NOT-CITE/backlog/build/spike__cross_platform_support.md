# Define cross-platform support

## Summary

Xaga supports and verifies macOS only. Windows and Linux behavior is undefined for security-critical proxy routing, persistent webview stores, webview history deletion, file locking, and the webview behavior matrix; packaging and platform integration also require separate decisions.

## Requirements

- Select target platforms and minimum versions before implementation.
- Do not ship a loader unless all remote traffic is forced through equivalent SSRF controls.
- Preserve single-writer and stale-view guarantees using each platform's file and rename semantics.
- Make unsupported privacy or persistence features explicit rather than silently ignoring them.
- Verify CSP, sandboxing, focus isolation, storage isolation, PDF handling, logging, packaging, and signing on every supported webview engine.

## Remaining work

- Verify `proxy_url` enforcement on WebView2 and WebKitGTK.
- Verify PDF response-to-download routing on WebView2 and WebKitGTK and define
  explicit unsupported behavior where equivalent routing is unavailable.
- Define persistent or intentionally ephemeral loader storage for non-macOS platforms.
- Replace the macOS-specific `clear_webview_history` path with platform implementations or explicit unsupported behavior.
- Verify `FileLock`, rename-over-lock, and path-identity behavior on Windows and Linux; design a non-Unix path-identity check where required.
- Define platform-native logging, transparency, packaging, update, and signing behavior.
- Present the supported-platform and feature-parity options with their security and maintenance costs for decision.

## References

- `docs/principles/secure_by_default.md`
- `docs/architecture/security/fetching_remote_content.md`
- `docs/architecture/single_writer_library_lock.md`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`
- `src-tauri/src/article_loader.rs`
- `src-tauri/src/commands/articles.rs`
- `crates/xaga-core/src/fs.rs`
