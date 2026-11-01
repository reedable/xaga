# Upgrading a library

Use the CLI matching the application version you intend to run. The current
0.14.0-dev CLI upgrades libraries to **0.14**, including tag identities, color-backed
article bookmarks, Site pin support, central pin storage, and the reading-state
conversion below.

## Reading-state format (0.10)

Xaga 0.10 stores Entry reading state in the `is_unread` article frontmatter field,
separately from tags. This is a library-format compatibility change: 0.9 code
cannot safely use a 0.10 library, and 0.10 code requires explicit conversion of a
0.9 library. The app does not run the upgrade automatically. Version differences, absent
version metadata, or an incomplete upgrade select read-only mode at startup.

1. Close Xaga and other processes using the library.
2. Back up the library before changing its format.
3. Run the CLI matching your destination app version (currently 0.14.0-dev):

   ```sh
   xaga-cli --data-dir /path/to/library library upgrade
   ```

4. After the command reports completion, open the library with the matching app.

The command requires the exclusive library write lock. It preserves existing
boolean `is_unread` values; otherwise presence of the legacy `unread` tag means
unread, and absence means read. It removes the legacy marker. Unread articles
store `is_unread: true`; read articles omit the field. Explicit false is accepted
on reads and normalized to omission on writes. Manually supplied articles
without the field are read; newly fetched Entries explicitly start unread.

Conversion preserves article content, annotations, unrelated metadata and
timestamps using sequential atomic file replacement under the library lock.
Only migration-owned frontmatter fields are edited; unrelated scalar text
(including numeric-looking IDs), unrelated comments, and body bytes are retained. Edited
fields may change formatting. Unsupported `images` frontmatter is removed under
the existing writer rule. Organizational tags retain their values; legacy bookmark markers convert to
article color as described below.

The command reports conversion and cache errors and exits nonzero when work is
unfinished. Resolve the reported errors and run the same command again. Retries
preserve files already converted and retry cache reconciliation even when no
files need rewriting. No embedding-model download is required. Existing cache
rows are reconciled in batches; missing rows remain for normal recovery to rebuild.

Do not reopen a partially converted library with either app version. Finish the
upgrade, or restore the full backup to return to 0.9.

## Central pin format (0.11)

Library version **0.11** stores all Entry, Feed, and Topic pins in
[`pins.yaml`](architecture/pins.md), independently of entity content. This format
was introduced in **0.11.0-dev** and is not 0.10-compatible.

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

## Site pins (0.12)

**0.12.0-dev** adds `type: site` targets identified by normalized `hostname` to the
shared pin store. Close library users, back up the library, and run the same
`library upgrade` command with the matching destination CLI. The existing flat
pins keep their representation and order; Site membership remains derived and requires no new
entity files or cache rows. Cache pin reconciliation applies only to Entry,
Feed, and Topic targets. Existing Site pins are retained on interrupted retries.

Site pins require library version **0.12** or later. Successful conversion
advances the manifest to the running CLI’s major/minor version, currently **0.14**.
Version-mismatched libraries open read-only until explicitly upgraded; the app
never relabels them automatically. Returning to an older app format requires
restoring the complete backup.

## Article bookmark colors (0.13)

Library version **0.13** stores an optional article `color` frontmatter field:
`red`, `orange`, `yellow`, `green`, `blue`, or `violet`, matching the annotation
swatches. A marked article is bookmarked; an unmarked article omits `color`.
There is no bookmark boolean in domain payloads or the cache.

Close library users, back up the library, and run the matching **0.13.0-dev** CLI
with the upgrade command above. Existing valid colors survive. A legacy
`bookmark` tag without a color becomes `color: blue`; the legacy marker is
removed in the same atomic file replacement. Unrelated tags, reading state,
annotations, timestamps, custom scalar text, and exact body bytes survive.
Invalid color values produce an error without replacing that file.

The converter reconciles existing cached colors in bounded batches, drops the
old bookmark boolean column and its index, and creates bitmap indexes on color
and kind. Missing rows are rebuilt by normal recovery. Each file replacement is
atomic and safe to retry; conversion or cache errors retain the pending version
and make the CLI fail. Retry the same command until successful before opening
the library. Older apps cannot safely edit this format.

Bookmark imports without colors mark articles Blue, including existing Entries.
Export includes every marked article, with a `COLOR` attribute for Xaga
round trips. Other Netscape bookmark consumers may discard color while retaining
membership. Reading and importing do not interpret organizational tags as colors.

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

Migration targets use the running CLI’s major/minor version, currently **0.14**;
there is no separately assigned library revision number.

## Entry tag identities (0.14)

The 0.14 upgrade registers existing article labels in `tags.yaml` and resolves
LanceDB assignments to stable UUIDs. Existing 0.13 article files retain their
readable labels and other content. Tags named `unread`, `bookmark`, and `pinned`
are organizational labels in a 0.13 library and are preserved.

Run the 0.14 CLI's `library upgrade` with the app closed. Definitions are durable
before cache conversion. An interrupted or failed upgrade retains its pending
version; rerun the same command to reuse IDs and finish reconciliation. No app
startup conversion is performed. The storage format includes rename, merge, and
deletion records for subsequent tag functionality; see [Tag storage](architecture/tags.md).
