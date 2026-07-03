# Read API scale gaps (pending)

Recorded facts and locations, all in `crates/xaga-core/src/embeddings/db.rs`:

- **`all_entries` has no limit/pagination** — "list all" materializes every entry row.
- **`entries_by_ids` builds one `id IN (…)` over all ids, no chunking** — large feed id-sets
  produce a large IN list.
- **Index coverage doesn't match the sort order.** `ensure_indexes` indexes `id` and
  `published_at`, but the list sort order is `updated_at → published_at → created_at`
  (`filter::sort_items`); `created_at`/`updated_at` are unindexed, so ordered pagination by
  the actual sort key isn't fully covered.

The recorded fix direction for the first two is reshaping the API to serve windows
(`windowed_list_reads.md`). Severity and ordering unassigned.
