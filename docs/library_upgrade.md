# Upgrading a library

Use Xaga matching the library format you want to run. The current app and CLI
use format **0.17**. Opening a library does not upgrade it automatically.

## Upgrade in Xaga

1. Back up the entire library folder and close other Xaga windows.
2. Open the library. A read-only explanation appears before model initialization
   or download. For an old directory layout, choose **Upgrade**, **Choose another Library**,
   or **Quit**. Conversion must finish before browsing; **Continue read-only**
   remains available for compatible new-layout libraries when applicable. The sidebar's **Read-only** button reopens this
   dialog.
3. Read the target version and backup instructions before choosing **Upgrade**.
   Older versions of Xaga will not be able to edit the upgraded library. To undo the
   upgrade, restore the complete backup.
4. Keep Xaga open while it reports conversion and cache reconciliation progress.
   Upgrading does not initialize or download a model. Closing Xaga cannot roll
   back committed changes.
5. After success, Xaga restarts and reopens the same library. Startup checks its
   version and write lock again; another writer can keep it read-only.

If conversion fails, inspect **Upgrade problems**, resolve the reported problems,
and choose **Retry Library Upgrade**. Independent files continue converting when
one source fails. Required failures leave the original completed version and
pending target recorded in `library.yaml`. This or a newer Xaga version can retry
from that original version, including after interruption.

For a newer library or a pending target newer than this app, **Upgrade Xaga**
opens the [Xaga Releases page](https://github.com/reedable/xaga/releases) in your
default browser. Metadata errors and unavailable write locks explain their causes
without offering a format upgrade.

Restart failure is reported separately from conversion success. Choose
**Restart Xaga** again or close and reopen the same library. Xaga attempts to
restore read-only database access before restarting, so failed conversion or
restart can retain counts and memberships. If recovery fails, it identifies the
unavailable features; unknown counts are hidden and Topic membership offers
**Retry** rather than displaying unchecked Topics.

## Upgrade from the CLI

The CLI uses the same core conversion and cache reconciliation as the app.
Close other library users, back up the complete library, and run the CLI matching
your destination app version:

```sh
xaga-cli --data-dir /path/to/library library upgrade
```

Wait for completion before reopening the library. Failures exit nonzero; resolve
the errors and rerun with the same destination version.

## Reading-state format (0.10)

Xaga 0.10 stores Entry reading state in the `is_unread` article frontmatter field,
separately from tags. This is a library-format compatibility change: 0.9 code
cannot safely use a 0.10 library, and 0.10 code requires explicit conversion of a
0.9 library. The app does not run the upgrade automatically. Version differences, absent
version metadata, or an incomplete upgrade select read-only mode at startup.

1. Close Xaga and other processes using the library.
2. Back up the library before changing its format.
3. Run the CLI matching your destination app version (currently 0.17.0-dev):

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
advances the manifest to the running caller’s major/minor version, currently **0.17**.
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

The manifest is managed by xaga-core. Before conversion, the app or CLI asks core to
record the pending target; only successful file conversion and cache reconciliation
allows core to advance the completed version. Failure leaves the pending marker
for a retry. A versioned library already at the target needs no conversion.

Mismatched, unversioned and partially upgraded libraries open read-only in the app.
Malformed or unreadable metadata also opens read-only and reports its error.
Startup never rewrites the version to make it match. Read-only opening does not
perform recovery or initialize an absent cache. Explicit CLI upgrade errors still
produce a nonzero exit status; close library users before upgrading.

Migration targets use the running caller’s major/minor version, currently **0.17**;
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

## Nested pin groups (0.15)

**0.15.0-dev** adds nested group nodes to `pins.yaml`. Close library users,
back up the library, and run the matching CLI:

```sh
xaga-cli --data-dir /path/to/library library upgrade
```

For a 0.14 library, the upgrade validates the pin store and advances `library.yaml`
to **0.15**. Flat target encoding, sequence order, and file bytes remain unchanged;
no article scan or cache rewrite is needed. Earlier libraries also receive their
required conversions described above. The application does not upgrade automatically.

Malformed pin stores leave the upgrade incomplete and the library read-only.
Resolve the reported error and retry the same command; an interrupted metadata
upgrade is safe to resume. Successful retries do not duplicate pins or create
groups. Older applications open a 0.15 library read-only because its version
no longer matches. Restore the complete backup to return to the previous format.

## Annotation tags (0.16)

**0.16.0-dev** adds optional readable-name `tags` sequences to individual annotation
frontmatter records. Missing tags mean untagged. Close library users, back up the
library, and run the matching CLI's `library upgrade` command above.

Upgrading a 0.14 or 0.15 library validates both pin and tag stores and advances
the manifest without rewriting article files or creating a cache. Existing
rename, merge, and delete transitions retain their definitions, aliases, and
parent Entry-ID pending batches byte-for-byte. The updated runtime resumes them
using article and annotation assignments. Recovery rebuilds derived annotation
tag IDs; no content re-embedding is required for a tag-only difference.

Invalid stores leave the target pending and the library read-only. Correct the
reported problem and retry with the same destination CLI. Interruption preserves
the pending target; retry neither changes tag identities nor drops unfinished
transitions. Older applications open 0.16 libraries read-only.

## Library directory layout (0.17)

The 0.17 upgrade moves supporting data into visible `metadata/` and runtime
files into `.xaga/`. See [Library directory layout](architecture/library_layout.md)
for the complete structure and backup contract. Existing database files move
without discarding vectors. Shared favicon references are updated in Markdown;
article body image references and primary content directory locations remain.

This layout conversion is required before browsing an old-layout library,
including read-only browsing. Failed or interrupted conversion keeps that gate
in place; retry the upgrade, select another library, or quit. Conversion still
requires your confirmation and never runs automatically. This requirement is
specific to the layout transition, not every future format upgrade.
