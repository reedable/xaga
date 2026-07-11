// The event names the rust layer raises:
//   DOMAIN_EVENT ↔ DomainEvent  (crates/xaga-core/src/events.rs)
//   APP_EVENT    ↔ AppEvent     (src-tauri/src/events.rs)
// Hand-maintained against the rust enums (the rust name tests pin each list;
// codegen to replace this mirror: docs-transient/backlog/build/spike__ts_type_codegen.md).

export const DOMAIN_EVENT = [
  "unread:changed", // payload: Entry (schema-minted)
  "bookmark:changed", // payload: Entry (schema-minted)
  "annotation:changed", // payload: ListItem (schema-minted)
  "annotation:removed", // payload: string (annotation id)
  "embedding:changed", // payload: none
  "embedding:shutdown", // payload: none
  "feed:refreshed", // payload: Feed (schema-minted)
  "feeds:refreshed", // payload: none
  "library:mode-changed", // payload: boolean (isLibraryReadOnly)
] as const;
export type DomainEvent = (typeof DOMAIN_EVENT)[number];

export const APP_EVENT = [
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
  "bookmarks:imported", // payload: ImportOutcome (no frontend consumer yet)
  "bookmarks:exported", // payload: ExportOutcome (no frontend consumer yet)
  "loader:will-unload", // payload: string (url)
  "loader:page-loaded", // payload: { url, html, mimeType }
  "loader:closed", // payload: { loaderUrl, pageUrl, samePage, html, mimeType }
] as const;
export type AppEvent = (typeof APP_EVENT)[number];
