pub mod articles;
pub mod bookmarks;
pub mod embeddings;
pub mod entries;
pub mod feeds;
pub mod library;
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

/// Commands that write to the open library (`.md` entity files and/or LanceDB). A
/// read-only instance — one that did not win the library lock at startup — has these
/// rejected centrally in the global invoke handler with a typed `E_READ_ONLY` error, the
/// same single-choke-point pattern as [`is_invoke_allowed`]. Account/app-scoped writes
/// (`set_library`, `settings::set_*`) are NOT library writes and are not listed.
/// `read_article` is not listed either: its incidental mark-read write is skipped
/// in-command on a read-only instance so reading stays seamless.
pub(crate) const WRITE_COMMANDS: &[&str] = &[
    "save_article",
    "create_article",
    "add_annotation",
    "replace_annotation",
    "remove_annotation",
    "mark_entry_read",
    "mark_entry_unread",
    "add_feed",
    "refresh_all_feeds",
    "rename_feed",
    "remove_feed",
    "set_feed_pinned",
    "create_topic",
    "save_topic",
    "delete_topic",
    "add_topic_item",
    "remove_topic_item",
    "set_topic_pinned",
    "add_bookmark",
    "remove_bookmark",
    "reset_embeddings",
];

/// The library-write gate's decision: refuse `command` iff this instance is read-only AND
/// the command writes to the library. Enforced once in the global invoke handler.
pub(crate) fn is_write_refused(command: &str, is_library_read_only: bool) -> bool {
    is_library_read_only && WRITE_COMMANDS.contains(&command)
}

/// The library writer, or [`AppError::ReadOnly`]. A write command's body calls this to
/// get the writer through which every `.md` write flows. The IPC gate already refuses write
/// commands on a read-only instance, so reaching the `None` case here is defense in depth.
pub(crate) fn require_writer(
    state: &crate::AppState,
) -> Result<xaga_core::library_writer::LibraryWriter, crate::error::AppError> {
    state.writer().ok_or(crate::error::AppError::ReadOnly)
}

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

pub(crate) fn parse_item_sort(sort: Option<String>) -> Result<xaga_core::types::ItemSort, String> {
    sort.as_deref()
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
        assert!(!is_invoke_allowed("loader-7", "read_entry_images"));
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

    #[test]
    fn read_only_instance_refuses_every_write_command_and_nothing_else() {
        for command in WRITE_COMMANDS {
            assert!(
                is_write_refused(command, true),
                "{command} must be refused when read-only"
            );
            assert!(
                !is_write_refused(command, false),
                "{command} must pass on the writer instance"
            );
        }
        // Read commands pass regardless of mode.
        for command in ["read_article", "find_article", "list_annotations", "list_feeds"] {
            assert!(!is_write_refused(command, true));
        }
        // Account/app-scope commands are not library writes and must never be gated:
        // `set_library` must work FROM a read-only instance (it is how one becomes the
        // writer of a different library), and settings/mode reads are process-local.
        for command in ["set_library", "get_library", "is_library_read_only", "set_theme"] {
            assert!(
                !is_write_refused(command, true),
                "{command} is not a library write and must not be gated"
            );
        }
    }

    #[test]
    fn read_article_is_not_a_gated_write_command() {
        // Its incidental mark-read write no-ops in-command on a read-only instance
        // (reading stays seamless), so gating it here would break reads.
        assert!(!WRITE_COMMANDS.contains(&"read_article"));
    }
}
