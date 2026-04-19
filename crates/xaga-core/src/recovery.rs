use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::embeddings::{Embedder, EmbeddingsDb, EmbeddingsError, embed_file};
use crate::util::parse_document;

// ── Public types ──────────────────────────────────────────────────────────────

/// The output of a recovery scan: what needs to happen to bring LanceDB back
/// into sync with the filesystem.
pub struct RecoveryPlan {
    /// IDs of topic `.md` files missing from LanceDB or with changed checksums.
    /// Separated so callers can route them to a higher-priority channel.
    pub to_embed_topics: Vec<String>,
    /// IDs of article `.md` files missing from LanceDB or with changed checksums.
    pub to_embed_articles: Vec<String>,
    /// IDs of LanceDB rows whose `.md` file no longer exists on disk.
    pub to_delete: Vec<String>,
}

// ── Directory scanner ─────────────────────────────────────────────────────────

/// Scans one directory of `.md` files and appends discovered entries into
/// `fs_entries` (id → (path, checksum)).  Yields between reads to avoid
/// starving higher-priority tasks.
async fn scan_dir(dir: &Path, fs_entries: &mut HashMap<String, (PathBuf, String)>) {
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

        let (fm, body) = match parse_document(&raw) {
            Some(f) => f,
            None => continue,
        };

        let id = crate::util::item_id(&path);
        fs_entries.insert(id, (path, fm.content_checksum(&body)));
        tokio::task::yield_now().await;
    }
}

// ── Public plan builder ───────────────────────────────────────────────────────

/// Scans `{data_dir}/topics/` and `{data_dir}/articles/` and compares against
/// LanceDB.  Topics entries appear first in `to_embed` so they are indexed
/// before articles.
///
/// Call `reconcile_feed_entries` before this so that article stubs exist on
/// disk before the articles directory is scanned.
pub async fn build_recovery_plan(
    data_dir: &Path,
    db: &EmbeddingsDb,
) -> Result<RecoveryPlan, EmbeddingsError> {
    let db_meta = db.all_entries_meta().await?; // id → checksum

    // Scan topics first, then articles, so to_embed preserves that order.
    let mut topic_entries: HashMap<String, (PathBuf, String)> = HashMap::new();
    scan_dir(&data_dir.join("topics"), &mut topic_entries).await;

    let mut article_entries: HashMap<String, (PathBuf, String)> = HashMap::new();
    for dir in crate::entries::entry_dirs(data_dir) {
        scan_dir(&dir, &mut article_entries).await;
    }

    // Build to_embed: on disk but absent or changed in LanceDB.
    // Each group sorted newest-modified first.
    let make_batch = |entries: &HashMap<String, (PathBuf, String)>| -> Vec<String> {
        let mut batch: Vec<(&String, &PathBuf)> = entries
            .iter()
            .filter(|(id, (_, checksum))| db_meta.get(id.as_str()).map_or(true, |s| s != checksum))
            .map(|(id, (path, _))| (id, path))
            .collect();
        batch.sort_by(|(_, a), (_, b)| {
            let mt_a = std::fs::metadata(a).and_then(|m| m.modified()).ok();
            let mt_b = std::fs::metadata(b).and_then(|m| m.modified()).ok();
            mt_b.cmp(&mt_a)
        });
        batch.into_iter().map(|(id, _)| id.clone()).collect()
    };
    let to_embed_topics = make_batch(&topic_entries);
    let to_embed_articles = make_batch(&article_entries);

    // Build to_delete: in LanceDB but not found in either directory.
    let fs_ids: HashSet<&str> = topic_entries
        .keys()
        .chain(article_entries.keys())
        .map(String::as_str)
        .collect();
    // Annotation rows use composite IDs "{article_id}:{annotation_id}".
    // For those, check the article-ID prefix against the filesystem.
    let to_delete: Vec<String> = db_meta
        .into_keys()
        .filter(|id| {
            let article_id = id.split_once(':').map_or(id.as_str(), |(a, _)| a);
            !fs_ids.contains(article_id)
        })
        .collect();

    Ok(RecoveryPlan {
        to_embed_topics,
        to_embed_articles,
        to_delete,
    })
}

// ── Feed entry reconciliation ─────────────────────────────────────────────────

/// For every entry in one `.atom` file, ensure an article stub exists on disk.
/// Called both during full recovery (once per feed) and live when the file
/// watcher detects a change to a specific `.atom` file.
pub async fn reconcile_atom_file(data_dir: &Path, feed_id: &str) {
    let atom_path = data_dir.join("feeds").join(format!("{feed_id}.atom"));

    let entries = match crate::feeds::feed_entry::entries_from_path(&atom_path).await {
        Ok(e) => e,
        Err(e) => {
            log::warn!("reconcile_atom_file {feed_id}: {e}");
            return;
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

        if let Err(e) = crate::entries::write_entry(
            data_dir,
            &entry.url,
            &entry.title,
            entry.published_at.as_deref(),
            entry.updated_at.as_deref(),
            entry.summary.as_deref(),
        )
        .await
        {
            log::warn!("reconcile_atom_file {feed_id}: {e}");
        }

        tokio::task::yield_now().await;
    }
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
/// 1. Create missing `.md` stubs for feed entries (articles must exist before scanning).
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
        Ok(plan) => apply_plan(data_dir, plan, embedder, db, &mut report).await,
        Err(e) => report
            .errors
            .push(format!("build_recovery_plan failed: {e}")),
    }

    report
}

async fn apply_plan(
    data_dir: &Path,
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
    for id in &plan.to_embed_topics {
        let path = crate::topics::topic_path(data_dir, id);
        match embed_file(&path, embedder, db).await {
            Ok(()) => report.embedded += 1,
            Err(e) => report.errors.push(format!("embed topic {id}: {e}")),
        }
    }
    for id in &plan.to_embed_articles {
        match crate::entries::find_entry_path(data_dir, id).await {
            Some(path) => match embed_file(&path, embedder, db).await {
                Ok(()) => report.embedded += 1,
                Err(e) => report.errors.push(format!("embed article {id}: {e}")),
            },
            None => report.errors.push(format!("embed article {id}: not found")),
        }
    }
}
