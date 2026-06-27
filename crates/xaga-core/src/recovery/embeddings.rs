//! The diff stage: scan the filesystem, compare against the index, and produce `EmbeddingsReconResults`
//! of what to embed and what to delete. Pure computation — the reads are delegated to persistence
//! (`scan_dir`/`parse_document`) and embeddings (`all_entries_meta`).

use std::collections::HashSet;
use std::path::Path;

use crate::embeddings::row::{article_embed_text, checksum, feed_embed_text, topic_embed_text};
use crate::embeddings::{EmbeddingsDb, EmbeddingsError};
use crate::fs::scan_md_dir;
use crate::md::{md_to_article, md_to_topic};
use crate::persistence::entries::entry_dirs;
use crate::types::{Article, Feed, Kind, Topic};

use super::EmbeddingsReconResults;

/// Scans `{data_dir}/topics/` and `{data_dir}/entries/` and compares against LanceDB. Topics
/// appear before articles so they are indexed first.
///
/// Call `reconcile_feed_entries` before this so that entries exist on disk before the entries
/// directory is scanned.
pub(crate) async fn reconcile_item_embeddings(
    data_dir: &Path,
    db: &EmbeddingsDb,
) -> Result<EmbeddingsReconResults, EmbeddingsError> {
    let db_meta = db.all_entries_meta().await?; // id → (checksum, reference, has_vector, kind)

    let mut all_topics: Vec<Topic> = Vec::new();
    let mut all_articles: Vec<Article> = Vec::new();

    // Scan file system for .md and map them to domain objects

    scan_md_dir(&data_dir.join("topics"), &mut all_topics, &md_to_topic).await;

    for dir in entry_dirs(data_dir) {
        scan_md_dir(&dir, &mut all_articles, &md_to_article).await;
    }

    // File system id-set for to_delete, captured before the objects are moved into batches.

    let fs_ids: HashSet<String> = all_topics
        .iter()
        .map(|t| t.id.clone())
        .chain(all_articles.iter().map(|a| a.meta.id.clone()))
        .collect();

    // Identify items that need to be embedded
    // Embedding is needed when the row is absent, its content changed, or its vector is missing.

    let needs_embedding = |id: &str, cksum: &str| -> bool {
        match db_meta.get(id) {
            None => true,
            Some((stored, _, has_vector, _)) => stored != cksum || !has_vector,
        }
    };

    let to_embed_topics: Vec<Topic> = all_topics
        .into_iter()
        .filter(|t| needs_embedding(&t.id, &checksum(&t.title, topic_embed_text(t))))
        .collect();

    let to_embed_articles: Vec<Article> = all_articles
        .into_iter()
        .filter(|a| needs_embedding(&a.meta.id, &checksum(&a.meta.title, article_embed_text(a))))
        .collect();

    // Feeds: diff like items — re-embed a feed only when its row is absent, its checksum changed,
    // or its vector is pending. A feed row is stale when its feed is gone from disk.

    let empty = HashSet::new();

    let all_feeds = crate::persistence::feeds::list_feeds(&data_dir.join("feeds"), &empty, &empty)
        .await
        .unwrap_or_default();

    let feed_ids: HashSet<String> = all_feeds.iter().map(|f| f.id.clone()).collect();

    let to_embed_feeds: Vec<Feed> = all_feeds
        .into_iter()
        .filter(|f| needs_embedding(&f.id, &checksum(&f.title, feed_embed_text(f))))
        .collect();

    // Build to_delete: in LanceDB but no longer on disk. Item rows: the `.md` is gone (annotation
    // rows carry a non-empty `reference` to their parent article — check that). Feed rows: the feed
    // is gone from disk.
    let to_delete: Vec<String> = db_meta
        .into_iter()
        .filter(|(id, (_, reference, _, kind))| {
            if kind == Kind::Feed.as_str() {
                return !feed_ids.contains(id);
            }

            let file_id = if reference.is_empty() {
                id.as_str()
            } else {
                reference.as_str()
            };

            !fs_ids.contains(file_id)
        })
        .map(|(id, _)| id)
        .collect();

    Ok(EmbeddingsReconResults {
        to_embed_topics,
        to_embed_articles,
        to_embed_feeds,
        to_delete,
    })
}
