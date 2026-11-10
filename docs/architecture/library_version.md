# Library version metadata

Each library stores its own version in `<library>/library.yaml`. The version uses
the application's `major.minor` convention; patch, prerelease, and build suffixes
are excluded. Multiple libraries may have different stored versions at the same
time. There is no independent storage counter or global current-library version.

```yaml
format_version: "0.10"
```

`xaga-core` owns the path, typed model, parsing, serialization, atomic writes,
and migration-state transitions through the public `library` API. Callers supply the
application's major/minor at runtime. App startup obtains it from the application's
package information; the CLI supplies its own application major/minor. No build
script examines libraries or enforces a separate package-version assertion.

The manifest owns the last completed version. An explicit upgrade first writes a
pending target while retaining that version:

```yaml
format_version: "0.9"
migration:
  target_version: "0.10"
```

A library without a manifest is unversioned. Its initial upgrade uses
`format_version: null` with the pending target; absence is never interpreted as a
specific historical version. Opening an empty directory initializes a new library at
the running app version while holding the library lock. Only the lock file may
already exist; directories containing any other files remain unversioned.

Core implements explicit content conversion and cache reconciliation through
`library::upgrade_library`, shared by the app and CLI. Callers supply the runtime
target version and retain the exclusive library lock for the entire operation.
Core calls `begin_upgrade` before changing source files or cache state. Only
successful conversion and reconciliation permits `complete`, which atomically
writes the target as the completed version and removes the pending marker.
Interruption retains the pending state. A newer app can advance the pending
target and reapply the migrations from the original completed source version;
retries with an older app cannot lower a pending target.
Manifest writes use sibling temporary files, fsync and atomic rename.

The app exposes typed library status: read-only cause, completed version, pending
target and app version. An older or unversioned library offers **Upgrade Library**;
an interrupted upgrade targeting this or an earlier version offers
**Resume Library Upgrade**. A newer library
offers **Upgrade Xaga**, opening the fixed GitHub Releases page. A pending target newer than the running app requires updating Xaga. Invalid metadata and lock contention
explain the problem without offering a format upgrade for those causes.

The app requires explicit confirmation after explaining full backup, restoration,
older-app compatibility and existing readers. It reserves the operation before
starting a blocking worker, acquires the exclusive lock and rechecks metadata.
Progress is typed and polled without blocking the UI. Duplicate submissions and
library switching cannot start competing operations. Ordinary writes remain
refused: conversion does not construct a runtime writer. Failures retain read-only
mode with retry feedback. No embedding model is needed.

On success the app saves the same selected library through the account settings
funnel, drains its writer if any, and spawns a successor. Spawn or settings failure
is reported separately from conversion success and permits restart retry. Normal
startup rechecks compatibility and lock ownership; completion never enables writes
in the old process. Opening a library still never upgrades it automatically.

## Runtime opening

At startup the app reads the selected library's manifest before creating its main
webview or starting a writer or embedding service:

- A completed version matching the app's major/minor may acquire the library
  write lock. Lock contention still produces a read-only instance.
- An empty directory is initialized at the app version and may open writable.
- A different version, missing manifest in a nonempty directory, or incomplete
  migration opens read-only.
- Malformed or unreadable metadata also opens read-only; startup reports the
  metadata error without replacing the manifest.

Having a writer determines read-write mode; its absence means read-only. The
same command checks and disabled controls apply for every read-only cause. Such startup
performs no migration, source writes, cache initialization or recovery. Existing
cache reads remain available; an absent cache is not created by a read-only open.
Library switching restarts the app and checks the newly selected library again.

The `library info` CLI command reports the selected library's stored version and
pending target without taking a write lock, reading the corpus, or changing the
manifest. These app startup checks do not add a new access restriction across all CLI commands.

Manifest reads are bounded to 64 KiB, use strict YAML without anchors, and reject
nonregular manifest files, unknown fields and malformed major/minor strings.
These parse errors preserve the original file. Explicit upgrade failures remain
errors; the read-only fallback applies to app startup.

The advisory library lock excludes writers, not existing readers. Close library
users before an explicit upgrade. Already released binaries that do not read the
manifest cannot enforce its meaning.
