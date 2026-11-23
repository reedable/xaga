# Verify remote image permissions in the macOS article loader

## Summary

Verify native WebKit image requests and image recovery on the
`compliance__article-loader-script-policy` branch. The build account has no
macOS desktop session, and the native fixtures stalled before reporting any
requests.

## Requirements

- Run from a logged-in macOS 14 or later desktop session under the operator's
  own account. Install Xcode Command Line Tools, Rust and Cargo, and the
  Protocol Buffers compiler.
- Use `origin/compliance__article-loader-script-policy`, which contains the
  implementation and these instructions together.
- Both fixtures must exit successfully. Use their temporary libraries and
  servers without changing a personal library or its settings.

## Remaining work

1. Fetch and check out the published story branch. Run the fixtures from that
   checkout.
2. Run the two fixtures separately from the repository root.

   ```sh
   cargo test -p xaga --test loader_lifecycle_webview --locked
   cargo test -p xaga --test persisted_resources_webview --locked
   ```

3. In the lifecycle fixture, verify that the article text is captured when
   remote images are off, with zero requests for the image element, CSS
   background, declared favicon, and fallback favicon. In the positive control,
   verify that browser image requests occur while offline saving is off, and
   that Xaga does not acquire a favicon for saving. Record whether WebKit
   requests a favicon in the positive control. If it makes none, report the
   zero requests in the blocked case as observed behavior for that runtime.
4. In the persisted resources fixture, verify that turning either image control
   off prevents network recovery and writes. With both on, verify acquisition
   and local readback. Confirm that a previously saved image is reused without
   another network request.
5. Record the macOS version, exit statuses, and fixture request logs.
   On failure, include the assertion and observed request path.

## References

- [Resource persistence verification](../../docs/development/resource_verification.md)
- [Loader lifecycle fixture](../../src-tauri/tests/loader_lifecycle_webview.rs)
- [Persisted resources fixture](../../src-tauri/tests/persisted_resources_webview.rs)
