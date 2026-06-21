# WTF.md — audit of tech debt / violations incurred this session

Audit of the persistence↔embeddings decoupling work. Grounded in the current tree. Items in
sections A–D are still in the code; section E was corrected within the session and is gone.

Nothing here has been fixed. Listed for decisions.

---

## A. Judgments made without asking, still uncorrected

1. **Row `tags` column rebuilt from booleans, dropping non-state tags.**
   `embeddings/row.rs` `from_article`/`from_topic`/`from_feed` synthesize `tags` from
   `is_unread`/`is_bookmarked`/`is_pinned`. The original copied raw frontmatter `tags`, so any
   non-state tag in a file is no longer mirrored into LanceDB. Flagged in a report; never approved.

2. **Feed row dates emptied.**
   `from_feed` sets `created_at`/`updated_at` to `""` to mimic old behavior, even though the domain
   `Feed` carries real values.

3. **Feed checksum scheme changed.**
   Old: `sha256("title\nsubtitle")`. New: `row::checksum` = `sha256("title\n\ndescription")`.
   Different bytes → every feed re-embeds once on upgrade. Never flagged.

4. **Bookmark-import "updated" path switched to `SyncState`.**
   `src-tauri/src/commands/bookmarks.rs` — already-existing bookmarked entries now enqueue
   `SyncState{is_bookmarked}` instead of the original `UpsertContent`. Behavior change; flagged,
   not approved.

5. **Per-kind Job split.**
   Chose `UpsertEntry`/`UpsertTopic` + `EmbedEntry`/`EmbedTopic` instead of one carrier. Direct
   cause of the duplication in section B.

6. **`Article` used as the uniform entry carrier even with no article content.**
   `entries.rs::write_entry` returns `Article { content: "", annotations: [] }` and `UpsertEntry`
   carries it. `domain.rs` defines `Article` as "a loaded, content-bearing Entry"; a body-less feed
   entry is really an `Entry`. Domain mismatch.

7. **`reconcile_feeds` calls `list_feeds(dir, &empty, &empty)`** (`recovery.rs`) — passes empty
   unread/bookmarked sets and discards the counts it computes, just to get domain `Feed`s.

8. **Part 2 crate-root re-exports.**
   `pub use persistence::{...}` / `pub use query::{...}` in `lib.rs` makes every module reachable by
   two paths (`crate::entries` and `crate::persistence::entries`). Compatibility shim; Part 2 was
   approved broadly, not this specific mechanism.

(Accepted-deferred, not a hidden judgment: bulk-memory — recovery emits domain objects onto the
unbounded queue. Explicitly deferred.)

---

## B. Copy-paste / not consolidated

1. **`sync_entry_row` vs `sync_topic_row`** (`embeddings/pipeline.rs`) — near-identical:
   get_entry → checksum compare → preserve/clear vector → changed-check → carry `topics`/`items`
   forward → upsert. Differ only in row builder and annotation reconcile.

2. **`embed_entry_vector` vs `embed_topic_vector` vs `embed_feed`** — same gate (get_entry →
   checksum/vector check) + compute_vector + carry-forward + upsert, three times.

3. **`bulk_insert_articles` vs `bulk_insert_topics`** (`recovery.rs`) — identical batch loop;
   differ only in row builder + annotation rows + job variant.

4. **`from_article` / `from_topic` / `from_feed`** (`embeddings/row.rs`) — parallel `EmbeddingEntry`
   construction with the same field set and the same tags-from-booleans block repeated.

5. **`article_embed_text` / `topic_embed_text` / `feed_embed_text`** — three near-identical
   one-liners.

6. **Article construction in 3+ places, inconsistently.**
   `entry_article` (`persistence/entries.rs:139`), `article_from_fm` (`persistence/articles.rs:241`),
   `article_from_document` (wraps it), and `save_article` (`persistence/articles.rs:86`) which builds
   `Article`/`Entry` inline instead of using the helper.

7. **`is_unread`/`is_bookmarked` derivation + literal `"unread"`/`"bookmark"`/`"pinned"` strings
   scattered** across `entries.rs`, `articles.rs`, `row.rs`, `pipeline.rs`, `db.rs` — no shared
   constant or helper. Partly pre-existing; more sites added this session.

---

## C. Principle violations still in the tree

1. **Domain has no model of `tags`.** Domain types expose only derived booleans, so the
   persistence↔index `tags` column is synthesized from those booleans (A1). Couples row-building to
   "only three tags exist." Modeling gap, not just duplication.

2. **Inconsistent Job payload modeling.** Some variants carry full domain objects
   (`UpsertEntry{article}`, `EmbedFeed{feed}`), some carry `EntityRef{id,kind}` (`SyncState`,
   `Delete`), some carry raw strings (`AddMember{topic_id, member_id}`). No single answer to "what
   does a Job carry." Content variants converted to objects; the rest left as-is.

3. **`Article` vs `Entry` misuse** (A6) — richer domain type used where the base type is accurate.

4. **Naming asymmetry introduced:** `embed_entry_vector` / `embed_topic_vector` but `embed_feed`
   (no `_vector`); `sync_entry_row` / `sync_topic_row` but no `sync_feed_row` (feeds are
   single-phase). Inconsistent vocabulary for the same role.

---

## D. Docs / design divergence (not updated)

1. **SPAM.md is now contradicted by the code.** Its "Settled" taxonomy says `UpsertContent{id,kind}`
   and "consumer reads the file." The implementation is `UpsertEntry{article}` etc., consumer
   file-free. The ratified design was changed without updating (or flagging) the doc that ratifies it.

2. **Stale doc comments referencing removed names:** `embeddings/service.rs:51`, `:104`, `:450`
   (`EmbedVector`/`UpsertContent`); `embeddings/pipeline.rs:110` and `persistence/topics.rs:131`
   (`sync_row`, "watcher").

3. **The plan file** (`~/.claude/plans/dapper-conjuring-orbit.md`) still describes the abandoned
   `Content` approach.

---

## E. Corrected within the session (no longer in the tree — for the record)

- **`Content`** — invented parallel type duplicating `Frontmatter` + the domain objects; the
  pig→spam→pig pattern; stereotyped generic name. Deleted.
- **`FeedMeta` leak** — persistence storage shape (`subtitle`) exposed to `embeddings`. Deleted;
  feeds now carry the domain `Feed`.
