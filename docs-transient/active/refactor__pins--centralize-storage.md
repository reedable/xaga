# Centralize Pin Storage

## Summary

Finish the minor-version transition for the central library pin format after
0.10 is finalized.

## Requirements

- Increment the minor release version for the storage-model change. Keep the
  version unchanged until 0.10 is finalized, as directed.
- Synchronize the application version, package versions, and Cargo.lock through
  the release tooling. Do not distribute this format as 0.10-compatible.

## Remaining work

- After 0.10 is finalized, start the next minor development version and replace
  pending-version wording in the pin-format and upgrade documentation with the
  app’s new major.minor version.

## References

- [Pin-store contract](../../docs/architecture/pins.md)
- [Library version metadata](../../docs/architecture/library_version.md)
- [Library upgrade](../../docs/library_upgrade.md)
- [Release cycle](../../docs/development/release_cycle.md)
