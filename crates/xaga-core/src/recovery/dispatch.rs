//! The service-driven apply stage: walk-and-repair, streaming each item's correction onto the
//! embedding service's tiers as it is found (fresh recovery bulk-inserts instead), plus feed
//! embedding, deletes, and the reverse-index rebuild.

use std::path::Path;
use std::sync::Arc;

use crate::embeddings::pipeline::{
    annotation_embedding_entry, embed_feed, reconcile_topic_membership,
};
use crate::embeddings::service::Senders;
use crate::embeddings::{Embedder, EmbeddingEntry, EmbeddingsDb, Job, Tier};
use crate::types::{Article, Topic};

use super::RepairSink;
use super::embeddings::repair_item_embeddings;

/// Sink for the service path. Incremental recovery enqueues a per-item `Upsert` as each item is
/// walked, so memory stays bounded. Fresh recovery (empty table) instead collects rows for the fast
/// bulk-insert path — per-row upserts on an empty table are ~100× slower — and flushes them after
/// the walk via [`DispatchSink::flush`].
struct DispatchSink<'a> {
    senders: &'a Senders,
    /// `Some` on fresh recovery: collected `(topics, articles)` awaiting bulk insert. `None` is
    /// incremental, where each item was already enqueued during the walk.
    fresh: Option<(Vec<Topic>, Vec<Article>)>,
    topic_count: usize,
    article_count: usize,
}

impl<'a> DispatchSink<'a> {
    fn new(fresh: bool, senders: &'a Senders) -> Self {
        Self {
            senders,
            fresh: fresh.then(|| (Vec::new(), Vec::new())),
            topic_count: 0,
            article_count: 0,
        }
    }

    /// Fresh recovery only: bulk-insert the collected rows and queue their embeds. A no-op when
    /// incremental (already enqueued during the walk). Topics first so they embed before articles.
    async fn flush(&mut self, db: &EmbeddingsDb) {
        if let Some((topics, articles)) = self.fresh.take() {
            bulk_insert_topics(db, topics, self.senders).await;
            bulk_insert_articles(db, articles, self.senders).await;
        }
    }
}

impl RepairSink for DispatchSink<'_> {
    fn topic(&mut self, topic: Topic) {
        self.topic_count += 1;
        match &mut self.fresh {
            Some((topics, _)) => topics.push(topic),
            None => self
                .senders
                .enqueue(Tier::Maintenance, Job::UpsertTopic { topic }),
        }
    }

    fn article(&mut self, article: Article) {
        self.article_count += 1;
        match &mut self.fresh {
            Some((_, articles)) => articles.push(article),
            None => self
                .senders
                .enqueue(Tier::Maintenance, Job::UpsertEntry { article }),
        }
    }
}

/// Walks the library and repairs the embeddings, streaming the corrections onto the service's tiers.
/// Lives here (the bridge), not in `embeddings`, so the filesystem reads it performs
/// (`repair_item_embeddings`, `list_topics`) stay out of the indexing layer.
/// Driven by the service's control loop via `Job::StartRecovery`.
///
/// Topics are enqueued before articles so they index first. Recovery is system-initiated, so its
/// work rides the Maintenance tier (lowest precedence) — except topic embedding, which is always
/// dispatched at Interactive, per the priority definitions in the service's `run_loop`.
pub(crate) async fn dispatch_recovery(
    library_dir: &Path,
    db: &EmbeddingsDb,
    embedder: &Arc<Embedder>,
    senders: &Senders,
) {
    // Compact the DB before queuing any new work to clear version files accumulated during
    // previous sessions. Without this, Lance's auto-cleanup fires mid-upsert and blocks the loop.
    log::info!("recovery: pre-recovery compact starting");

    if let Err(e) = db.compact().await {
        log::error!("recovery: pre-recovery compact failed: {e}");
    }

    // Capture emptiness before any inserts, so a fresh post-reset recovery can use the fast
    // bulk-insert path instead of per-row upserts.
    let fresh = db.count().await.unwrap_or(1) == 0;

    let mut sink = DispatchSink::new(fresh, senders);

    let residue = match repair_item_embeddings(library_dir, db, &mut sink).await {
        Ok(residue) => residue,
        Err(e) => {
            log::error!("recovery: repair_item_embeddings failed: {e}");
            return;
        }
    };

    // Fresh recovery: flush the collected rows via the bulk path. Incremental already streamed.
    sink.flush(db).await;

    log::info!(
        "recovery: {} topics + {} articles repaired, {} feeds to embed, {} to delete",
        sink.topic_count,
        sink.article_count,
        residue.to_embed_feeds.len(),
        residue.to_delete.len(),
    );

    // Embed feed rows synchronously (few; run here in the bridge).
    for feed in &residue.to_embed_feeds {
        if let Err(e) = embed_feed(feed, embedder, db).await {
            log::error!("recovery: embed feed {}: {e}", feed.id);
        }
    }

    // All deletes (items + stale feed rows) as a single batch (one LanceDB version).
    senders.enqueue(
        Tier::Maintenance,
        Job::DeleteBatch {
            ids: residue.to_delete,
        },
    );

    // Rebuild the member→topic reverse index from the topic files. Unconditional: a
    // membership-only change doesn't alter a member's checksum, so repair_item_embeddings
    // would skip it. Bounded by drift via the topic-row `items` base.
    match crate::persistence::topics::list_topics(library_dir).await {
        Ok(topics) => {
            for topic in &topics {
                let member_ids: Vec<String> =
                    topic.members.iter().map(|i| i.id().to_string()).collect();

                if let Err(e) = reconcile_topic_membership(db, &topic.id, &member_ids).await {
                    log::error!(
                        "recovery: reverse-index reconcile for topic {} failed: {e}",
                        topic.id
                    );
                }
            }
        }
        Err(e) => {
            log::error!("recovery: list_topics for reverse-index rebuild failed: {e}")
        }
    }

    // After all background work drains, compact + prune, then (re)build scalar indexes.
    senders.enqueue(Tier::Maintenance, Job::Compact);
    senders.enqueue(Tier::Maintenance, Job::EnsureIndexes);
}

/// Batch size for the fresh-recovery bulk inserts. Safe only when the table has no pre-existing
/// rows for these ids (a fresh post-reset recovery).
const BULK_BATCH: usize = 1024;

/// Bulk path for fresh recovery (entries): builds each article's row (plus a null-vector row per
/// annotation) and inserts in batches, then queues the carried `Article` for the embed pass.
async fn bulk_insert_articles(db: &EmbeddingsDb, articles: Vec<Article>, senders: &Senders) {
    let mut buf: Vec<EmbeddingEntry> = Vec::with_capacity(BULK_BATCH);

    for article in articles {
        buf.push(EmbeddingEntry::from_article(&article, None));

        for annotation in &article.annotations {
            buf.push(annotation_embedding_entry(
                &article,
                annotation,
                Vec::new(),
                None,
            ));
        }

        senders.enqueue(Tier::Maintenance, Job::EmbedEntry { article });

        if buf.len() >= BULK_BATCH {
            if let Err(e) = db.add_batch(std::mem::take(&mut buf)).await {
                log::error!("bulk_insert_articles: add_batch failed: {e}");
            }
        }
    }

    if let Err(e) = db.add_batch(buf).await {
        log::error!("bulk_insert_articles: add_batch failed: {e}");
    }
}

/// Bulk path for fresh recovery (topics): builds each topic's row and queues the carried `Topic`
/// for the embed pass.
async fn bulk_insert_topics(db: &EmbeddingsDb, topics: Vec<Topic>, senders: &Senders) {
    let mut buf: Vec<EmbeddingEntry> = Vec::with_capacity(BULK_BATCH);

    for topic in topics {
        buf.push(EmbeddingEntry::from_topic(&topic, None));
        // All topic embeddings are dispatched at Interactive priority, regardless of initiator,
        // per the priority definitions in the embedding service's `run_loop` (T1). Recovery is
        // system-initiated, but topic embedding is the documented exception that stays Interactive.
        senders.enqueue(Tier::Interactive, Job::EmbedTopic { topic });

        if buf.len() >= BULK_BATCH {
            if let Err(e) = db.add_batch(std::mem::take(&mut buf)).await {
                log::error!("bulk_insert_topics: add_batch failed: {e}");
            }
        }
    }

    if let Err(e) = db.add_batch(buf).await {
        log::error!("bulk_insert_topics: add_batch failed: {e}");
    }
}
