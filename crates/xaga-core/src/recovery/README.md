# Recovery

Provides Xaga's recovery capability to the Tauri app and `xaga-cli`.

Recovery brings a library's **derived layers** back into agreement after an
interruption — a crash, power loss, `kill -9`, OOM, panic, or a transient I/O
error. A library is stored as three layers, each a projection of the one above:

- **Feed membership** — `feeds/<id>.atom` — which entries belong to a feed.
- **Entry content** — `entries/<id>.md` — the entry's content and state. This is
  the irreplaceable layer; it cannot be re-fetched once a feed item ages out.
- **Index** — `embeddings.lance` (LanceDB) — embeddings + denormalized state for
  fast reads. A pure cache: source of truth for nothing.

So "recovery" is mostly **re-deriving a lower layer from a higher one**. Today the
only active recovery *pass* is the index reconciliation,
[`repair_item_embeddings`](embeddings.rs): scan `entries/` and `topics/`,
diff against LanceDB, and queue the corrections (embed what's new or changed,
delete what's gone). The `.atom` ↔ `.md` relationship is kept consistent **by
write ordering, not by a recovery pass** — see below for why that's deliberate.

## Failure Modes

### Failure during `.atom` refresh

**Scenario.** A feed refresh fetches new items, then must write two things the
filesystem cannot commit atomically together: the new entries' `.md` files and
the updated `.atom`. A hard interruption — or a transient write error — can land
between them.

**Design: write the `.md` files first, then commit the `.atom`**
([`refresh_feed`](../persistence/feeds.rs)). The order is load-bearing. The `.md`
is the **irreplaceable content**; the `.atom` is only a **membership label**, and
membership is cheaply re-derivable because the live feed still lists those entries
on the next refresh.

**Consequence of an interruption.** A crash leaves entries that are *materialized
but not yet in the atom*. This is **benign and self-healing**:

- The content is on disk — nothing is lost.
- The next refresh of that feed re-adds the entries to the atom from the live
  feed; `save_new_entries` skips the already-written `.md`, so membership catches
  up with no duplicate work, and the entry opens instantly (its content was
  written on the interrupted attempt).
- The index layer catches the orphan `.md` independently:
  `repair_item_embeddings` scans `entries/`, finds it, and embeds it.

From the user's perspective nothing is ever lost: the entry is readable for as
long as its `.md` exists, and the feed list reconciles on the next refresh.

**Why there is deliberately no `.atom` → `.md` reconciliation.**
A reviewer might expect a recovery pass that reads each `.atom`, finds entries
with no `.md`, and recreates them — and an earlier version had exactly that
(`reconcile_feed_entries`, since removed). It is unnecessary *given the write
order*. Such a pass is only needed if the writes happen in the **reverse** order
(atom first): then a crash loses the **content** — an atom id with no `.md` and no
index row, invisible and unrecoverable except by rebuilding the `.md` from the
atom. Writing content first makes that failure mode impossible, so the
reconciliation has nothing to do. **Its absence is the design, not an oversight.**

**Edge case.** If an interrupted entry ages out of the live feed before the next
refresh (feeds typically list only recent items), its `.md` remains a permanent
orphan — still a valid, readable entry (it appears in index-backed views such as
all-items and search), just no longer attributed to its feed's membership. This
is strictly better than losing it, and is accepted.
