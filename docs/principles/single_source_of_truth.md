# Single Source of Truth

**Every datum has exactly one source of truth. The producers of a derived store are enumerated; a write from outside that enumeration is, by definition, drift.**

## The assignments in this app

- **`.atom` files** own feed membership (the entry id-set per feed).
- **Entry `.md` files** own entry content and state (`is_unread` frontmatter,
  optional article `color`, organizational tags, annotations). Reading state is independent of organizational
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
- **LanceDB** is the queryable read layer that mirrors the above. The durability invariant — every column is reconstructible from the files — is what makes LanceDB a cache, not a second source of truth.

## The producer rule

LanceDB has exactly two producers: the command layer (live, from user intent) and recovery (reconciliation). A third producer of cache writes is, by definition, drift — the clearest on-course check.

## What follows from it

- **Derived stores are disposable.** Recovery rebuilds them from the source files; a rebuild of a derived structure is decoupled from unrelated gates (the reverse-index rebuild runs every recovery regardless of the checksum/embedding gate, because a membership-only change never touches the entry `.md` — `crates/xaga-core/src/recovery/`).
- **Writes land on the source of truth**; mirrors follow. The ordering between the two is the subject of `docs/architecture/crash_consistency.md`.
- **A derived index serves only the direction its source cannot.** Forward (topic → members) is read from the topic `.md` itself; the derived `topics` column serves only the reverse direction (entry/annotation → topics; the `list_entry_topics` command, `src-tauri/src/commands/topics.rs`).
- **Making anything other than the source of truth authoritative loses information.** The alternative — moving membership into each member's frontmatter — would have dropped insertion order, member type, and the annotation `reference`, and would have drifted because the membership write path writes only the topic file and would never have touched it.
