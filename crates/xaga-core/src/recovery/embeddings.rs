//! The diff stage: scan the filesystem, compare against the index, and produce `EmbeddingsReconResults`
//! of what to embed and what to delete. Pure computation — the reads are delegated to persistence
//! (`scan_dir`/`parse_document`) and embeddings (`all_entries_meta`).

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::embeddings::row::{article_embed_text, checksum, feed_embed_text, topic_embed_text};
use crate::embeddings::{EmbeddingsDb, EmbeddingsError};
use crate::persistence::articles::article_from_document;
use crate::persistence::frontmatter::{Frontmatter, parse_document};
use crate::persistence::topics::topic_from_document;
use crate::types::{Article, Feed, Kind, Topic};

use super::EmbeddingsReconResults;

/// Scans one directory of `.md` files and appends each parsed document into `out`
/// (id, path, frontmatter, body). The single parse recovery performs; the caller turns these into
/// domain objects. Yields between reads to avoid starving higher-priority tasks.
async fn scan_dir(dir: &Path, out: &mut Vec<(String, PathBuf, Frontmatter, String)>) {
    if !dir.exists() {
        return;
    }

    let mut entries = match tokio::fs::read_dir(dir).await {
        Ok(e) => e,
        Err(_) => return,
    };

    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();

        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }

        let raw = match tokio::fs::read_to_string(&path).await {
            Ok(r) => r,
            Err(_) => continue,
        };

        let id = crate::util::item_id(&path);

        let (fm, body) = match parse_document(&raw) {
            Some(x) => x,
            None => continue,
        };

        out.push((id, path, fm, body));
        tokio::task::yield_now().await;
    }
}

/// Sorts a `(path, item)` batch newest-modified first (in place).
fn sort_by_mtime_desc<T>(batch: &mut [(PathBuf, T)]) {
    batch.sort_by(|(a, _), (b, _)| {
        let mt_a = std::fs::metadata(a).and_then(|m| m.modified()).ok();
        let mt_b = std::fs::metadata(b).and_then(|m| m.modified()).ok();
        mt_b.cmp(&mt_a)
    });
}

/// Scans `{data_dir}/topics/` and `{data_dir}/entries/` and compares against LanceDB. Topics
/// appear before articles so they are indexed first.
///
/// Call `reconcile_feed_entries` before this so that entries exist on disk before the entries
/// directory is scanned.
pub async fn reconcile_item_embeddings(
    data_dir: &Path,
    db: &EmbeddingsDb,
) -> Result<EmbeddingsReconResults, EmbeddingsError> {
    let db_meta = db.all_entries_meta().await?; // id → (checksum, reference, has_vector, kind)

    // Scan topics first, then entries.
    let mut topic_raw: Vec<(String, PathBuf, Frontmatter, String)> = Vec::new();

    scan_dir(&data_dir.join("topics"), &mut topic_raw).await;

    let mut article_raw: Vec<(String, PathBuf, Frontmatter, String)> = Vec::new();

    for dir in crate::persistence::entries::entry_dirs(data_dir) {
        scan_dir(&dir, &mut article_raw).await;
    }

    // Build domain objects (parsed once); keep the path for the mtime sort.
    let topics: Vec<(PathBuf, Topic)> = topic_raw
        .into_iter()
        .map(|(id, path, fm, body)| (path, topic_from_document(&id, fm, body)))
        .collect();

    let articles: Vec<(PathBuf, Article)> = article_raw
        .into_iter()
        .map(|(id, path, fm, body)| (path, article_from_document(&id, fm, body)))
        .collect();

    // Filesystem id-set for to_delete, captured before the objects are moved into batches.
    let fs_ids: HashSet<String> = topics
        .iter()
        .map(|(_, t)| t.id.clone())
        .chain(articles.iter().map(|(_, a)| a.meta.id.clone()))
        .collect();

    // Re-embed when the row is absent, its content changed, or its vector is still pending.
    let needs = |id: &str, cksum: &str| -> bool {
        match db_meta.get(id) {
            None => true,
            Some((stored, _, has_vector, _)) => stored != cksum || !has_vector,
        }
    };

    let mut topic_batch: Vec<(PathBuf, Topic)> = topics
        .into_iter()
        .filter(|(_, t)| needs(&t.id, &checksum(&t.title, topic_embed_text(t))))
        .collect();

    sort_by_mtime_desc(&mut topic_batch);

    let to_embed_topics: Vec<Topic> = topic_batch.into_iter().map(|(_, t)| t).collect();

    let mut article_batch: Vec<(PathBuf, Article)> = articles
        .into_iter()
        .filter(|(_, a)| needs(&a.meta.id, &checksum(&a.meta.title, article_embed_text(a))))
        .collect();

    sort_by_mtime_desc(&mut article_batch);

    let to_embed_articles: Vec<Article> = article_batch.into_iter().map(|(_, a)| a).collect();

    // Feeds: diff like items — re-embed a feed only when its row is absent, its checksum changed,
    // or its vector is pending. A feed row is stale when its feed is gone from disk.
    let empty = HashSet::new();

    let all_feeds = crate::persistence::feeds::list_feeds(&data_dir.join("feeds"), &empty, &empty)
        .await
        .unwrap_or_default();

    let feed_ids: HashSet<String> = all_feeds.iter().map(|f| f.id.clone()).collect();

    let to_embed_feeds: Vec<Feed> = all_feeds
        .into_iter()
        .filter(|f| needs(&f.id, &checksum(&f.title, feed_embed_text(f))))
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
