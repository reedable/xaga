# External-edit last-wins window has no detection

## Current behavior

- The library flock binds only xaga processes; external tools (e.g. Obsidian)
  editing `.md` files are not participants. External edits are reconciled from
  files by recovery at the next startup
  (`docs/architecture/single_writer_library_lock.md`).
- Inside a running writer instance, an external edit landing between a lane's
  read and its write is overwritten (last-wins) within the ms-scale
  read-modify-write window. No external-editor detection (file watcher, mtime
  check at write time, or similar) exists.
- Related: `docs-transient/backlog/ui/improvement__read_only_mode_ux.md` notes
  the absence of a live cross-process update mechanism for the read-only
  instance's stale view.

## Work

Detect (or otherwise not silently overwrite) an external edit that lands
inside the read-modify-write window of a running writer.

## Design questions

- Detection mechanism: mtime/checksum compare at write time inside the lane
  vs. a file watcher vs. accept-and-log.
- What the writer should do on detection: refuse and surface, merge, or
  last-wins with a visible warning.
