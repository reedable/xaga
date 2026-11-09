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

Before 1.0, the CLI orchestrates content conversion and cache reconciliation. It
calls core's `begin_upgrade` while holding the library write lock, before changing
source files or cache state. Only successful conversion and reconciliation permits
`complete`, which atomically writes the target as the completed version and
removes the pending marker. Interruption leaves the pending state available for
retry. Retrying cannot silently change an in-progress target. Manifest writes
reuse core's sibling-temporary-file, fsync and atomic-rename implementation.
At 1.0, responsibility for executing library-format migrations will move into
core as directed; no automatic runtime migration is implemented now.

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
