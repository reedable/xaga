use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use flume;

use crate::articles::article_from_document;
use crate::embeddings::pipeline::{
    embed_entry_vector, embed_topic_vector, sync_entry_row, sync_topic_row,
};
use crate::embeddings::row::{article_embed_text, checksum, topic_embed_text};
use crate::embeddings::{Embedder, EmbeddingsDb, EmbeddingsError};
use crate::persistence::frontmatter::{parse_document, Frontmatter};
use crate::topics::topic_from_document;
use crate::types::{Article, Entry, Kind, SavedItem, Topic};

// ── Public types ──────────────────────────────────────────────────────────────

/// The output of a recovery scan: what needs to happen to bring LanceDB back into sync with the
/// filesystem. Recovery is the filesystem-reading producer: it parses each file **once** here and
/// carries the domain object (`Article`/`Topic`) so the embedding pass never re-reads.
pub struct RecoveryPlan {
    /// Topics missing from LanceDB or with changed checksums.
    /// Separated so callers can route them to a higher-priority channel.
    pub to_embed_topics: Vec<Topic>,
    /// Articles missing from LanceDB or with changed checksums.
    pub to_embed_articles: Vec<Article>,
    /// IDs of LanceDB rows whose `.md` file no longer exists on disk.
    pub to_delete: Vec<String>,
}

// ── Directory scanner ─────────────────────────────────────────────────────────

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

// ── Public plan builder ───────────────────────────────────────────────────────

/// Scans `{data_dir}/topics/` and `{data_dir}/entries/` and compares against LanceDB. Topics
/// appear before articles so they are indexed first.
///
/// Call `reconcile_feed_entries` before this so that entries exist on disk before the entries
/// directory is scanned.
pub async fn build_recovery_plan(
    data_dir: &Path,
    db: &EmbeddingsDb,
) -> Result<RecoveryPlan, EmbeddingsError> {
    let db_meta = db.all_entries_meta().await?; // id → (checksum, reference, has_vector, kind)

    // Scan topics first, then entries.
    let mut topic_raw: Vec<(String, PathBuf, Frontmatter, String)> = Vec::new();
    scan_dir(&data_dir.join("topics"), &mut topic_raw).await;

    let mut article_raw: Vec<(String, PathBuf, Frontmatter, String)> = Vec::new();
    for dir in crate::entries::entry_dirs(data_dir) {
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

    // Build to_delete: in LanceDB but not found in either directory.
    // Annotation rows carry a non-empty `reference` (parent article ID); check the reference
    // against the filesystem. Feed rows are managed by embed_feeds, never deleted here.
    let to_delete: Vec<String> = db_meta
        .into_iter()
        .filter(|(id, (_, reference, _, kind))| {
            if kind == Kind::Feed.as_str() {
                return false;
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

    Ok(RecoveryPlan {
        to_embed_topics,
        to_embed_articles,
        to_delete,
    })
}

// ── Feed entry reconciliation ─────────────────────────────────────────────────

/// For every entry in one `.atom` file, ensure an Entry exists on disk. Returns the `Article`s it
/// newly created, so the caller can enqueue their indexing. Called during recovery (per feed) and
/// by `refresh_feed` after a fetch.
pub async fn reconcile_atom_file(data_dir: &Path, feed_id: &str) -> Vec<Article> {
    let mut created: Vec<Article> = Vec::new();
    let atom_path = data_dir.join("feeds").join(format!("{feed_id}.atom"));

    let entries = match crate::feeds::feed_entry::entries_from_path(&atom_path).await {
        Ok(e) => e,
        Err(e) => {
            log::warn!("reconcile_atom_file {feed_id}: {e}");
            return created;
        }
    };

    for entry in &entries {
        if entry.url.is_empty() {
            continue;
        }

        let normalized = match crate::util::normalize_url(&entry.url) {
            Ok(u) => u,
            Err(e) => {
                log::warn!("reconcile_atom_file {feed_id}: bad url {}: {e}", entry.url);
                continue;
            }
        };

        let hash12 = crate::util::url_hash12(&normalized);

        if crate::entries::find_entry_path(data_dir, &hash12)
            .await
            .is_some()
        {
            tokio::task::yield_now().await;
            continue;
        }

        match crate::entries::write_entry(
            data_dir,
            &entry.url,
            &entry.title,
            entry.published_at.as_deref(),
            entry.updated_at.as_deref(),
            entry.description.as_deref(),
        )
        .await
        {
            Ok(article) => created.push(article),
            Err(e) => log::warn!("reconcile_atom_file {feed_id}: {e}"),
        }

        tokio::task::yield_now().await;
    }

    created
}

/// For every feed, reconcile its atom file. Calls `reconcile_atom_file` per
/// feed so the logic is never duplicated.
pub async fn reconcile_feed_entries(data_dir: &Path) {
    let feeds_dir = data_dir.join("feeds");
    let mut dir = match tokio::fs::read_dir(&feeds_dir).await {
        Ok(d) => d,
        Err(_) => return,
    };
    while let Ok(Some(entry)) = dir.next_entry().await {
        let path = entry.path();
        let name = path.to_string_lossy();

        if name.ends_with(".atom") && !name.ends_with(".atom.tmp") {
            if let Some(id) = path.file_stem().and_then(|s| s.to_str()) {
                reconcile_atom_file(data_dir, id).await;
            }
        }
    }
}

// ── Full recovery (CLI) ───────────────────────────────────────────────────────

/// Summary of a completed recovery run.
pub struct RecoveryReport {
    pub embedded: usize,
    pub deleted: usize,
    pub errors: Vec<String>,
}

/// Executes a full recovery: reconciles topics and articles against LanceDB.
///
/// Order of operations:
/// 1. Create missing `.md` for feed entries (articles must exist before scanning).
/// 2. Build and apply the unified recovery plan (topics embedded before articles).
///
/// Designed to be called from the CLI (`xaga embeddings recover` / `reindex`).
/// In the Tauri app, use `dispatch_recovery` in the embedding service instead.
pub async fn run_full_recovery(
    data_dir: &Path,
    embedder: &Arc<Embedder>,
    db: &EmbeddingsDb,
) -> RecoveryReport {
    let mut report = RecoveryReport {
        embedded: 0,
        deleted: 0,
        errors: Vec::new(),
    };

    reconcile_feed_entries(data_dir).await;

    match build_recovery_plan(data_dir, db).await {
        Ok(plan) => apply_plan(plan, embedder, db, &mut report).await,
        Err(e) => report
            .errors
            .push(format!("build_recovery_plan failed: {e}")),
    }

    report
}

async fn apply_plan(
    plan: RecoveryPlan,
    embedder: &Arc<Embedder>,
    db: &EmbeddingsDb,
    report: &mut RecoveryReport,
) {
    for id in &plan.to_delete {
        match db.delete_by_id(id).await {
            Ok(()) => report.deleted += 1,
            Err(e) => report.errors.push(format!("delete {id}: {e}")),
        }
    }
    // Recovery runs at startup; suppress change events to avoid flooding the UI.
    let (noop_unread_tx, _) = flume::unbounded::<Entry>();
    let (noop_bookmarked_tx, _) = flume::unbounded::<Entry>();
    let (noop_annotation_tx, _) = flume::unbounded::<SavedItem>();

    // The domain object carries everything sync/embed need; no file is re-read here.
    for topic in &plan.to_embed_topics {
        if let Err(e) = sync_topic_row(topic, db).await {
            report.errors.push(format!("sync topic {}: {e}", topic.id));
            continue;
        }
        match embed_topic_vector(topic, embedder, db).await {
            Ok(()) => report.embedded += 1,
            Err(e) => report.errors.push(format!("embed topic {}: {e}", topic.id)),
        }
    }
    for article in &plan.to_embed_articles {
        if let Err(e) =
            sync_entry_row(article, db, &noop_unread_tx, &noop_bookmarked_tx, &noop_annotation_tx).await
        {
            report.errors.push(format!("sync article {}: {e}", article.meta.id));
            continue;
        }
        match embed_entry_vector(article, embedder, db).await {
            Ok(()) => report.embedded += 1,
            Err(e) => report.errors.push(format!("embed article {}: {e}", article.meta.id)),
        }
    }
}
