# Read-state propagation latency and the consistency model (pending)

## Symptom

Selecting an entry in `src/components/list/ItemList.tsx` marks it read, but the read state is slow to propagate to the UI, and the sidebar unread count is slower still.

## Current state

Marking read goes: `.md` write → typed job on the service queue → LanceDB commit (async) → count re-query (`reload` + full unread-set materialization). The old in-memory `EntryStateCache` was updated synchronously (instant); it was retired when reads moved to LanceDB. Which step dominates the latency has not been measured.

## Recorded options

- Optimistic UI update on mark-read.
- Write-through to LanceDB for user actions (not just the `.md`).
- Event-driven count refresh instead of re-query.

The reload facet is itemized separately in `db_reload_on_every_read.md`. The underlying open question — the app's read-after-write consistency model for user actions — is a design decision that has not been made.
