# Verify native image attachments on macOS

## Summary

Verify the two PNG attachment cases in `resource_download_webview` at commit
`a15290b1f8aa8720b0ed0bd27bcef484b01838cf` from
`origin/compliance__resource-download--response-routing`. The fixtures require
a logged-in macOS desktop session.

## Requirements

- The existing HTML and PDF controls finish successfully.
- The valid attachment reaches the image validator and is accepted. The
  truncated attachment reaches the same validator and is rejected. Both cases
  remove their staging directory.

## Remaining work

- Run these commands from the repository root with the normal local build
  prerequisites installed. Each command starts its own fixture process.

  ```sh
  XAGA_DOWNLOAD_SMOKE_CASE=html cargo test -p xaga --test resource_download_webview --locked
  XAGA_DOWNLOAD_SMOKE_CASE=pdf cargo test -p xaga --test resource_download_webview --locked
  XAGA_DOWNLOAD_SMOKE_CASE=image-attachment cargo test -p xaga --test resource_download_webview --locked
  XAGA_DOWNLOAD_SMOKE_CASE=malformed-image-attachment cargo test -p xaga --test resource_download_webview --locked
  ```

- Record the tested commit, macOS version, exit status, and fixture output for
  each case. Review any failure against the expected validator result and
  staging cleanup before adding the attachment cases to the default native
  suite.

## References

- [Resource verification](../../docs/development/resource_verification.md#native-image-attachment-smoke-cases)
- [Native fixture](../../src-tauri/tests/resource_download_webview.rs)
