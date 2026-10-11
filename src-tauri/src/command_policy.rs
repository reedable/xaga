//! Generated classifications are shared by Tauri ACL and the native invoke gate.
include!(concat!(env!("OUT_DIR"), "/command_policy.rs"));

pub const TRUSTED_WINDOW_LABELS: &[&str] = &["main", "settings"];

pub fn is_invoke_allowed(label: &str, command: &str) -> bool {
    if label.starts_with("loader-") {
        LOADER_COMMANDS.contains(&command)
    } else if TRUSTED_WINDOW_LABELS.contains(&label) {
        TRUSTED_COMMANDS.contains(&command)
    } else {
        false
    }
}
