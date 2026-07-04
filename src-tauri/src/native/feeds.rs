//! Feed OPML import/export, orchestrated entirely on the native side.
//!
//! These are NOT `#[tauri::command]`s: they take no path from the webview.
//! The menu handler opens the file dialog in Rust (`super::import_feeds_dialog`
//! / `export_feeds_dialog`) and calls these directly, so a compromised renderer
//! has no invoke door to reach them (see docs/architecture/security/command_surface.md).
//! The write-then-enqueue orchestration stays in this layer per
//! docs/architecture/layer_ownership.md.

use std::collections::HashSet;
use std::path::Path;

use serde::Serialize;
use xaga_core::embeddings::{Job, Tier};
use xaga_core::persistence::feeds;
use xaga_core::types::Feed;

use crate::{AppState, get_bookmarked_ids, get_unread_ids};

#[derive(Serialize, Clone)]
pub(crate) struct ImportResult {
    pub added: Vec<Feed>,
    pub skipped: Vec<String>,
    pub failed: Vec<String>,
}

pub(crate) async fn import_feeds(
    state: &AppState,
    file_path: &Path,
) -> Result<ImportResult, String> {
    let library = state.library.read().unwrap().clone();

    let max_bytes = *state.import_max_bytes.read().unwrap();
    let metadata = tokio::fs::metadata(file_path)
        .await
        .map_err(|e| e.to_string())?;
    if metadata.len() > max_bytes {
        return Err(format!(
            "import file is {} bytes, exceeding the {} byte limit",
            metadata.len(),
            max_bytes
        ));
    }

    let xml = tokio::fs::read_to_string(file_path)
        .await
        .map_err(|e| e.to_string())?;

    let raw_urls = xaga_core::persistence::opml::parse_opml(&xml).map_err(|e| e.to_string())?;

    let timeout_secs = *state.feed_timeout_secs.read().unwrap();
    let max_body_bytes = *state.feed_max_body_bytes.read().unwrap();
    let client = crate::fetch_client::build(
        state.ssrf_proxy_port,
        *state.max_redirects.read().unwrap(),
        *state.referer_allowed.read().unwrap(),
        *state.gzip_allowed.read().unwrap(),
    );
    let (unread, bookmarked) = (get_unread_ids(state).await, get_bookmarked_ids(state).await);
    let mut seen: HashSet<String> = HashSet::new();
    let mut added: Vec<Feed> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();
    let mut failed: Vec<String> = Vec::new();

    for raw in raw_urls {
        let normalized = match xaga_core::util::normalize_url(&raw) {
            Ok(u) => u,
            Err(e) => {
                log::warn!("import_feeds: invalid URL {raw:?}: {e}");
                failed.push(raw);
                continue;
            }
        };

        if !seen.insert(normalized.clone()) {
            skipped.push(normalized);
            continue;
        }

        match feeds::add_feed(
            &client,
            &library,
            &normalized,
            &unread,
            &bookmarked,
            timeout_secs,
            max_body_bytes,
        )
        .await
        {
            Ok((feed, created)) => {
                // User-initiated CRUD (OPML import): the user is waiting, so the entry upsert
                // rides Interactive (T1); its embedding follows to Regular via `embed_follow`,
                // per the priority definitions in the service's `run_loop`.
                for article in created {
                    state.enqueue(Tier::Interactive, Job::UpsertEntry { article });
                }
                added.push(feed);
            }
            Err(e) => {
                log::warn!("import_feeds: failed to add {normalized:?}: {e}");
                failed.push(normalized);
            }
        }
    }

    Ok(ImportResult {
        added,
        skipped,
        failed,
    })
}

pub(crate) async fn export_feeds(state: &AppState, file_path: &Path) -> Result<(), String> {
    let library = state.library.read().unwrap().clone();

    let (unread, bookmarked) = (get_unread_ids(state).await, get_bookmarked_ids(state).await);
    let feed_list = feeds::list_feeds(&library, &unread, &bookmarked)
        .await
        .map_err(|e| e.to_string())?;

    let pairs: Vec<(&str, &str)> = feed_list
        .iter()
        .map(|f| (f.url.as_str(), f.title.as_str()))
        .collect();

    let opml = xaga_core::persistence::opml::build_opml(&pairs);

    tokio::fs::write(file_path, opml)
        .await
        .map_err(|e| e.to_string())
}
