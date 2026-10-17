//! App-layer events and the single emit funnel.
//!
//! Every event the rust layer raises to the UI is defined as an enum variant
//! that pairs the payload with the event name: domain events in
//! `xaga_core::events::DomainEvent`, app-layer events (settings, menu,
//! loader, import/export) in [`AppEvent`] here — split per
//! docs/architecture/layer_ownership.md.
//!
//! INVARIANT: `.emit(` appears only in this module. Raising an event
//! anywhere else goes through [`emit_domain`] / [`emit_app`], so an event
//! cannot exist without a typed definition and a registered name. The UI
//! side of the names is `src/platform/events.ts`, hand-maintained against
//! this enum — the `name()` test here and the TS list pin each other, so a
//! name added on one side alone fails a test.

use serde::Serialize;
use tauri::{Emitter, Runtime};
use xaga_core::events::DomainEvent;

use crate::native::bookmarks::BookmarkImportResult;
use crate::native::feeds::ImportResult;

/// Event payload for a native-triggered import/export. On failure `error`
/// carries the typed [`SerializedError`] (`{ code, message, detail }`) — the
/// same shape a rejected command returns, so the UI treats both channels alike.
#[derive(Serialize, Clone)]
pub(crate) struct Outcome<T: Serialize + Clone> {
    pub(crate) result: Option<T>,
    pub(crate) error: Option<crate::error::SerializedError>,
}

/// Payload of `loader:page-loaded`.
#[derive(Serialize, Clone)]
pub(crate) struct LoaderPageLoaded {
    pub(crate) url: String,
    pub(crate) html: String,
    #[serde(rename = "mimeType")]
    pub(crate) mime_type: String,
    /// Favicon the loader resolved and fetched for this page. Carried here
    /// because the loader is the only context allowed to fetch it — see
    /// `docs/architecture/security/fetching_remote_content.md`.
    pub(crate) favicon: Option<String>,
    #[serde(rename = "faviconData")]
    pub(crate) favicon_data: Option<String>,
}

/// Payload of `loader:closed`.
#[derive(Serialize, Clone)]
pub(crate) struct LoaderClosed {
    /// The URL the loader window was opened with (the main window's request).
    #[serde(rename = "loaderUrl")]
    pub(crate) loader_url: String,
    /// The URL the page was actually on when the window closed (redirects,
    /// SPA canonicalization, or user navigation may have moved it).
    #[serde(rename = "pageUrl")]
    pub(crate) page_url: String,
    /// Whether `pageUrl` still denotes the same page as `loaderUrl` under
    /// `normalize_url` — the captured `html` belongs to `pageUrl`, so a
    /// mismatch means it must not be treated as content for `loaderUrl`.
    #[serde(rename = "samePage")]
    pub(crate) same_page: bool,
    pub(crate) html: String,
    #[serde(rename = "mimeType")]
    pub(crate) mime_type: String,
    /// See `LoaderPageLoaded::favicon`.
    pub(crate) favicon: Option<String>,
    #[serde(rename = "faviconData")]
    pub(crate) favicon_data: Option<String>,
}

pub(crate) enum AppEvent {
    Error(crate::error::SerializedError),
    SettingsTheme(String),
    SettingsRefreshOnStartup(bool),
    SettingsHtmlReaderFontFamily(String),
    SettingsHtmlReaderFontSize(f64),
    SettingsHtmlReaderLineHeight(f64),
    SettingsHtmlReaderView(String),
    SettingsInlinePdfReader(bool),
    SettingsAllowedDomains(Vec<String>),
    MenuOpenLibrary,
    MenuSetTheme(String),
    MenuShowSidebar(bool),
    MenuHtmlReaderView(String),
    MenuClearHistory,
    MenuFind,
    FeedsImported(Outcome<ImportResult>),
    FeedsExported(Outcome<()>),
    BookmarksImported(Outcome<BookmarkImportResult>),
    BookmarksExported(Outcome<()>),
    LoaderWillUnload(String),
    LoaderPageLoaded(LoaderPageLoaded),
    LoaderClosed(LoaderClosed),
}

impl AppEvent {
    /// The event name as the UI receives it — mirrored in
    /// `src/platform/events.ts` (`APP_EVENT`).
    pub(crate) fn name(&self) -> &'static str {
        match self {
            AppEvent::Error(_) => "app:error",
            AppEvent::SettingsTheme(_) => "settings:theme",
            AppEvent::SettingsRefreshOnStartup(_) => "settings:refresh-on-startup",
            AppEvent::SettingsHtmlReaderFontFamily(_) => "settings:html-reader-font-family",
            AppEvent::SettingsHtmlReaderFontSize(_) => "settings:html-reader-font-size",
            AppEvent::SettingsHtmlReaderLineHeight(_) => "settings:html-reader-line-height",
            AppEvent::SettingsHtmlReaderView(_) => "settings:html-reader-view",
            AppEvent::SettingsInlinePdfReader(_) => "settings:inline-pdf-reader",
            AppEvent::SettingsAllowedDomains(_) => "settings:allowed-domains",
            AppEvent::MenuOpenLibrary => "menu:open-library",
            AppEvent::MenuSetTheme(_) => "menu:set-theme",
            AppEvent::MenuShowSidebar(_) => "menu:show-sidebar",
            AppEvent::MenuHtmlReaderView(_) => "menu:html-reader-view",
            AppEvent::MenuClearHistory => "menu:clear-history",
            AppEvent::MenuFind => "menu:find",
            AppEvent::FeedsImported(_) => "feeds:imported",
            AppEvent::FeedsExported(_) => "feeds:exported",
            AppEvent::BookmarksImported(_) => "bookmarks:imported",
            AppEvent::BookmarksExported(_) => "bookmarks:exported",
            AppEvent::LoaderWillUnload(_) => "loader:will-unload",
            AppEvent::LoaderPageLoaded(_) => "loader:page-loaded",
            AppEvent::LoaderClosed(_) => "loader:closed",
        }
    }
}

/// Emits a domain event to `target` (an `AppHandle`, `Window`, or
/// `WebviewWindow`), preserving the caller-selected delivery scope.
pub(crate) fn emit_domain<R: Runtime>(target: &impl Emitter<R>, event: DomainEvent) {
    let name = event.name();
    let result = match &event {
        DomainEvent::UnreadChanged(entry) => target.emit(name, entry),
        DomainEvent::BookmarkChanged(entry) => target.emit(name, entry),
        DomainEvent::AnnotationChanged(item) => target.emit(name, item),
        DomainEvent::AnnotationRemoved { id } => target.emit(name, id),
        DomainEvent::FeedRefreshed(feed) => target.emit(name, feed),
        DomainEvent::LibraryModeChanged {
            is_library_read_only,
        } => target.emit(name, is_library_read_only),
        DomainEvent::EmbeddingChanged
        | DomainEvent::EmbeddingShutdown
        | DomainEvent::FeedsRefreshed => target.emit(name, ()),
    };
    if let Err(e) = result {
        log::warn!("emit {name} failed: {e}");
    }
}

/// Emits an app-layer event to `target`. Same delivery-scope contract as
/// [`emit_domain`].
pub(crate) fn emit_app<R: Runtime>(target: &impl Emitter<R>, event: AppEvent) {
    let name = event.name();
    let result = match &event {
        AppEvent::Error(v) => target.emit(name, v),
        AppEvent::SettingsTheme(v)
        | AppEvent::SettingsHtmlReaderFontFamily(v)
        | AppEvent::SettingsHtmlReaderView(v)
        | AppEvent::MenuSetTheme(v)
        | AppEvent::MenuHtmlReaderView(v)
        | AppEvent::LoaderWillUnload(v) => target.emit(name, v),
        AppEvent::SettingsRefreshOnStartup(v)
        | AppEvent::SettingsInlinePdfReader(v)
        | AppEvent::MenuShowSidebar(v) => target.emit(name, v),
        AppEvent::SettingsHtmlReaderFontSize(v) | AppEvent::SettingsHtmlReaderLineHeight(v) => {
            target.emit(name, v)
        }
        AppEvent::SettingsAllowedDomains(v) => target.emit(name, v),
        AppEvent::MenuOpenLibrary | AppEvent::MenuClearHistory | AppEvent::MenuFind => {
            target.emit(name, ())
        }
        AppEvent::FeedsImported(v) => target.emit(name, v),
        AppEvent::FeedsExported(v) | AppEvent::BookmarksExported(v) => target.emit(name, v),
        AppEvent::BookmarksImported(v) => target.emit(name, v),
        AppEvent::LoaderPageLoaded(v) => target.emit(name, v),
        AppEvent::LoaderClosed(v) => target.emit(name, v),
    };
    if let Err(e) = result {
        log::warn!("emit {name} failed: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outcome<T: Serialize + Clone>() -> Outcome<T> {
        Outcome { result: None, error: None }
    }

    fn all_variants() -> Vec<AppEvent> {
        vec![
            AppEvent::Error(crate::error::AppError::Message("test".into()).serialized()),
            AppEvent::SettingsTheme(String::new()),
            AppEvent::SettingsRefreshOnStartup(false),
            AppEvent::SettingsHtmlReaderFontFamily(String::new()),
            AppEvent::SettingsHtmlReaderFontSize(0.0),
            AppEvent::SettingsHtmlReaderLineHeight(0.0),
            AppEvent::SettingsHtmlReaderView(String::new()),
            AppEvent::SettingsInlinePdfReader(false),
            AppEvent::SettingsAllowedDomains(Vec::new()),
            AppEvent::MenuOpenLibrary,
            AppEvent::MenuSetTheme(String::new()),
            AppEvent::MenuShowSidebar(false),
            AppEvent::MenuHtmlReaderView(String::new()),
            AppEvent::MenuClearHistory,
            AppEvent::MenuFind,
            AppEvent::FeedsImported(outcome()),
            AppEvent::FeedsExported(outcome()),
            AppEvent::BookmarksImported(outcome()),
            AppEvent::BookmarksExported(outcome()),
            AppEvent::LoaderWillUnload(String::new()),
            AppEvent::LoaderPageLoaded(LoaderPageLoaded {
                url: String::new(),
                html: String::new(),
                mime_type: String::new(),
                favicon: None,
                favicon_data: None,
            }),
            AppEvent::LoaderClosed(LoaderClosed {
                loader_url: String::new(),
                page_url: String::new(),
                same_page: false,
                html: String::new(),
                mime_type: String::new(),
                favicon: None,
                favicon_data: None,
            }),
        ]
    }

    /// The domain names pinned by xaga-core's `events::tests`; listed here to
    /// assert cross-enum uniqueness without reconstructing domain payloads.
    const DOMAIN_NAMES: [&str; 8] = [
        "unread:changed",
        "bookmark:changed",
        "annotation:changed",
        "annotation:removed",
        "embedding:changed",
        "embedding:shutdown",
        "feed:refreshed",
        "feeds:refreshed",
    ];

    /// App names match the expected list exactly (a rename is a deliberate
    /// edit here AND in src/platform/events.ts) and the full name space —
    /// both enums — is collision-free.
    #[test]
    fn names_are_the_expected_set_and_unique_across_both_enums() {
        let names: Vec<&str> = all_variants().iter().map(|e| e.name()).collect();
        assert_eq!(
            names,
            [
                "app:error",
                "settings:theme",
                "settings:refresh-on-startup",
                "settings:html-reader-font-family",
                "settings:html-reader-font-size",
                "settings:html-reader-line-height",
                "settings:html-reader-view",
                "settings:inline-pdf-reader",
                "settings:allowed-domains",
                "menu:open-library",
                "menu:set-theme",
                "menu:show-sidebar",
                "menu:html-reader-view",
                "menu:clear-history",
                "menu:find",
                "feeds:imported",
                "feeds:exported",
                "bookmarks:imported",
                "bookmarks:exported",
                "loader:will-unload",
                "loader:page-loaded",
                "loader:closed",
            ]
        );
        let all: Vec<&str> = names.iter().copied().chain(DOMAIN_NAMES).collect();
        let unique: std::collections::HashSet<&str> = all.iter().copied().collect();
        assert_eq!(unique.len(), all.len());
    }
}

/// Deliver an unsolicited error only to the webview that owns the operation.
pub(crate) fn emit_error<R: Runtime>(app: &tauri::AppHandle<R>, label: &str, error: crate::error::SerializedError) {
    let event = AppEvent::Error(error);
    if let AppEvent::Error(payload) = &event {
        if let Err(error) = app.emit_to(tauri::EventTarget::webview_window(label), event.name(), payload) {
            log::warn!("error notification failed: {error}");
        }
    }
}
