# Design Scale

**Evaluate performance at the declared design target, including bulk operations. Record unverified assumptions and their code locations. Do not silently decide their severity or priority.**

## The declared target

- Expected intake: ~240–360K entries/year.
- Design target: ~4× expected (activity ×2, merger ×2) — a multi-million-entry corpus.

Use the design target, rather than expected intake alone, when choosing and evaluating implementations.

## What the target changes

- **Include bulk membership changes.** Topic deletion, import, and recovery can update thousands of members through the same code used for one live edit. The former per-row implementation created excessive database fragments and could repeatedly stall recovery. `reconcile_topic_membership` now processes batches with saved checkpoints (`crates/xaga-core/src/embeddings/pipeline.rs`; see `crash_consistency.md`).
- **Performance analysis runs at the design target.** Costs that round to zero at the average do not at the target (a full-corpus embed takes hours; an unbounded in-memory queue of content-carrying jobs reaches tens of GB at the million scale).
- **Record unverified choices.** Identify the assumption and its code location, such as an unmeasured batch size or sequential read loop. Leave severity and scheduling for an explicit decision instead of silently accepting or changing the choice.

## Article color query measurement

On 2026-10-31, the `color_queries_at_design_scale` Rust test passed on macOS
ARM64 with LanceDB 0.33.0 / lance-index 9.0.0 in a debug build. Reproduce with:

```sh
cargo test -p xaga-core -p xaga-cli --lib --bins --locked color_queries_at_design_scale -- --ignored --nocapture
```

The isolated synthetic cache contains two million rows: 1.8 million Entries and
200,000 annotations. Initially 257,143 Entries are Blue; the rest are unmarked.
Vectors are null; the fixture measures scalar query behavior, not embedding or
nearest-neighbor performance. Load took 15.1 seconds and scalar-index creation
21.7 seconds. Each query projects the actual summary columns and materializes
all matching Arrow batches, after inspecting its plan. The query timings below
exclude UI rendering, domain-object mapping, and sorting.

| State | Blue Entries (ms) | Marked Entries (ms) | Saved items (ms) |
| --- | ---: | ---: | ---: |
| Initial | 3,320 | 3,273 | 4,892 |
| Assign Green | 3,271 | 3,275 | 4,872 |
| Recolor to Blue | 3,268 | 3,279 | 4,882 |
| Remove color | 3,268 | 3,275 | 4,974 |

Every plan used `ScalarIndexQuery` with bitmap indexes on kind and color.
Equality combined both indexes with AND. Marked membership used indexed empty
color equality followed by NOT and kind equality. Saved used that intersection
OR the indexed annotation-kind predicate. This remained true after each mutation
without rebuilding the indexes. All result counts matched the mutations:
257,143–257,144 matching Entries and 457,143–457,144 Saved items.

The three individual cache mutations took 24, 16, and 16 milliseconds. Indexed
selection does not eliminate the cost of returning hundreds of thousands of
summaries: the current unbounded list result still takes seconds to materialize
at this fixture's selectivity. These measurements do not establish release-build
latency, cold-cache latency, rendering cost, or performance on other hardware.

## Tag migration scaling

Measured on 2026-11-01 on macOS Darwin 27 ARM64, with the debug Rust test build,
LanceDB 0.33.0 and lance 9.0.0. Each point runs in a separate process with a fresh
synthetic 0.13 library: one regular 307-byte article file per Entry, a populated
cache with dense 384-value Float32 vectors, one Topic reference per row, and
`Research`/`RESEARCH` labels resolving to one definition. Files are independent
copies, not hard links. No embedding model runs during conversion.

The measured interval is `upgrade_library`, excluding fixture creation and
post-conversion verification. OS peak physical footprint includes the entire test
process, including setup and verification. Cache commits include scalar-index and
compaction maintenance as well as assignment updates.

| Entries | Migration (s) | Peak footprint (MiB) | Cache commits |
| ---: | ---: | ---: | ---: |
| 2,048 | 0.624 | 44.2 | 12 |
| 4,096 | 1.253 | 75.5 | 14 |
| 8,192 | 2.756 | 167.0 | 18 |
| 16,384 | 5.939 | 335.9 | 26 |
| 32,768 | 14.509 | 395.4 | 42 |
| 65,536 | 43.235 | 628.1 | 76 |
| 131,072 | 239.747 | 859.5 | 141 |

Every point verified assignment IDs/counts, retained checksum and Topic references,
reading/color state, sampled source bytes and the reported unchanged-file count,
completed library version, and stable
IDs on a completed-upgrade retry. The populated-vector and unrelated-column
preservation tests also cover interrupted upgrades and different assignment sets
across a 1,024-row batch boundary.

The 128k point exercises the periodic maintenance trigger at 102,400 files. Its
elapsed time is 5.55 times the 64k point, so the measurements do not support a
single constant cost per Entry across the tested range.

Two extrapolation scenarios, both anchored to the measured 131,072-Entry point:

- Constant throughput: `t(N) = 239.747 × (N / 131072)` seconds.
- Continued empirical growth: `t(N) = 239.747 × (N / 131072)^1.370394` seconds.
  The exponent is the least-squares slope of log elapsed time against log Entry
  count across all seven points; anchoring it to the largest point avoids treating
  the fit's underestimate at that point as observed performance.

| Target Entries | Constant-throughput scenario | Empirical-growth scenario |
| ---: | ---: | ---: |
| 1,000,000 | 30.5 min | 64.7 min |
| 2,000,000 | 61.0 min | 167.3 min |

These are modeled scenarios, not measured runtimes, confidence bounds, or latency
commitments. Only one periodic maintenance cycle is observed in the curve. More
cycles, larger articles, cold filesystem reads, additional annotations/columns,
and different tag-set populations can change the growth. The measured memory
curve does not establish a hard memory ceiling at one or two million Entries.
The converter's own retained pending work is batch-bounded; Lance's cache/index
and compaction allocations contribute to process memory separately.

Reproduce one point after building the test executable:

```sh
XAGA_TAG_SCALE_ROWS=65536 cargo test --workspace --lib --bins --locked tag_upgrade_scaling_point -- --ignored --nocapture
```

Use 2048, 4096, 8192, 16384, 32768, 65536 and 131072 for the curve. To measure peak
memory independently of compilation, run the emitted `xaga_cli` test executable
with `/usr/bin/time -l`, passing `tag_upgrade_scaling_point --ignored --nocapture`
and the same environment variable.

The separate `tag_registry_snapshot_costs` benchmark used 10,000 definitions
(651,207 serialized bytes). Write took 127 ms; reading and building the lookup map
153 ms; two million name resolutions 2,589 ms. OS maximum RSS was 38,944,768 bytes
and peak physical footprint 27,067,040 bytes. This measures registry work separately
from corpus conversion, not a requirement for that many definitions in a library.

## Tag query scaling

Tag browsing uses a LabelList index for `array_has_all(tags, selectedIds)`,
intersected with the Entry-kind and unread bitmap indexes. An explained unread
query includes `ScalarIndexQuery` with `tags_idx(LabelList)`, `kind_idx(Bitmap)`,
and `is_unread_idx(Bitmap)`. The list query projects summaries, excluding vectors
and content. Counts use `count_rows` directly.

A bounded local debug-build measurement added 2k–64k synthetic Entry rows to one
cache, optimizing indexes at each point. Every row carried one tag, 1% carried
both selected tags, and 0.5% were unread. Times are single runs, not latency
percentiles; the first point includes cold query overhead. `build/index` includes
that point's added rows and index creation/catch-up. Sparse list reads return the
1% intersection; dense reads return every Entry with no tags selected.

| Total Entries | Build/index (ms) | Unread count (ms) | Sparse list (ms) | Dense list (ms) |
| ---: | ---: | ---: | ---: | ---: |
| 2,000 | 225 | 11.8 | 10.1 | 13.1 |
| 4,000 | 97 | 4.3 | 7.9 | 20.6 |
| 8,000 | 146 | 5.0 | 12.2 | 37.7 |
| 16,000 | 213 | 5.1 | 16.8 | 69.9 |
| 32,000 | 320 | 5.0 | 25.0 | 133.5 |
| 64,000 | 600 | 6.5 | 42.4 | 262.5 |

Scaling the 64k per-row list costs linearly gives roughly 0.7–1.3 seconds for a
1% intersection at 1–2 million Entries, and 4.1–8.2 seconds for returning every
Entry. These are core-query estimates, excluding IPC serialization and frontend
allocation/filtering/sorting. Count times showed no comparable linear growth in
this range; that does not establish a latency bound at two million rows. Tag
cardinality, result density, real summary sizes, index freshness, and storage
conditions affect the estimates. This fixture uses two tag values and does not
measure high-cardinality indexes.

The existing list API returns the entire matching result; virtualization bounds
rendered rows, not query payload or client memory. Broad or empty tag selections
at the design target therefore retain that list architecture's output-volume
cost. The LabelList index avoids scanning all tag lists for selective queries;
it does not remove the cost of returning millions of matching summaries.

Reproduce the curve without article-file fixtures:

```sh
cargo test --workspace --lib --bins --locked tag_browsing_scaling_curve -- --ignored --nocapture
```

## Tag rename scaling

Rename queries indexed tag UUID assignments and fetches only candidate Entry IDs,
ordered by ID, in batches of at most 1,024. It opens only those files during the
prompt phase. The persisted pending list precedes writes; the existing `after`
field advances only after that batch succeeds. Four tag workers may submit up to
eight file rewrites each, within the writer's global eight-lane execution cap.
Registry and Markdown inputs retain their 12 MiB limits; parsing and patching
allocate additional copies. No corpus-wide file/ID list is retained.

A local debug-build regression curve used four affected files in each library,
10,002 definitions, small Markdown files, and fully indexed Entry summaries.
Timing includes opening the read table, querying candidates, checkpoints, four
file rewrites, completion checkpointing, and alias retirement. Fixture/index creation and
the initial registry rename request are excluded. Native notification delivery
and UI rendering are also excluded.

| Library Entries | Affected files | Rename completion (seconds) |
| ---: | ---: | ---: |
| 2,000 | 4 | 1.261 |
| 4,000 | 4 | 1.265 |
| 8,000 | 4 | 1.259 |

The prompt cost in this fixture stayed flat as unrelated files increased. This
replaces the earlier full-library rename scan and its corpus-linear timing
estimates. These are bounded-fixture measurements, not a latency guarantee at
two million rows; index freshness, registry size, cache/storage conditions, and
candidate count affect cost. Popular tags still require rewriting all affected
files, in bounded batches.

Rename finishes after its indexed batches commit. It does not add a source scan
to verify completeness. Files missing from the index retain their old spelling;
later recovery/discovery may register that name as a separate tag. There is no
post-rename cleanup status or restart requirement for another rename.

```sh
cargo test --workspace --lib --bins --locked tag_rename_scaling_curve -- --ignored --nocapture
```


## Tag deletion scaling

Deletion uses the same bounded indexed candidate query as rename, with at most
1,024 Entry IDs per batch and eight concurrent source-file patches. It then
removes the UUID with one partial-column cache update for that batch. Source
reads are limited to indexed candidates, persisted pending IDs, and queued
article/assignment work that encounters an active deleting identity. Unrelated
source files are not scanned to prove completion.

For a tag assigned to every Entry in a two-million-Entry library, an uninterrupted
indexed pass requires 1,954 batches, plus completion. Application-held candidate
and checkpoint lists remain bounded independently of that population; registry
parsing, Markdown copies, and the query engine have additional memory costs.
Each batch performs four serialized registry checkpoints, file work, and one
cache update. Live continuations yield between service turns; recovery maintains
the cache every 100 batches. Sparse tags avoid corpus-wide source reads, but query
and cache-update costs still depend on index/storage conditions. No deletion
latency or whole-process memory bound at two million Entries has been measured.

The bounded regression fixture uses 1,032 indexed candidates, including missing
files and an unrelated-label false positive. The first step removes exactly
1,024 cached assignments; the next pass handles the remaining eight. Separate
fixtures cover pending-file replay after a cache reset, failed batches, and
post-retirement discovery of missed labels. These establish batch and recovery
behavior, rather than a production-scale timing claim.


## Tag merge scaling

Merge selects the union of both participating IDs, projecting only Entry IDs in
ordered batches of at most 1,024. It persists both phase checkpoints, patches up
to eight source files concurrently, and updates only losing-ID cache assignments.
Two idempotent partial-column updates handle rows with and without an existing
survivor assignment. Candidate/checkpoint lists stay bounded; query-engine memory,
registry parsing, and Markdown copies add separate costs. Live batches retain
Interactive priority and yield between service turns; existing service compaction
intervals apply. Recovery maintains the cache every 100 batches.

On 2026-11-03, `tag_merge_at_design_scale` passed on macOS ARM64 with LanceDB
0.33.0 / Lance 9.0.0 in a debug build. The temporary synthetic cache held two
million Entries: one million source-only, 500,000 destination-only, and 500,000
carrying both. Vectors were null. Only the first 3,072 Entries had small Markdown
source fixtures; the test measured three real 1,024-file batches, not full-corpus
completion. Cache construction and index maintenance took 118.191 seconds.

| Operation | Elapsed |
| --- | ---: |
| Count populations and persist acceptance | 39 ms |
| First 1,024-Entry batch | 7,277 ms |
| Second 1,024-Entry batch | 8,744 ms |
| Third 1,024-Entry batch | 7,640 ms |

The source ID survived its larger population while all rewritten labels became
the destination name. Each batch removed exactly 512 losing-ID assignments,
including dual-tag rows, and retained the survivor assignment. The test also
checked rewritten source-only and dual-tag files. Timings include query,
checkpoints, source patches, and cache updates; they exclude native event delivery
and rendering. Other build/test processes ran concurrently. These are single-run
observations, not latency percentiles, whole-process memory bounds, or a measured
completion time for two million source files. Larger Markdown documents, registry
size, index freshness, and storage conditions affect cost. A live batch occupies
the indexing consumer until it yields, so the measured seconds per batch also
identify a possible wait for subsequent queued cache work.

Reproduce in an isolated temporary library:

```sh
cargo test -p xaga-core --lib tag_merge_at_design_scale --locked -- --ignored --nocapture
```
