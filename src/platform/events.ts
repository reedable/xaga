// The event names the rust layer raises:
//   DOMAIN_EVENT ↔ DomainEvent  (crates/xaga-core/src/events.rs)
//   APP_EVENT    ↔ AppEvent     (src-tauri/src/events.rs)
// Hand-maintained against the rust enums. The rust `name()` tests pin each
// list and this file pins the same set, so a name added on one side without
// the other fails a test rather than becoming an event nobody listens for.

export const DOMAIN_EVENT = [
  "unread:changed", // payload: Entry (schema-minted)
  "bookmark:changed", // payload: Entry (schema-minted)
  "annotation:changed", // payload: ListItem (schema-minted)
  "annotation:replaced", // payload: { oldId, item: ListItem }
  "annotation:removed", // payload: string (annotation id)
  "favicon:changed", // payload: Article (schema-minted)
  "embedding:changed", // payload: none
  "embedding:shutdown", // payload: none
  "feed:refreshed", // payload: Feed (schema-minted)
  "feeds:refreshed", // payload: none
  "library:mode-changed", // payload: boolean (isLibraryReadOnly)
] as const;
export type DomainEvent = (typeof DOMAIN_EVENT)[number];

export const APP_EVENT = [
  "app:error", // payload: AppError (optional diagnosticId)
  "settings:theme", // payload: string
  "settings:refresh-on-startup", // payload: boolean
  "settings:html-reader-font-family", // payload: string
  "settings:html-reader-font-size", // payload: number
  "settings:html-reader-line-height", // payload: number
  "settings:html-reader-view", // payload: string
  "settings:inline-pdf-reader", // payload: boolean
  "settings:allowed-domains", // payload: string[]
  "menu:open-library", // payload: none
  "menu:set-theme", // payload: string
  "menu:show-sidebar", // payload: boolean
  "menu:html-reader-view", // payload: string
  "menu:clear-history", // payload: none
  "menu:find", // payload: none
  "feeds:imported", // payload: ImportOutcome<Feed>
  "feeds:exported", // payload: ExportOutcome
  "bookmarks:imported", // payload: BookmarkImportOutcome
  "bookmarks:exported", // payload: ExportOutcome
  "loader:will-unload", // payload: string (url)
  "loader:page-loaded", // payload: { url, html, mimeType }
  "loader:closed", // payload: { loaderUrl, pageUrl, samePage, html, mimeType }
] as const;
export type AppEvent = (typeof APP_EVENT)[number];
