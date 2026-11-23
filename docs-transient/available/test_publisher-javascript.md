# Verify publisher JavaScript in the macOS article loader

## Summary

Run the native WebKit fixtures for the **Allow publisher JavaScript** preference.
The verification covers publisher script execution and the capture and recovery
paths used when the preference is off.

## Requirements

- Run both fixtures from Terminal in a logged-in macOS desktop session
  under the operator's own account. Use macOS 14 or later and a checkout
  containing commit `14867b8`, which introduced the preference and test coverage.
- Both fixtures must exit successfully. The lifecycle fixture checks that
  publisher scripts run only with explicit opt-in, that Xaga's injected scripts
  still return HTML and read back article images through loader IPC, and that
  deadlines preserve captured HTML when favicon acquisition stalls. The persisted
  resources fixture checks that image recovery still acquires, validates, and
  saves images with publisher JavaScript disabled.
- Use the fixtures' temporary resources. They do not require a personal library
  or a change to Xaga Settings.

## Remaining work

1. Ensure Xcode Command Line Tools, Rust and Cargo, and the Protocol Buffers
   compiler are available. Check with `xcrun --find clang`, `cargo --version`,
   and `protoc --version`. From the repository root, confirm the checkout
   contains the implementation with `git merge-base --is-ancestor 14867b8 HEAD`
   (exit status 0).
2. Run these commands separately so a failure in one fixture does not hide the
   result of the other.

   ```sh
   cargo test -p xaga --test loader_lifecycle_webview --locked
   cargo test -p xaga --test persisted_resources_webview --locked
   ```

3. Record the tested commit (`git rev-parse HEAD`), macOS version and build
   (`sw_vers`), both exit statuses, and any failing assertion or diagnostic.
   If a fixture stalls for two minutes after Cargo prints its `Running tests/`
   line, interrupt it and record the last output as a blocked run.
   Close this story once both fixtures pass.

## References

- [Native loader policy](../../docs/architecture/security/handling_remote_content.md#publisher-javascript)
- [Lifecycle fixture](../../src-tauri/tests/loader_lifecycle_webview.rs)
- [Persisted resources fixture](../../src-tauri/tests/persisted_resources_webview.rs)
- [Native resource verification](../../docs/development/resource_verification.md#automated-coverage)
