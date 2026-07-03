# Crash Consistency: write ordering, checkpoints, recovery backstop

**Every multi-step write has a defined crash story: ordering decides what survives, checkpoints
bound the re-work, and recovery heals the rest from the source of truth.**

## The coverage model

Two failure classes are distinguished, and they are covered by different mechanisms:

- **Graceful shutdown.** The service's biased `select!` (`run_loop`,
  `crates/xaga-core/src/embeddings/service.rs`) takes `Shutdown` between loop turns, so work
  done inline within one turn is atomic with respect to it. Work still queued at shutdown is
  dropped, not drained.
- **Hard kill / crash.** Nothing inline is guaranteed; a multi-commit sequence can stop between
  any two commits.

**Recovery is the universal backstop for both:** dropped jobs and half-applied sequences are
rebuilt from the source files on the next run.

## Instances

- **Entry `.md` before feed `.atom`** (`write_atom`, `crates/xaga-core/src/persistence/feeds.rs`).
  Entry content — the irreplaceable part — is persisted before feed membership. A crash between
  the two leaves entries not yet in the atom; the next refresh re-adds the membership from the
  live feed and skips the already-written `.md` files, so no content is lost. The reverse order
  could lose content.
- **Chunked fan-out with per-chunk checkpoints** (`reconcile_topic_membership`,
  `crates/xaga-core/src/embeddings/pipeline.rs`). The mechanism: the topic's LanceDB row stores
  its member-id list (`items`) as the prior-membership base — persisted on the row rather than
  held in memory so it survives restart — and each reconcile diffs the topic `.md`'s members
  against that base, touching only the delta. The reconcile is the sole writer of the `topics`
  (member) and `items` (topic) columns; entry/annotation sync carries them forward untouched on
  every upsert, so the column lags the file and the diff stays meaningful. Deltas are written in
  chunks of 1024, and the `items` base is checkpointed after each chunk, so a crash mid-fan-out
  resumes from the remaining members instead of re-scanning the whole set. Known window: the
  delete-then-add within a chunk on a hard kill; recovery heals it.
- **Unconditional recovery rebuild, decoupled from unrelated gates.** The reverse-index rebuild
  runs on every recovery regardless of the checksum/embedding gate — a membership-only change
  never touches the entry `.md`, so a gated rebuild would skip it. It backstops both a dropped
  topic event (graceful shutdown before it ran) and a crash mid-fan-out
  (`crates/xaga-core/src/recovery/`).
- **Out-of-band changes are a recovery concern.** External edits and migrations are not
  reflected live; recovery reconciles them on its next run. Eventual sync, not live sync.
