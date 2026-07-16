# Job payload modeling is inconsistent (pending — design decision)

## Current state

There is no single answer to "what does a Job carry" (`crates/xaga-core/src/embeddings/`, the `Job` enum):

- Some variants carry full domain objects: `UpsertEntry{article}`, `EmbedFeed{feed}`.
- Some carry `EntityRef{id, kind}`: `SyncState`, `Delete`.
- Some carry raw strings: `AddMember{topic_id, member_id}`.

The per-kind split (`UpsertEntry`/`UpsertTopic`, `EmbedEntry`/`EmbedTopic`) was chosen over a single kind-carrying variant during the decoupling work without approval, and is the direct cause of several of the duplications in `docs-transient/backlog/cross_cutting/refactor__dry_consolidation.md`. The content variants were converted to carry objects; the rest were left as-is.

## Open decisions

One modeling answer: what a Job carries (object vs `EntityRef` vs raw ids — or a defined rule for when each applies), and whether the per-kind variant split stays or a kind-carrying variant replaces it. Interacts with the memory-ceiling item (`fresh_reindex_memory_ceiling.md`: content-carrying jobs are what make the queue unbounded in size).
