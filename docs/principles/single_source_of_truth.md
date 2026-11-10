# Single Source of Truth

**Each value has one authoritative source. Other stored copies must be rebuildable from it, and their permitted write paths must be listed. A write outside those paths violates this rule.**

## The assignments in this app

- **`.atom` files** own feed membership (the entry id-set per feed).
- **Entry `.md` files** own entry content and state (`is_unread` frontmatter,
  optional article `color`, organizational tags, annotations and their direct readable-name tag assignments). Reading state is independent of organizational
  tags: `is_unread: true` means unread; an absent field or explicit false means
  read. Writers omit read state rather than persisting false. Collection unread
  counts aggregate member Entry booleans.
- **Topic `.md` files** own topic membership — ordered, typed, carrying the annotation `reference`. The membership write path writes only the topic file (`crates/xaga-core/src/persistence/topics.rs`).
- **Pins** are owned by the library's ordered `pins.yaml` store of typed target
  references. Feed categories, Topic/article tags, and LanceDB rows do not own
  membership. Displayed pin state resolves central references against current
  targets; unavailable references remain stored until explicitly unpinned.
  See the [pin-store contract](../architecture/pins.md).
- **`library.yaml`** owns each library’s completed major/minor version and any
  pending migration target. Core owns the file API; an application version is a
  runtime input for comparison, never a replacement for reading the selected
  library’s metadata. See [library version](../architecture/library_version.md).
- **`tags.yaml`** owns library tag identities, names, and pending rename/merge/delete
  state. Entry files own readable assignments; indexed tag IDs derive from both.
  See [Tag storage](../architecture/tags.md).
- **LanceDB** is the queryable read layer that mirrors the above. Every column must be reconstructible from the files. This makes LanceDB a cache rather than an independent source of saved data.

## The producer rule

During normal operation, LanceDB has two permitted producers: command handlers, which apply saved user actions, and recovery, which reconciles the database with files. Adding another runtime database write path violates this rule. Explicit library upgrades use the shared core converter under the exclusive library lock, with ordinary runtime writers absent. This migration boundary reconciles derived rows from authoritative files before completing the manifest; app and CLI call the same implementation.

## What follows from it

- **Derived stores are disposable.** Recovery rebuilds them from the source files; rebuild decisions must depend on the relevant source data. The reverse index is rebuilt on every recovery regardless of whether content needs embedding, because a membership-only change does not touch the Entry `.md` file (`crates/xaga-core/src/recovery/`).
- **Writes land on the source of truth**; mirrors follow. The ordering between the two is the subject of `docs/architecture/crash_consistency.md`.
- **Use the derived index for reverse lookup.** Forward (topic → members) is read from the topic `.md` itself; the derived `topics` column serves only the reverse direction (entry/annotation → topics; the `list_entry_topics` command, `src-tauri/src/commands/topics.rs`).
- **Making anything other than the source of truth authoritative loses information.** The alternative — moving membership into each member's frontmatter — would have dropped insertion order, member type, and the annotation `reference`, and would have drifted because the membership write path writes only the topic file and would never have touched it.
