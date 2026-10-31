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
