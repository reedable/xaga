# Read-model abstraction over the LanceDB cache (pending)

## Current state

The command layer calls `EmbeddingsDb` directly (`unread_ids` / `bookmarked_entries` /
`get_entry` / …), binding consumers to LanceDB specifically. Per
`docs/principles/single_source_of_truth.md`, LanceDB is a cache — every column reconstructible
from the files — not a second source of truth. The `is_cache_ready` / `useCacheReady` naming
already reflects the cache abstraction; the rest of the read API does not.

## Recorded idea

A thin read-model / cache abstraction: a trait the commands depend on, with `EmbeddingsDb`
(and historically `EntryStateCache`) as implementations, so the consumer layer depends on
"a cache," not on LanceDB.

## Open decision

Whether to introduce the trait, its surface, and its timing relative to the windowed-reads
reshaping (`windowed_list_reads.md`, which would change the same API's shape).
