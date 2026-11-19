# Library directory layout

Core's `library` API owns paths and stored resource references. Version 0.17 uses:

```text
Library/
├── .xaga/
│   ├── xaga.lock
│   └── embeddings.lance/
├── metadata/
│   ├── library.yaml
│   ├── favicons/
│   ├── feeds/
│   ├── pins.yaml
│   └── tags.yaml
├── entries/
├── topics/
├── pdfs/
└── images/
```

Saved records own identity; filenames provide lookup indexes validated against
the record. See [persisted entity identity](persisted_entity_identity.md).

## Entry location contract

The `library` API must provide the complete ordered collection of Entry scan
directories, including `entries/` and `pdfs/`. Entry discovery must use that
collection rather than assume that Entries occupy one directory. Only Markdown
Entry records are included; PDF bytes and other resources are not Entries to
parse as Markdown.

Entry lookup and selection of an existing write destination must share one
resolver using those locations. An existing `entries/{id}.md` takes precedence
over `pdfs/{id}.md`; otherwise the PDF companion is the Entry record. Discovery
must apply the same precedence and return each ID once. A lookup failure due to
unreadable storage must not be treated as absence or authorize a competing file.
Resolving a record does not move, merge, or delete it.

All library paths, scan roots, and stored resource references must come from the
owning `library` API. Callers, including tests, must not hardcode library paths,
repeat the directory list, or implement their own location precedence. Keep
new-file destination construction distinct from resolution of an existing Entry
so creation does not bypass an existing PDF companion. See
[DRY](../principles/dry.md).

This is the required contract. Updating callers that still assume a single
Entry directory is implementation work; the directory layout itself is unchanged.

Back up every visible directory. `metadata/` contains authoritative format
metadata, subscriptions and feed membership, pins, and tag definitions and pending
transitions, as well as shared website icons. Article Markdown retains annotations
and readable tag assignments; Topic Markdown retains ordered membership.

With all library users closed, `.xaga/` may be omitted from a backup or deleted.
Writable startup recreates its lock and missing database, and normal recovery
rebuilds derived data. Rebuilding vectors requires the embedding model and time;
read-only startup does not create a database. Do not delete an active lock file.

## Conversion from the root layout

The explicit app/CLI converter records the pending target before relocating any
store or database. Each move refuses an existing destination and syncs both
parent directories; retries accept a completed move when its old path is absent.
Conflicting manifests are reported rather than choosing a version. The manifest
moves after the other stores. Favicon frontmatter references change from
`../favicons/<hash>` to `../metadata/favicons/<hash>` before completion; body image
references remain `../images/<id>/<hash>`.

Conversion retains both the legacy root lock and the internal lock. A live old
writer prevents conversion. After successful conversion the obsolete root lock
path is removed; the internal lock remains held through the operation. Older
executables are unsupported after conversion: old CLI commands that ignore
format compatibility cannot be prevented from creating a new root lock. Close
all library users before upgrading and use the matching app and CLI afterward.

App startup requires this conversion before browsing, offers upgrading, selecting
another library, or quitting, and keeps the gate closed after failure or
interruption. Completion restarts the app before normal operations. The gate is
specific to an unfinished layout conversion; future version differences in a
completed new-layout library retain existing read-only behavior.

The current CLI also refuses ordinary library commands until the layout is
converted. `library info`, `library upgrade`, and account configuration remain
available. This gate does not add a blanket version-compatibility restriction to
CLI operations on completed new-layout libraries.
