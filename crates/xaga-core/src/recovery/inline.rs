//! The synchronous execute stage (CLI): run a full recovery and process the corrections in-process,
//! reporting what was embedded and deleted.

use std::path::Path;
use std::sync::Arc;

use crate::embeddings::pipeline::{
    embed_entry_vector, embed_feed, embed_topic_vector, sync_entry_row, sync_topic_row,
};
use crate::embeddings::{Embedder, EmbeddingsDb};
use crate::types::{Article, Entry, ListItem, Topic};

use super::embeddings::repair_item_embeddings;
use super::{RepairResidue, RepairSink};

/// Summary of a completed recovery run.
pub struct RecoveryReport {
    pub embedded: usize,
    pub deleted: usize,
    pub errors: Vec<String>,
}

/// Sink for the CLI path. The in-process embed work is async, so it can't run inside the sync walk
/// visitor; this collects the items and processes them after the walk completes.
#[derive(Default)]
struct InlineSink {
    topics: Vec<Topic>,
    articles: Vec<Article>,
}

impl RepairSink for InlineSink {
    fn topic(&mut self, topic: Topic) {
        self.topics.push(topic);
    }

    fn article(&mut self, article: Article) {
        self.articles.push(article);
    }
}

/// Executes a full recovery: scans the filesystem and reconciles topics and articles against
/// LanceDB, applying the corrections (embeds + deletes) in-process.
///
/// Designed to be called from the CLI (`xaga embeddings recover` / `reindex`).
/// In the Tauri app, use `dispatch::dispatch_recovery` in the embedding service instead.
pub async fn run_full_recovery(
    library_dir: &Path,
    embedder: &Arc<Embedder>,
    db: &EmbeddingsDb,
    writer: &crate::library_writer::LibraryWriter,
) -> RecoveryReport {
    let mut report = RecoveryReport {
        embedded: 0,
        deleted: 0,
        errors: Vec::new(),
    };

    let mut sink = InlineSink::default();

    match repair_item_embeddings(library_dir, db, &mut sink, Some(writer)).await {
        Ok(residue) => process_inline(sink, residue, embedder, db, &mut report).await,
        Err(e) => report
            .errors
            .push(format!("repair_item_embeddings failed: {e}")),
    }

    report
}

// Unlike `dispatch`, this processes the corrections inline (synchronously), after the walk.
// Intended for use in CLI, not GUI.
async fn process_inline(
    sink: InlineSink,
    residue: RepairResidue,
    embedder: &Arc<Embedder>,
    db: &EmbeddingsDb,
    report: &mut RecoveryReport,
) {
    for id in &residue.to_delete {
        match db.delete_by_id(id).await {
            Ok(()) => report.deleted += 1,
            Err(e) => report.errors.push(format!("delete {id}: {e}")),
        }
    }

    // Recovery runs at startup; suppress change events to avoid flooding the UI.
    let (noop_unread_tx, _) = flume::unbounded::<Entry>();
    let (noop_bookmarked_tx, _) = flume::unbounded::<Entry>();
    let (noop_annotation_tx, _) = flume::unbounded::<ListItem>();

    // The domain object carries everything sync/embed need; no file is re-read here.
    for topic in &sink.topics {
        if let Err(e) = sync_topic_row(topic, db).await {
            report.errors.push(format!("sync topic {}: {e}", topic.id));

            continue;
        }

        match embed_topic_vector(topic, embedder, db).await {
            Ok(()) => report.embedded += 1,
            Err(e) => report.errors.push(format!("embed topic {}: {e}", topic.id)),
        }
    }

    for article in &sink.articles {
        if let Err(e) = sync_entry_row(
            article,
            db,
            &noop_unread_tx,
            &noop_bookmarked_tx,
            &noop_annotation_tx,
        )
        .await
        {
            report
                .errors
                .push(format!("sync article {}: {e}", article.meta.id));

            continue;
        }

        match embed_entry_vector(article, embedder, db).await {
            Ok(()) => report.embedded += 1,
            Err(e) => report
                .errors
                .push(format!("embed article {}: {e}", article.meta.id)),
        }
    }

    for feed in &residue.to_embed_feeds {
        match embed_feed(feed, embedder, db).await {
            Ok(()) => report.embedded += 1,
            Err(e) => report.errors.push(format!("embed feed {}: {e}", feed.id)),
        }
    }
}
