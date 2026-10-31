# Upgrading a library to 0.10

Xaga 0.10 stores Entry reading state in the `is_unread` article frontmatter field,
separately from tags. This is a library-format compatibility change: 0.9 code
cannot safely use a 0.10 library, and 0.10 code requires explicit conversion of a
0.9 library. The app does not run the upgrade automatically. Version differences, absent
version metadata, or an incomplete upgrade select read-only mode at startup.

1. Close Xaga and other processes using the library.
2. Back up the library before changing its format.
3. Run the 0.10 CLI against that library:

   ```sh
   xaga-cli --data-dir /path/to/library library upgrade
   ```

4. After the command reports completion, open the library with Xaga 0.10.

The command requires the exclusive library write lock. It preserves existing
boolean `is_unread` values; otherwise presence of the legacy `unread` tag means
unread, and absence means read. It removes the legacy marker. Unread articles
store `is_unread: true`; read articles omit the field. Explicit false is accepted
on reads and normalized to omission on writes. Manually supplied articles
without the field are read; newly fetched Entries explicitly start unread.

Conversion preserves article content, annotations, unrelated metadata and
timestamps using sequential atomic file replacement under the library lock. YAML
formatting and comments are not retained; unsupported `images` frontmatter is removed under the existing
writer rule. The stored bookmark marker and organizational tags retain their values.

The command reports conversion and cache errors and exits nonzero when work is
unfinished. Resolve the reported errors and run the same command again. Retries
preserve files already converted and retry cache reconciliation even when no
files need rewriting. No embedding-model download is required. Existing cache
rows are reconciled in batches; missing rows remain for normal recovery to rebuild.

Do not reopen a partially converted library with either app version. Finish the
upgrade, or restore the full backup to return to 0.9.

## Upcoming central pin format

The next minor format stores all Entry, Feed, and Topic pins in
[`pins.yaml`](architecture/pins.md), independently of entity content. The corresponding app minor-version
bump is intentionally pending until the 0.10 release is finalized; the
committed version has not yet been incremented. Do not distribute this format as a
0.10-compatible library change.

Using the CLI built with the central-pin implementation, close all library users,
back up the library, and run the same `xaga-cli --data-dir /path/to/library library
upgrade` command. It transfers article/Topic `pinned` tags and only app-namespaced
Feed `pinned` categories. Publisher categories and unrelated tags, body content,
annotations, metadata, and timestamps survive. The existing reading-state
conversion also remains available.

Every transferred reference is committed before its old marker is removed.
Rerunning after interruption preserves already-transferred and unresolved pins,
then retries conversion and cache reconciliation. A missing store means no pins in
normal runtime access; the application never silently migrates old markers.
Wait for successful conversion before opening the library. Older application
versions cannot safely edit the new pin representation; restoring the complete
backup is the way to return to the old format.

## Inspecting the library version

Run:

```sh
xaga-cli --data-dir /path/to/library library info
```

This reads that library's `library.yaml` and reports its stored `major.minor`
version and any incomplete migration target. It does not create a cache or change
metadata. A missing manifest is reported as unversioned, without guessing a
historical version from file shapes. Several libraries may have different stored
versions. App startup compares only the selected library with the app's own
major/minor at runtime.

The manifest is managed by xaga-core. Before conversion, the CLI asks core to
record the pending target; only successful file conversion and cache reconciliation
allows core to advance the completed version. Failure leaves the pending marker
for a retry. A versioned library already at the target needs no conversion.

Mismatched, unversioned and partially upgraded libraries open read-only in the app.
Malformed or unreadable metadata also opens read-only and reports its error.
Startup never rewrites the version to make it match. Read-only opening does not
perform recovery or initialize an absent cache. Explicit CLI upgrade errors still
produce a nonzero exit status; close library users before upgrading.

The application version remains unchanged until 0.10 is finalized, as directed.
The unreleased pin-format change still requires the pending minor-release bump.
Once that release version changes, new migration targets use its major/minor;
there is no separately assigned library revision number.
