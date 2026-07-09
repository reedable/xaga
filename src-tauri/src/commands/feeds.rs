use tauri::{Emitter, State};
use xaga_core::embeddings::{EntityRef, Job, StatePatch, Tier};
use xaga_core::persistence::feeds;
use xaga_core::types::{Entry, Feed, Kind};

use super::parse_item_filter;
use crate::{AppState, get_bookmarked_ids, get_unread_ids};

#[tauri::command]
pub async fn list_feeds(state: State<'_, AppState>) -> Result<Vec<Feed>, String> {
    let library = state.library.read().unwrap().clone();
    let (unread, bookmarked) = (
        get_unread_ids(&state).await,
        get_bookmarked_ids(&state).await,
    );
    feeds::list_feeds(&library, &unread, &bookmarked)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_feed(url: String, state: State<'_, AppState>) -> Result<Feed, String> {
    let library = state.library.read().unwrap().clone();
    let timeout_secs = *state.feed_timeout_secs.read().unwrap();
    let max_body_bytes = *state.feed_max_body_bytes.read().unwrap();
    let client = crate::fetch_client::build(
        state.ssrf_proxy_port,
        *state.max_redirects.read().unwrap(),
        *state.referer_allowed.read().unwrap(),
        *state.gzip_allowed.read().unwrap(),
    );
    let (unread, bookmarked) = (
        get_unread_ids(&state).await,
        get_bookmarked_ids(&state).await,
    );
    let (feed, created) = feeds::add_feed(
        &client,
        &library,
        &url,
        &unread,
        &bookmarked,
        timeout_secs,
        max_body_bytes,
    )
    .await
    .map_err(|e| e.to_string())?;
    // User-initiated CRUD: the user invoked add_feed and is waiting on the result, so the
    // entry upsert rides Interactive (T1). Its entry embedding then follows to Regular via
    // `embed_follow`, per the priority definitions in the embedding service's `run_loop`.
    for article in created {
        state.enqueue(Tier::Interactive, Job::UpsertEntry { article });
    }
    Ok(feed)
}

#[tauri::command]
pub async fn refresh_all_feeds(
    window: tauri::Window,
    state: State<'_, AppState>,
) -> Result<Vec<Feed>, String> {
    let library = state.library.read().unwrap().clone();
    let timeout_secs = *state.feed_timeout_secs.read().unwrap();
    let max_body_bytes = *state.feed_max_body_bytes.read().unwrap();
    let client = crate::fetch_client::build(
        state.ssrf_proxy_port,
        *state.max_redirects.read().unwrap(),
        *state.referer_allowed.read().unwrap(),
        *state.gzip_allowed.read().unwrap(),
    );
    let (unread, bookmarked) = (
        get_unread_ids(&state).await,
        get_bookmarked_ids(&state).await,
    );
    let results = feeds::refresh_all_feeds(
        &client,
        &library,
        &unread,
        &bookmarked,
        timeout_secs,
        max_body_bytes,
    )
    .await;
    let mut refreshed = Vec::new();

    for (id, result) in results {
        match result {
            Ok((feed, created)) => {
                // User-initiated CRUD: the user clicked Refresh and is waiting, so the entry
                // upsert rides Interactive (T1); its embedding follows to Regular via
                // `embed_follow`, per the priority definitions in the service's `run_loop`.
                for article in created {
                    state.enqueue(Tier::Interactive, Job::UpsertEntry { article });
                }

                window.emit("feed:refreshed", &feed).ok();
                refreshed.push(feed);
            }

            Err(e) => log::warn!("refresh_all_feeds {id}: {e}"),
        }
    }

    Ok(refreshed)
}

#[tauri::command]
pub async fn set_feed_pinned(
    id: String,
    pinned: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let library = state.library.read().unwrap().clone();
    let changed = feeds::set_feed_pinned(&library, &id, pinned)
        .await
        .map_err(|e| e.to_string())?;

    if changed {
        state.enqueue(
            Tier::Interactive,
            Job::SyncState {
                entity: EntityRef::new(id, Kind::Feed),
                state: StatePatch {
                    is_pinned: Some(pinned),
                    ..Default::default()
                },
            },
        );
    }

    Ok(())
}

#[tauri::command]
pub async fn rename_feed(
    id: String,
    title: String,
    state: State<'_, AppState>,
) -> Result<Feed, String> {
    let library = state.library.read().unwrap().clone();
    let (unread, bookmarked) = (
        get_unread_ids(&state).await,
        get_bookmarked_ids(&state).await,
    );

    let feed = feeds::rename_feed(&library, &id, &title, &unread, &bookmarked)
        .await
        .map_err(|e| e.to_string())?;

    // Re-embed the feed row (title changed). `rename_feed` already returns the domain `Feed`;
    // carry it on the job so the consumer embeds without touching the filesystem.
    state.enqueue(Tier::Regular, Job::EmbedFeed { feed: feed.clone() });

    Ok(feed)
}

#[tauri::command]
pub async fn remove_feed(url: String, state: State<'_, AppState>) -> Result<String, String> {
    let library = state.library.read().unwrap().clone();
    let feed_id = feeds::url_to_feed_id(&url).map_err(|e| e.to_string())?;
    feeds::remove_feed(&library, &feed_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(url)
}

#[tauri::command]
pub async fn list_entries(
    url: Option<String>,
    item_filter: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<Entry>, String> {
    let library = state.library.read().unwrap().clone();
    let item_filter = parse_item_filter(item_filter)?;

    // Phase C: the .atom is the source of truth for feed membership (the id-set); LanceDB
    // returns those entries with their state. Reload the read handle so state reflects the
    // service's latest writes, then filter/sort as before.
    let ids = feeds::list_entry_ids(&library, url.as_deref())
        .await
        .map_err(|e| e.to_string())?;

    let db = state.embeddings_read.lock().unwrap().clone();
    let Some(db) = db else {
        return Err("embeddings read handle unavailable".to_string());
    };

    let entries = db
        .list_entries_by_ids(&ids)
        .await
        .map_err(|e| e.to_string())?;
    let mut items = xaga_core::query::filter::filter_items(entries, &item_filter)
        .await
        .map_err(|e| e.to_string())?;
    xaga_core::query::filter::sort_items(&mut items);
    Ok(items)
}
