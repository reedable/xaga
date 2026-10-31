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
navigation with their type, identity, a cannot-be-found indication, and a manually
operable unpin action. Restoring a target resolves the same reference on the next
target refresh. Removing a pin preserves the current list and reading scope.
Article shortcuts suppress counts and contribute zero to unread sorting; the
built-in All items target stays first. Alphabetical/unread sorting is presentation
state and never rewrites the sequence.

The Site reference shape is reserved as `type: site` plus normalized `hostname`;
Site support is not accepted by this version of the parser. Future nested groups
use `type: group`, stable `id`, `title`, and an ordered `children` sequence. Neither
Site controls nor groups are implemented by the flat-store contract.

This is an upcoming minor library-format transition. The corresponding app
minor-version bump remains pending until 0.10 is finalized. Existing libraries require the explicit
[CLI upgrade](../library_upgrade.md#upcoming-central-pin-format); the application
does not infer legacy membership when the store is missing.

The recovery walk constructs a membership set once, so its per-entity membership
lookup does not scan the ordered sequence. Article-shortcut resolution reads one
membership snapshot and each referenced article file, without repeatedly parsing
the store. Normal pin mutations and each newly transferred legacy pin still read
or serialize the complete pin sequence; the converter streams one source document
at a time. Those costs are governed by the number of pins and the 12 MiB store
bound, rather than a full-corpus pin discovery scan during normal navigation.
