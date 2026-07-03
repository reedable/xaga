# Cross-platform support (pending)

Make xaga run on platforms beyond macOS. Today the app is developed and verified on macOS
only. This item inventories the known platform-specific code, dependencies, and assumptions
(verified 2026-07-02 unless marked otherwise), so the porting work starts from facts.

## Known platform-specific points

1. **Named webview data stores are compile-time macOS-only.**
   `supports_named_data_stores()` is `cfg!(target_os = "macos")`
   (`src-tauri/src/article_loader.rs`); per-domain persistent `WKWebsiteDataStore`
   identifiers require macOS 14+ — the app's `bundle.macOS.minimumSystemVersion`
   (`src-tauri/tauri.conf.json`). On any other platform, every loader webview is incognito:
   the `allowed_domains` persistent-store feature is silently inoperative.
2. **The SSRF proxy routing depends on a macOS feature flag.**
   `WebviewWindowBuilder::proxy_url` takes effect on macOS only with the `macos-proxy`
   feature (`src-tauri/Cargo.toml`). This is a load-bearing security control
   (`docs/architecture/security/fetching_untrusted_content.md`); whether `proxy_url` is honored on
   Windows (WebView2) and Linux (WebKitGTK) has not been verified — if it is not, the loader
   on those platforms has no SSRF protection and must not ship without an equivalent.
3. **Private-API transparency is macOS-only.** The `macos-private-api` feature +
   `"macOSPrivateApi": true` enable the transparent webview used for the launch white-flash
   fix (`src-tauri/Cargo.toml` comments, `tauri.conf.json`). Other platforms need their own
   answer or accept the flash.
4. **`clear_webview_history` hardcodes a macOS path.** It removes
   `~/Library/WebKit/<product>` (`src-tauri/src/commands/articles.rs`). On other platforms
   the webview state lives elsewhere (WebView2/WebKitGTK have their own layouts); as written
   the command is a no-op there — a privacy feature that silently does nothing.
5. **Webview engine differences are unverified.** Every webview-dependent behavior was
   verified on WKWebView only: CSP enforcement in the bundle (including the reader-script
   hash), the sandboxed reader iframe, the hidden-loader focus suppression (written against
   WKWebView-level show/focus callbacks), incognito semantics, and PDF handling. Windows
   (WebView2/Chromium) and Linux (WebKitGTK) each need this matrix re-verified.
6. **The library lock's advisory `flock` semantics.** `FileLock`
   (`crates/xaga-core/src/fs.rs`) relies on handle-tied advisory locking; the equivalent
   behavior on Windows (`LockFileEx` semantics via whatever the implementation uses) has not
   been verified — the single-writer guarantee
   (`docs/architecture/single_writer_library_lock.md`) must hold on every shipped platform.
7. **Code signing is per-platform.** `macos_code_signing.md` covers macOS identity; Windows
   (Authenticode) and Linux (none/distro-specific) are separate work.
8. **Already cross-platform by construction** (no action expected, listed to bound the work):
   model dir via `dirs::data_dir()` (`crates/xaga-core/src/embeddings/model.rs`), settings
   and library paths via Tauri's `app_config_dir`/`app_data_dir`, TLS via rustls (no system
   OpenSSL dependency), and the SSRF proxy itself (a plain TCP listener on 127.0.0.1).

## Shape of the work

Per-platform verification of points 2, 5, and 6 decides feasibility; points 1, 3, 4, and 7
are feature-parity decisions (equivalent, degrade-with-notice, or platform-exclusive —
`docs/principles/secure_by_default.md` applies where the degraded form is less secure, e.g.
points 2 and 4). Which platforms to target, and in what order, is undecided.
