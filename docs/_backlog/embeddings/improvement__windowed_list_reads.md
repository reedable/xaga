# List rendering via windowed LanceDB reads (pending)

## Current state

List commands load whole result sets; `src/components/list/ItemList.tsx` renders them. The
collection counts (total / entry / unread) are already carried on Feed/Topic.

## Recorded design direction

Use the counts together with Virtuoso to render only the visible window and fetch the matching
row range from LanceDB on demand. The read API would be shaped to serve a window (row range +
total count), not full result sets — the same API whose scale gaps are recorded in
`read_api_scale_gaps.md`, and whose abstraction question is in
`db_as_cache_read_abstraction.md`.

Idea had been pending ~2 months when recorded (2026-07).
