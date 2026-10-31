use tauri::State;
use xaga_core::embeddings::{Job, Tier};
use xaga_core::library_writer::Tier as WriteTier;
use xaga_core::persistence::feeds;
use xaga_core::types::{Entry, Feed};

use super::parse_item_filter;
use crate::error::AppError;
use crate::{AppState, get_bookmarked_ids, get_unread_ids};

#[tauri::command]
pub async fn list_feeds(state: State<'_, AppState>) -> Result<Vec<Feed>, AppError> {
    let library = state.library_path();
    let (unread, bookmarked) = (
        get_unread_ids(&state).await,
        get_bookmarked_ids(&state).await,
    );
    feeds::list_feeds(&library, &unread, &bookmarked)
        .await
        .map_err(AppError::from)
}

#[tauri::command]
pub async fn add_feed(url: String, state: State<'_, AppState>) -> Result<Feed, AppError> {
    let library = state.library_path();
    let writer = crate::commands::require_writer(&state)?;
    let timeout_secs = *state.feed_timeout_secs.read().unwrap();
    let max_body_bytes = *state.feed_max_body_bytes.read().unwrap();
    let client = crate::fetch_client::build(
        state.network_proxy_port,
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
        &writer,
        &url,
        &unread,
        &bookmarked,
        timeout_secs,
        max_body_bytes,
    )
    .await
    .map_err(AppError::from)?;
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
) -> Result<Vec<Feed>, AppError> {
    let library = state.library_path();
    let writer = crate::commands::require_writer(&state)?;
    let timeout_secs = *state.feed_timeout_secs.read().unwrap();
    let max_body_bytes = *state.feed_max_body_bytes.read().unwrap();
    let client = crate::fetch_client::build(
        state.network_proxy_port,
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
        &writer,
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

                crate::events::emit_domain(
                    &window,
                    xaga_core::events::DomainEvent::FeedRefreshed(feed.clone()),
                );
                refreshed.push(feed);
            }

            Err(e) => log::warn!("refresh_all_feeds {id}: {e}"),
        }
    }

    // One aggregate signal after the whole refresh completes, so list views re-fetch once
    // instead of once per feed. The per-feed `feed:refreshed` above still drives the sidebar's
    // per-feed unread-count patch.
    crate::events::emit_domain(&window, xaga_core::events::DomainEvent::FeedsRefreshed);

    Ok(refreshed)
}

#[tauri::command]
pub async fn set_feed_pinned(
    id: String,
    pinned: bool,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    super::pins::set_pin(xaga_core::pins::PinTarget::Feed { id }, pinned, app, state).await
}

#[tauri::command]
pub async fn rename_feed(
    id: String,
    title: String,
    state: State<'_, AppState>,
) -> Result<Feed, AppError> {
    let writer = crate::commands::require_writer(&state)?;
    let (unread, bookmarked) = (
        get_unread_ids(&state).await,
        get_bookmarked_ids(&state).await,
    );

    let feed = writer
        .rename_feed(
            WriteTier::Interactive,
            id.clone(),
            title,
            unread,
            bookmarked,
        )
        .await
        .map_err(AppError::from)?;

    // Re-embed the feed row (title changed). `rename_feed` already returns the domain `Feed`;
    // carry it on the job so the consumer embeds without touching the filesystem.
    state.enqueue(Tier::Regular, Job::EmbedFeed { feed: feed.clone() });

    Ok(feed)
}

#[tauri::command]
pub async fn remove_feed(url: String, state: State<'_, AppState>) -> Result<String, AppError> {
    let writer = crate::commands::require_writer(&state)?;
    let feed_id = feeds::url_to_feed_id(&url).map_err(AppError::from)?;
    writer
        .remove_feed(WriteTier::Interactive, feed_id)
        .await
        .map_err(AppError::from)?;
    Ok(url)
}

#[tauri::command]
pub async fn list_entries(
    url: Option<String>,
    item_filter: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<Entry>, AppError> {
    let library = state.library_path();
    let item_filter = parse_item_filter(item_filter)?;

    // The .atom is the source of truth for feed membership (the id-set); LanceDB
    // returns those entries with their state. Reload the read handle so state reflects the
    // service's latest writes, then filter/sort as before.
    let ids = feeds::list_entry_ids(&library, url.as_deref())
        .await
        .map_err(AppError::from)?;

    let db = state.embeddings_read.lock().unwrap().clone();
    let Some(db) = db else {
        return Err(AppError::Message(
            "embeddings read handle unavailable".to_string(),
        ));
    };

    let entries = db.list_entries_by_ids(&ids).await.map_err(AppError::from)?;
    let mut items = xaga_core::query::filter::filter_items(entries, &item_filter)
        .await
        .map_err(|e| e.to_string())?;
    xaga_core::query::filter::sort_items(&mut items);
    Ok(items)
}
