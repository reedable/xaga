pub mod articles;
pub mod bookmarks;
pub mod entries;
pub mod embeddings;
pub mod feeds;
pub mod library;
pub mod saved;
pub mod settings;
pub mod topics;

/// Labels of the trusted first-party UI windows. Any window not in this set —
/// notably the `loader-*` webviews that navigate to untrusted remote content —
/// is denied access to the main command surface.
pub(crate) const TRUSTED_WINDOW_LABELS: &[&str] = &["main", "settings"];

/// Commands that may *only* be invoked from a loader webview (the off-screen
/// windows that render untrusted remote content). These are the callbacks the
/// loader's own init script uses to return captured HTML; no other window may
/// call them, and a loader window may call *nothing else*.
pub(crate) const LOADER_COMMANDS: &[&str] = &[
    "receive_html",
    "loader_will_unload",
    "loader_page_loaded",
    "capture_and_close_loader",
];

/// Central IPC access policy, enforced once in the global invoke handler so that
/// every command — present and future — is covered without a per-command guard:
///
/// * `loader-*` webviews (untrusted remote content) may call only
///   [`LOADER_COMMANDS`].
/// * Trusted UI windows ([`TRUSTED_WINDOW_LABELS`]) may call everything *except*
///   [`LOADER_COMMANDS`].
/// * Any other (unknown / future) window is denied outright — allowlist by
///   default.
pub(crate) fn is_invoke_allowed(label: &str, command: &str) -> bool {
    let is_loader_command = LOADER_COMMANDS.contains(&command);
    if label.starts_with("loader-") {
        is_loader_command
    } else if TRUSTED_WINDOW_LABELS.contains(&label) {
        !is_loader_command
    } else {
        false
    }
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

pub(crate) fn parse_item_sort(
    sort: Option<String>,
) -> Result<xaga_core::types::ItemSort, String> {
    sort
        .as_deref()
        .map(|s| s.parse::<xaga_core::types::ItemSort>())
        .transpose()
        .map_err(|e| e)?
        .map_or(Ok(Default::default()), Ok)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loader_windows_may_call_only_loader_commands() {
        assert!(is_invoke_allowed("loader-7", "receive_html"));
        assert!(is_invoke_allowed("loader-7", "capture_and_close_loader"));
        // Loader content must not reach the main command surface.
        assert!(!is_invoke_allowed("loader-7", "read_article"));
        assert!(!is_invoke_allowed("loader-7", "set_library"));
        assert!(!is_invoke_allowed("loader-7", "get_library"));
    }

    #[test]
    fn trusted_windows_may_call_everything_except_loader_commands() {
        for label in TRUSTED_WINDOW_LABELS {
            assert!(is_invoke_allowed(label, "read_article"));
            assert!(is_invoke_allowed(label, "set_library"));
            // The UI must never invoke a loader callback.
            assert!(!is_invoke_allowed(label, "receive_html"));
        }
    }

    #[test]
    fn unknown_windows_are_denied_by_default() {
        assert!(!is_invoke_allowed("evil", "read_article"));
        assert!(!is_invoke_allowed("evil", "receive_html"));
        assert!(!is_invoke_allowed("", "get_library"));
    }
}
