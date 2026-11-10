# Upgrade libraries from Xaga

## Summary

Verify desktop browser launching and the complete native upgrade/restart workflow
using disposable libraries.

## Requirements

- **Upgrade Xaga** opens the fixed Xaga GitHub Releases page in the default
  browser: `https://github.com/reedable/xaga/releases`.
- A confirmed **Upgrade Library** completes, restarts Xaga through controlled
  shutdown, and reopens the same selected library using normal compatibility and
  lock checks. Upgrade completion alone must not enable writes in the old process.
- A restart failure reports that conversion completed and offers restart retry;
  the library remains read-only until startup establishes write access.

## Remaining work

- Run a desktop smoke test for default-browser launching and a complete app
  upgrade/restart with a disposable older library. Verify the same library reopens
  and obtains write access only through startup checks.
- Exercise native restart failure/retry and check the displayed outcome. Browser
  fixtures cover the presentation, but do not launch a desktop browser or replace
  the running Xaga process.

## References

- [Shared converter and regression tests](../../crates/xaga-core/src/library/upgrade.rs)
- [App upgrade commands and ownership tests](../../src-tauri/src/commands/library.rs)
- [CLI subprocess parity and locking test](../../crates/xaga-cli/tests/library_upgrade.rs)
- [Browser workflows and visual captures](../../e2e/library-upgrade.spec.ts)
- [Library version contract](../../docs/architecture/library_version.md)
- [Upgrade guide](../../docs/library_upgrade.md)
