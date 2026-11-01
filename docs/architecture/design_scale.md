# Design Scale

**The app is designed to the declared design target, not to the observed average or the common case. A scale or "good enough" assumption baked in without being surfaced is debt: it is recorded as fact and location, with severity and ordering left unassigned.**

## The declared target

- Expected intake: ~240–360K entries/year.
- Design target: ~4× expected (activity ×2, merger ×2) — a multi-million-entry corpus.

The app is not small, and it is not designed to the expected scale; it is designed to the target above.

## What the target changes

- **Sizing must hold for the bulk case within the target, not the typical case.** The "usually 1" framing for membership fan-out held only for small live edits; topic delete, bulk import, and fresh-recovery rebuild put thousands of members through the same path — well within design assumptions — and the per-row version degraded into fragment explosion and a potential recovery stall loop. The fix is the chunked, checkpointed fan-out (`reconcile_topic_membership`, `crates/xaga-core/src/embeddings/pipeline.rs`; see `crash_consistency.md`).
- **Performance analysis runs at the design target.** Costs that round to zero at the average do not at the target (a full-corpus embed takes hours; an unbounded in-memory queue of content-carrying jobs reaches tens of GB at the million scale).
- **Unsurfaced assumptions are recorded, not fixed silently.** The record states fact and location — what was assumed and where it lives in the code — with severity and ordering left unassigned. A fixed batch size or a sequential read loop chosen without being surfaced is exactly what belongs in such a record.

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
