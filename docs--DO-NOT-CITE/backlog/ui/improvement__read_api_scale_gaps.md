# Read API scale gaps (pending)

Recorded facts and locations, all in `crates/xaga-core/src/embeddings/db.rs`:

- **`all_entries` has no limit/pagination** — "list all" materializes every entry row.
- **`entries_by_ids` builds one `id IN (…)` over all ids, no chunking** — large feed id-sets produce a large IN list.
- **Index coverage doesn't match the sort order.** `ensure_indexes` indexes `id` and `published_at`, but the list sort order is `updated_at → published_at → created_at` (`filter::sort_items`); `created_at`/`updated_at` are unindexed, so ordered pagination by the actual sort key isn't fully covered.

Severity and ordering unassigned.

## Recorded design direction (fix for the first two)

List commands load whole result sets; `src/components/list/ItemList.tsx` renders them. The collection counts (total / entry / unread) are already carried on Feed/Topic.

Use the counts together with Virtuoso to render only the visible window and fetch the matching row range from LanceDB on demand. The read API would be shaped to serve a window (row range + total count), not full result sets. This does not cover the third item above (index/sort-order coverage); see also `db_as_cache_read_abstraction.md`.

Idea had been pending ~2 months when recorded (2026-07).
