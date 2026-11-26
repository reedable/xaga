# Verify native image rendering on macOS

## Summary

Verify the `relative-image` and `cross-origin-image` WebKit cases at commit
`9dd3a33759edd8d4aaaacf41142b58caacd3d109` on
`origin/compliance__resource-download--response-routing`.

## Requirements

- The HTML control renders and exits successfully.
- In each image case, WebKit reports the image loaded with a natural width of
  16 pixels, the fixture proxy records one image request to the expected
  origin, and the download callback does not fire before the test exits.

## Remaining work

- In a logged-in macOS desktop session under the account running Cargo, fetch
  the story branch and check out the exact fixture commit in a clean worktree.
  Use the normal local build prerequisites. The fixture generates its PNG and
  serves both reserved origins through a local proxy.
- Run each command from the repository root. Each launches a separate fixture
  process.

  ```sh
  XAGA_DOWNLOAD_SMOKE_CASE=html cargo test -p xaga --test resource_download_webview --locked
  XAGA_DOWNLOAD_SMOKE_CASE=relative-image cargo test -p xaga --test resource_download_webview --locked
  XAGA_DOWNLOAD_SMOKE_CASE=cross-origin-image cargo test -p xaga --test resource_download_webview --locked
  ```

- Record the tested commit, macOS version, exit status, and full output for each
  command. A pass prints `PASS: <case>, rendered without download conversion`
  and exits zero. If a command fails or times out, record whether Cargo
  launched the fixture and the last observed fixture output. Resolve any
  failing control before interpreting an image case.

## References

- [Native fixture](../../src-tauri/tests/resource_download_webview.rs)
- [Resource verification](../../docs/development/resource_verification.md#native-image-rendering-smoke-cases)
