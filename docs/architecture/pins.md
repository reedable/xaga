# Library pin store

`<library>/pins.yaml` owns pin membership. Its path is derived by
`library::pins_path`; the Rust `Pins` and `PinTarget` types define the parsed
contract, and the platform exposes that model to the shared UI provider.

```yaml
pins:
  - type: entry
    id: abc123
  - type: feed
    id: feed456
  - type: topic
    id: topic789
```

Identity is target type plus ID. Sequence order is preserved; pinning an existing
target is a no-op, and unpinning removes only that typed reference. Titles and
counts are resolved from current targets rather than duplicated in the file.
Missing stores mean empty membership. Malformed stores, duplicate targets,
anchors, unknown fields, and unsafe IDs are refused before mutation. Reads are
bounded to 12 MiB. IDs cannot contain path separators, parent traversal, or NUL.

Mutations serialize the parsed model; YAML formatting and comments are not retained.

All pin writes use one Interactive library-writer lane, under the existing
library write lock. The complete file is replaced through a sibling temporary
file and atomic rename. A pin edit does not modify the target entity. Library
read-only gating applies to generic and entity-specific pin commands; target
navigation remains available in read-only libraries.

The command emits typed `pin:changed` intent after committing storage, then the
UI rereads current central membership. Request generations prevent earlier
responses from overwriting later snapshots; delayed notifications also read
current membership. Feed/Topic controls and the article toolbar project membership
from the shared provider. Article shortcuts resolve source-file metadata without
waiting for cache recovery. The `is_pinned` cache column is a disposable mirror;
recovery derives it from the central store, never legacy tags or categories.

Unresolved references remain in storage and appear in both sidebar and responsive
navigation as compact “Shortcut unavailable” rows. Opening a row shows its type
and identity in a details dialog, with Retry and Remove shortcut actions. The
initial explanation says “We can’t find this topic/feed/article,” describing
resolution failure without claiming a cause: bulk lists can omit unreadable targets. Retry reports user-safe read errors where available, refreshes
the corresponding shared list, and remains available in read-only libraries.
Removal is disabled in read-only libraries and affects only the pin. Initial
Topic/Feed loading uses skeleton rows instead of unavailable-target errors.
Restoring a target resolves the same reference on the next target refresh.
Removing a pin preserves the current list and reading scope.
Article shortcuts suppress counts and contribute zero to unread sorting; the
built-in All items target stays first. Alphabetical/unread sorting is presentation
state and never rewrites the sequence.

Site pins use `type: site` and a normalized `hostname`, not an entity ID:

```yaml
pins:
  - type: site
    hostname: example.com
```

The persistence boundary validates bare DNS hostnames and applies the Site
identity rules for case, international domains, and terminal dots. Pin payloads
already carry Site identities: the article URL’s one-leading-`www.` alias rule
is not reapplied to them. For example, an article on `www.www.example.com`
derives to `www.example.com`, which remains unchanged across pin reads/writes.
URLs, paths, ports, credentials, and IP literals are refused. Hostnames never
become filesystem path components. Equivalent Site references cannot occur twice.
Site pin changes use the same writer lane and `pin:changed` event as other pins;
there is no Site entity row or cache pin column to update.

Site Settings shows a read-only hostname and an **Add to Shortcuts** checkbox.
Membership comes from the shared pin store each time the dialog opens. Read-only
libraries can navigate pinned Sites and inspect Settings, but cannot change pins.
Pinned Sites use current derived total/unread counts and existing Site navigation.
Once the Site list has loaded, a pinned hostname absent from that projection is
still a valid empty Site with zero counts, not an unavailable target. Its pin
survives cache recovery and out-of-band changes. Initial Site loading uses the
same skeleton rows as other collection shortcuts. Alphabetical/unread sorting
includes Sites while **All items** stays first.

Site targets extend the library format to **0.12**, implemented by **0.12.0-dev**.
Existing 0.11 libraries require the explicit
[CLI upgrade](../library_upgrade.md#site-pins-012). Flat Entry/Feed/Topic pins
retain their representation and order. Older applications cannot edit 0.12
libraries; their version mismatch selects read-only mode. No Site membership is
persisted or converted, and Site references never create cache entity rows.

Future nested groups use `type: group`, stable `id`, `title`, and an ordered
`children` sequence. Groups are not implemented by the flat-store contract.

The recovery walk constructs a membership set once, so its per-entity membership
lookup does not scan the ordered sequence. Article-shortcut resolution reads one
membership snapshot and each referenced article file, without repeatedly parsing
the store. Normal pin mutations and each newly transferred legacy pin still read
or serialize the complete pin sequence; the converter streams one source document
at a time. Those costs are governed by the number of pins and the 12 MiB store
bound, rather than a full-corpus pin discovery scan during normal navigation.
