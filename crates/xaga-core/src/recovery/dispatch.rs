//! The service-driven apply stage: process the recon results by enqueuing the work as Maintenance-tier jobs
//! onto the embedding service, plus the feed-row reconciliation and reverse-index rebuild.

use std::path::Path;
use std::sync::Arc;

use crate::embeddings::pipeline::{
    annotation_embedding_entry, embed_feed, reconcile_topic_membership,
};
use crate::embeddings::service::Senders;
use crate::embeddings::{Embedder, EmbeddingEntry, EmbeddingsDb, Job, Tier};
use crate::types::{Article, Topic};

use super::embeddings::reconcile_item_embeddings;
use super::feeds::reconcile_feed_entries;

/// Reconciles the embeddings and enqueues the reconciled work onto the service's tiers.
/// Lives here (the bridge), not in `embeddings`, so the filesystem reads it performs
/// (`reconcile_feed_entries`, `reconcile_item_embeddings`, `list_topics`) stay out of
/// the indexing layer. Driven by the service's control loop via `Job::StartRecovery`.
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
    // bulk-insert path below instead of per-row upserts.
    let fresh = db.count().await.unwrap_or(1) == 0;

    // Find feed entries the atoms declare but that are missing on disk, then save them so they
    // exist before the entries scan.
    let missing = reconcile_feed_entries(library_dir).await;
    crate::persistence::feeds::save_new_entries(library_dir, &missing).await;

    match reconcile_item_embeddings(library_dir, db).await {
        Ok(recon_results) => {
            log::info!(
                "recovery: {} topics + {} articles + {} feeds to embed, {} to delete",
                recon_results.to_embed_topics.len(),
                recon_results.to_embed_articles.len(),
                recon_results.to_embed_feeds.len(),
                recon_results.to_delete.len(),
            );

            // Embed feed rows synchronously (few; run here in the bridge). Items are queued below.
            for feed in &recon_results.to_embed_feeds {
                if let Err(e) = embed_feed(feed, embedder, db).await {
                    log::error!("recovery: embed feed {}: {e}", feed.id);
                }
            }

            // All deletes (items + stale feed rows) as a single batch (one LanceDB version).
            senders.enqueue(
                Tier::Maintenance,
                Job::DeleteBatch {
                    ids: recon_results.to_delete,
                },
            );

            if fresh {
                // Empty table: bulk-insert rows (no per-row get_entry/delete), then queue each
                // object for the vector pass. Topics first so they embed before articles.
                bulk_insert_topics(db, recon_results.to_embed_topics, senders).await;
                bulk_insert_articles(db, recon_results.to_embed_articles, senders).await;
            } else {
                // Carry each parsed domain object through Upsert so the consumer never re-reads.
                for topic in recon_results.to_embed_topics {
                    senders.enqueue(Tier::Maintenance, Job::UpsertTopic { topic });
                }

                for article in recon_results.to_embed_articles {
                    senders.enqueue(Tier::Maintenance, Job::UpsertEntry { article });
                }
            }

            // Rebuild the member→topic reverse index from the topic files. Unconditional: a
            // membership-only change doesn't alter a member's checksum, so reconcile_item_embeddings
            // would skip it. Bounded by drift via the topic-row `items` base.
            match crate::persistence::topics::list_topics(library_dir).await {
                Ok(topics) => {
                    for topic in &topics {
                        let member_ids: Vec<String> =
                            topic.members.iter().map(|i| i.id().to_string()).collect();

                        if let Err(e) = reconcile_topic_membership(db, &topic.id, &member_ids).await
                        {
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
        Err(e) => log::error!("recovery: reconcile_item_embeddings failed: {e}"),
    }
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
