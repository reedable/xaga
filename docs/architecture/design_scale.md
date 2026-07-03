# Design Scale

**The app is designed to the declared design target, not to the observed average or the common
case. A scale or "good enough" assumption baked in without being surfaced is debt: it is
recorded as fact and location, with severity and ordering left unassigned.**

## The declared target

- Expected intake: ~240–360K entries/year.
- Design target: ~4× expected (activity ×2, merger ×2) — a multi-million-entry corpus.

The app is not small, and it is not designed to the expected scale; it is designed to the
target above.

## What the target changes

- **Sizing must hold for the bulk case within the target, not the typical case.** The "usually
  1" framing for membership fan-out held only for small live edits; topic delete, bulk import,
  and fresh-recovery rebuild put thousands of members through the same path — well within design
  assumptions — and the per-row version degraded into fragment explosion and a potential
  recovery stall loop. The fix is the chunked, checkpointed fan-out
  (`reconcile_topic_membership`, `crates/xaga-core/src/embeddings/pipeline.rs`; see
  `crash_consistency.md`).
- **Performance analysis runs at the design target.** Costs that round to zero at the average do
  not at the target (a full-corpus embed takes hours; an unbounded in-memory queue of
  content-carrying jobs reaches tens of GB at the million scale).
- **Unsurfaced assumptions are recorded, not fixed silently.** The record states fact and
  location — what was assumed and where it lives in the code — with severity and ordering left
  unassigned. A fixed batch size or a sequential read loop chosen without being surfaced is
  exactly what belongs in such a record.
