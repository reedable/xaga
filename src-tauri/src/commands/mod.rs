pub mod articles;
pub mod bookmarks;
pub mod entries;
pub mod embeddings;
pub mod feeds;
pub mod library;
pub mod saved;
pub mod settings;
pub mod topics;

/// Reject any IPC call that does not originate from the main UI window.
/// Prevents malicious JS loaded in a loader webview from calling commands
/// it has no business calling.
pub(crate) fn require_main_window(window: &tauri::Window) -> Result<(), String> {
    if window.label().starts_with("loader-") {
        return Err("unauthorized: command not available from loader context".to_string());
    }
    Ok(())
}

/// Reject any IPC call that does not originate from a loader webview.
/// `receive_html` must only be called by our own loader windows, never by
/// the main UI.
pub(crate) fn require_loader_window(window: &tauri::Window) -> Result<(), String> {
    if !window.label().starts_with("loader-") {
        return Err("unauthorized: command only available from loader context".to_string());
    }
    Ok(())
}

pub(crate) fn parse_item_filter(
    item_filter: Option<String>,
) -> Result<xaga_core::types::ItemFilter, String> {
    item_filter
        .as_deref()
        .map(|s| s.parse::<xaga_core::types::ItemFilter>())
        .transpose()
        .map_err(|e| e)?
        .map_or(Ok(Default::default()), Ok)
}
