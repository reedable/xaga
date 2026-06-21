use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::content::Content;
use crate::embeddings::{Embedder, EmbeddingsDb, EmbeddingsError, embed_file};
use crate::types::{Entry, Kind, SavedItem};
use flume;

// ── Public types ──────────────────────────────────────────────────────────────

/// The output of a recovery scan: what needs to happen to bring LanceDB back
/// into sync with the filesystem. Recovery is the filesystem-reading producer: it parses each
/// file **once** here and carries the `Content` so the embedding pass never re-reads.
pub struct RecoveryPlan {
    /// Topic `.md` payloads missing from LanceDB or with changed checksums.
    /// Separated so callers can route them to a higher-priority channel.
    pub to_embed_topics: Vec<Content>,
    /// Article `.md` payloads missing from LanceDB or with changed checksums.
    pub to_embed_articles: Vec<Content>,
    /// IDs of LanceDB rows whose `.md` file no longer exists on disk.
    pub to_delete: Vec<String>,
}

// ── Directory scanner ─────────────────────────────────────────────────────────

/// Scans one directory of `.md` files and appends discovered items into `fs_entries`
/// (id → (path, parsed `Content`)). The single parse recovery performs; the `Content` is carried
/// into the embed pass. Yields between reads to avoid starving higher-priority tasks.
async fn scan_dir(dir: &Path, fs_entries: &mut HashMap<String, (PathBuf, Content)>) {
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
        let content = match crate::persistence::frontmatter::parse_content(id.clone(), &raw) {
            Some(c) => c,
            None => continue,
        };

        fs_entries.insert(id, (path, content));
        tokio::task::yield_now().await;
    }
}

// ── Public plan builder ───────────────────────────────────────────────────────

/// Scans `{data_dir}/topics/` and `{data_dir}/entries/` and compares against
/// LanceDB.  Topics entries appear first in `to_embed` so they are indexed
/// before entries.
///
/// Call `reconcile_feed_entries` before this so that Entry exist on
/// disk before the articles directory is scanned.
pub async fn build_recovery_plan(
    data_dir: &Path,
    db: &EmbeddingsDb,
) -> Result<RecoveryPlan, EmbeddingsError> {
    let db_meta = db.all_entries_meta().await?; // id → (checksum, reference, has_vector, kind)

    // Scan topics first, then articles, so to_embed preserves that order.
    let mut topic_entries: HashMap<String, (PathBuf, Content)> = HashMap::new();
    scan_dir(&data_dir.join("topics"), &mut topic_entries).await;

    let mut article_entries: HashMap<String, (PathBuf, Content)> = HashMap::new();
    for dir in crate::entries::entry_dirs(data_dir) {
        scan_dir(&dir, &mut article_entries).await;
    }

    // Build to_embed: on disk but absent or changed in LanceDB. The parsed `Content` is carried
    // forward (no re-read). Each group sorted newest-modified first.
    let make_batch = |entries: &HashMap<String, (PathBuf, Content)>| -> Vec<Content> {
        let mut batch: Vec<&(PathBuf, Content)> = entries
            .values()
            .filter(|(_, content)| {
                // Re-embed when the row is absent, its content changed, or its vector is
                // still pending (the row was inserted before the embedding was computed).
                match db_meta.get(content.id.as_str()) {
                    None => true,
                    Some((stored_checksum, _, has_vector, _)) => {
                        stored_checksum != &content.content_checksum() || !has_vector
                    }
                }
            })
            .collect();
        batch.sort_by(|(a, _), (b, _)| {
            let mt_a = std::fs::metadata(a).and_then(|m| m.modified()).ok();
            let mt_b = std::fs::metadata(b).and_then(|m| m.modified()).ok();
            mt_b.cmp(&mt_a)
        });
        batch.into_iter().map(|(_, content)| content.clone()).collect()
    };
    let to_embed_topics = make_batch(&topic_entries);
    let to_embed_articles = make_batch(&article_entries);

    // Build to_delete: in LanceDB but not found in either directory.
    let fs_ids: HashSet<&str> = topic_entries
        .keys()
        .chain(article_entries.keys())
        .map(String::as_str)
        .collect();
    // Annotation rows carry a non-empty `reference` (parent article ID).
    // For those, check the reference against the filesystem; for all other
    // rows, check the row's own ID.
    let to_delete: Vec<String> = db_meta
        .into_iter()
        .filter(|(id, (_, reference, _, kind))| {
            // Feed rows are managed by embed_feeds, not the entry/topic plan — never
            // delete them here (their ids are not in the entries/topics filesystem scan).
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

/// For every entry in one `.atom` file, ensure an Entry exists on disk. Returns the ids of
/// the entries it newly created, so the caller can enqueue their indexing. Called during
/// recovery (per feed) and by `refresh_feed` after a fetch.
pub async fn reconcile_atom_file(data_dir: &Path, feed_id: &str) -> Vec<Content> {
    let mut created: Vec<Content> = Vec::new();
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
            Ok(content) => created.push(content),
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

    // The `Content` carries everything embed_file needs; no file is re-read here.
    for content in plan.to_embed_topics.iter().chain(plan.to_embed_articles.iter()) {
        let label = if content.kind == Kind::Topic { "topic" } else { "article" };
        match embed_file(content, embedder, db, &noop_unread_tx, &noop_bookmarked_tx, &noop_annotation_tx).await {
            Ok(()) => report.embedded += 1,
            Err(e) => report.errors.push(format!("embed {label} {}: {e}", content.id)),
        }
    }
}
