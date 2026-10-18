//! Tauri command to webview error type
//!
//! Wraps `xaga_core::error::DomainError` and adds app-layer failures
//! (read-only refusal, unauthorized command, unsupported download, stale settings view).
//!
//! Serializes as `{ code, message, detail }`:
//! - `code` — stable identifier the UI switches on. Mirrored in
//!   `src/platform/errors.ts`.
//! - `message` — user-safe sentence. The only field the UI may show.
//! - `detail` — full technical text. Console/log only, never shown to a user.
//!
//! `Message` receives ad-hoc string errors through `From<String>`; user-actionable
//! failures use typed variants.

use serde::{Serialize, Serializer};
use xaga_core::embeddings::EmbeddingsError;
use xaga_core::error::DomainError;
use xaga_core::library_writer::WriterError;
use xaga_core::network_proxy::ProxyError;
use xaga_core::persistence::bookmarks::BookmarksError;
use xaga_core::persistence::entries::EntryError;
use xaga_core::persistence::feeds::FeedError;
use xaga_core::persistence::topics::TopicsError;
use xaga_core::query::items::ItemError;

#[derive(Debug, thiserror::Error)]
pub(crate) enum AppError {
    #[error(transparent)]
    Domain(#[from] DomainError),
    /// A library write was refused because this instance did not win the
    /// library lock. Absorbs the former `E_READ_ONLY` string.
    #[error("this instance is read-only — another xaga instance holds the library lock")]
    ReadOnly,
    /// A command was invoked from a window not permitted to call it.
    #[error("command not available from this window")]
    Unauthorized,
    /// Downloaded bytes failed validation; the payload is log-only detail.
    #[error("Download format is unsupported: {0}")]
    UnsupportedDownload(String),
    /// A settings write raced another xaga process; the in-memory mirrors were
    /// refreshed and the caller should retry.
    #[error("settings changed by another xaga process; view refreshed — retry")]
    SettingsStaleView,
    #[error("another xaga process holds the account lock")]
    SettingsBusy,
    #[error("import file exceeds the configured size limit")]
    ImportTooLarge,
    #[error("import size limit must be positive whole MiB and exactly representable in bytes")]
    InvalidImportLimit,
    /// Transitional catch-all for ad-hoc string errors not yet promoted to a
    /// typed variant. The string is log-only detail.
    #[error("{0}")]
    Message(String),
}

impl AppError {
    /// The stable code the UI matches on — mirrored in `src/platform/errors.ts`.
    /// Core failures delegate to their `DomainError` code.
    pub(crate) fn code(&self) -> &'static str {
        match self {
            AppError::Domain(e) => e.code(),
            AppError::ReadOnly => "library:read-only",
            AppError::Unauthorized => "command:unauthorized",
            AppError::UnsupportedDownload(_) => "article:unsupported-download",
            AppError::SettingsStaleView => "settings:stale-view",
            AppError::SettingsBusy => "settings:busy",
            AppError::ImportTooLarge => "import:too-large",
            AppError::InvalidImportLimit => "settings:invalid-import-limit",
            AppError::Message(_) => "app:error",
        }
    }

    /// A generic, user-safe message. Full detail stays in `Display`.
    pub(crate) fn user_message(&self) -> &'static str {
        match self {
            AppError::Domain(e) => e.user_message(),
            AppError::ReadOnly => {
                "This library is read-only because another xaga window has it open."
            }
            AppError::Unauthorized => "This action isn't available here.",
            AppError::UnsupportedDownload(_) => "This download’s format isn’t supported.",
            AppError::SettingsStaleView => {
                "Settings changed in another xaga window — please try again."
            }
            AppError::SettingsBusy => {
                "Another xaga instance is busy. Please try saving settings again later."
            }
            AppError::ImportTooLarge => {
                "This file exceeds the import size limit. Increase Maximum import file size in Settings → General, or import a smaller file."
            }
            AppError::InvalidImportLimit => "Enter a positive whole number of MiB.",
            AppError::Message(_) => "Something went wrong.",
        }
    }
}

/// The long tail of stringified errors funnels into `Message`, keeping existing
/// `.map_err(|e| e.to_string())?` sites valid once a signature moves to `AppError`.
impl From<String> for AppError {
    fn from(s: String) -> Self {
        AppError::Message(s)
    }
}

impl From<&str> for AppError {
    fn from(s: &str) -> Self {
        AppError::Message(s.to_string())
    }
}

/// Each subsystem error converts straight to `AppError` (via `DomainError`), so
/// `?` in a command body propagates the typed code without a `.map_err`.
macro_rules! app_from_domain {
    ($($t:ty),* $(,)?) => {
        $(impl From<$t> for AppError {
            fn from(e: $t) -> Self {
                AppError::Domain(DomainError::from(e))
            }
        })*
    };
}
app_from_domain!(
    EntryError,
    FeedError,
    EmbeddingsError,
    TopicsError,
    BookmarksError,
    ItemError,
    WriterError,
    ProxyError,
);

/// The owned, `Clone` wire shape of an error: `{ code, message, detail }`.
///
/// `AppError` itself can't be `Clone` (it wraps non-`Clone` foreign errors like
/// `reqwest::Error`), so event payloads that need `Clone` — the import/export
/// [`crate::events::Outcome`] — carry this instead. It serializes identically to
/// a directly-returned `AppError`, so the webview sees one shape on both
/// channels. `message` is the only user-facing field; `detail` is log-only.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct SerializedError {
    #[serde(rename = "diagnosticId", skip_serializing_if = "Option::is_none")]
    pub(crate) diagnostic_id: Option<String>,
    pub(crate) code: &'static str,
    pub(crate) message: &'static str,
    pub(crate) detail: String,
}

impl AppError {
    /// Import/export diagnostics have already been logged natively.
    pub(crate) fn import_export_error(&self) -> SerializedError {
        SerializedError {
            detail: String::new(),
            ..self.serialized()
        }
    }

    /// The owned wire shape, for event payloads that need `Clone`.
    pub(crate) fn serialized(&self) -> SerializedError {
        SerializedError {
            diagnostic_id: None,
            code: self.code(),
            message: self.user_message(),
            detail: self.to_string(),
        }
    }
}

/// Serialized to the webview as `{ code, message, detail }` — the same shape as
/// [`SerializedError`], so a returned error and an event-carried one are
/// indistinguishable on the JS side.
impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.serialized().serialize(serializer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// App-layer codes, in variant order. Mirrored in `src/platform/errors.ts`.
    const APP_CODES: &[&str] = &[
        "library:read-only",
        "command:unauthorized",
        "article:unsupported-download",
        "settings:stale-view",
        "settings:busy",
        "import:too-large",
        "settings:invalid-import-limit",
        "app:error",
    ];

    /// The namespaces owned by `DomainError` (core). App codes must not collide
    /// with these — the two enums share one flat code namespace on the wire.
    const DOMAIN_NAMESPACES: &[&str] = &[
        "entry",
        "feed",
        "embeddings",
        "topic",
        "bookmark",
        "item",
        "writer",
        "proxy",
    ];

    #[test]
    fn app_codes_are_wellformed_unique_and_disjoint_from_domain() {
        for code in APP_CODES {
            assert!(code.contains(':'), "code {code:?} is not namespaced");
            let ns = code.split(':').next().unwrap();
            assert!(
                !DOMAIN_NAMESPACES.contains(&ns),
                "app code {code:?} collides with a domain namespace"
            );
        }
        let unique: std::collections::HashSet<&&str> = APP_CODES.iter().collect();
        assert_eq!(unique.len(), APP_CODES.len(), "duplicate app code");
    }

    #[test]
    fn serializes_code_message_detail() {
        let e = AppError::UnsupportedDownload("magic bytes were %PDF-ish".to_string());
        let v = serde_json::to_value(&e).unwrap();
        assert_eq!(v["code"], "article:unsupported-download");
        assert_eq!(v["message"], "This download’s format isn’t supported.");
        // detail carries the technical text; message never does.
        assert!(v["detail"].as_str().unwrap().contains("magic bytes"));
        assert!(!v["message"].as_str().unwrap().contains("magic bytes"));
    }

    #[test]
    fn import_errors_do_not_serialize_private_details() {
        let error = AppError::Message("OPML fragment: private-file-content".into());
        assert!(error.serialized().detail.contains("private-file-content"));
        let wire = serde_json::to_value(error.import_export_error()).unwrap();
        assert_eq!(wire["detail"], "");
        assert!(!wire.to_string().contains("private-file-content"));
        assert_eq!(wire["message"], "Something went wrong.");
    }

    #[test]
    fn import_limit_directs_users_to_the_general_settings_control() {
        let wire = AppError::ImportTooLarge.import_export_error();
        assert_eq!(wire.code, "import:too-large");
        assert!(wire.message.contains("smaller file"));
        assert!(wire.message.contains("Settings → General"));
        assert!(wire.message.contains("Maximum import file size"));
    }

    #[test]
    fn string_funnels_into_message() {
        let e: AppError = "embedding service not running".to_string().into();
        assert_eq!(e.code(), "app:error");
        assert!(e.to_string().contains("embedding service not running"));
    }
}
