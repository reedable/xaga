import type { ImageRecovery } from "../text/imageReferences";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { createContext, useContext } from "react";
import type {
  ArticleDiagnostic,
  ArticleDiagnosticOutcome,
  ArticleDiagnosticStep,
  Annotation,
  PartialAnnotation,
  Article,
  Ranked,
  RankedItem,
  Entry,
  Feed,
  Filter,
  HtmlReaderView,
  ImportOutcome,
  BookmarkImportOutcome,
  ExportOutcome,
  Item,
  ListItem,
  AllItemsCounts,
  Topic,
  TopicSort,
} from "../types";

export interface Platform {
  onError(callback: (error: import("./errors").AppError) => void): Promise<UnlistenFn>;
  // Window
  setWindowTitle(title: string): Promise<void>;
  /** Wait for pending edits before closing; false keeps the window open. */
  onWindowCloseRequested(callback: () => Promise<boolean>): Promise<UnlistenFn>;

  openFileDialog(
    directory: boolean,
    multiple?: boolean
  ): Promise<string | null>;

  openUrl(url: string): Promise<void>;

  // Library
  getLibrary(): Promise<string>;
  setLibrary(path: string): Promise<string>;
  /** Whether this instance is read-only for the open library (another xaga
   *  instance holds the library lock). Same name as the Rust accessor, the
   *  Tauri command, and the `library:mode-changed` event payload. */
  isLibraryReadOnly(): Promise<boolean>;

  // Feeds
  listFeeds(signal: AbortSignal): Promise<Feed[]>;
  addFeed(signal: AbortSignal, feedUrl: string): Promise<Feed>;
  refreshAllFeeds(signal: AbortSignal): Promise<Feed[]>;
  renameFeed(signal: AbortSignal, feedId: string, title: string): Promise<Feed>;
  removeFeed(signal: AbortSignal, feedUrl: string): Promise<string>;

  // Startup
  isCacheReady(): Promise<boolean>;

  // Entries
  listEntries(
    signal: AbortSignal,
    feedUrl: string,
    filter: Filter
  ): Promise<Entry[]>;
  listAllItems(signal: AbortSignal, filter: Filter): Promise<ListItem[]>;
  getAllItemsCounts(): Promise<AllItemsCounts | null>;
  markEntryRead(signal: AbortSignal, id: string): Promise<Entry>;
  markEntryUnread(signal: AbortSignal, id: string): Promise<Entry>;

  // Articles
  createArticleDiagnostic(mode: string): Promise<string>;
  addArticleDiagnosticEvent(
    id: string,
    step: ArticleDiagnosticStep,
    outcome: ArticleDiagnosticOutcome,
    code: string
  ): Promise<void>;
  getArticleDiagnostic(id: string): Promise<ArticleDiagnostic | null>;
  readEntry(signal: AbortSignal, id: string): Promise<Entry>;
  openStoredPdf(url: string): Promise<void>;
  readStoredPdf(signal: AbortSignal, url: string): Promise<{ pdfBase64?: string | null } | null>;
  loadArticle(
    signal: AbortSignal,
    url: string,
    keepOpen?: boolean,
    diagnosticId?: string
  ): Promise<{
    mimeType: string;
    html: string;
    pdfBase64?: string | null;
    // The favicon the loader resolved and fetched for this page. Fetched there
    // so the main window never issues the request itself.
    favicon?: string | null;
    faviconData?: string | null;
  }>;
  findArticle(
    signal: AbortSignal,
    url: string,
    diagnosticId?: string
  ): Promise<Article | null>;
  // `imageData` maps each fetched image's remote URL to its `data:` bytes (from
  // processEntry); the backend writes files and uses local body URLs only for
  // successful writes. Omit/empty when saving without freshly-fetched images.
  saveArticle(
    signal: AbortSignal,
    article: Article,
    imageData?: Record<string, string>,
    diagnosticId?: string
  ): Promise<Article>;
  readArticle(
    signal: AbortSignal,
    id: string,
    diagnosticId?: string
  ): Promise<Article>;
  // Resolves an entry's stored `../images/…` references to `data:` URIs for
  // rendering, index-aligned with `refs`. A `null` at index i means that
  // reference did not resolve (missing/oversize/foreign) and its `src` should be
  // left as-is.
  recoverEntryImages(signal: AbortSignal, id: string, recovery: ImageRecovery, diagnosticId?: string): Promise<Article>;
  readEntryImages(
    signal: AbortSignal,
    id: string,
    refs: string[]
  ): Promise<(string | null)[]>;
  // `__kind` is minted at the boundary (toArticle), not caller-supplied,
  // so it's omitted from the input. Tactical fix — see TECH_DEBT.md
  // "Frontend type branding (`__kind`)".
  createArticle(
    signal: AbortSignal,
    article: Omit<Article, "id" | "createdAt" | "__kind" | "isBookmarked">,
    imageData?: Record<string, string>,
    diagnosticId?: string
  ): Promise<Article>;

  // Annotations
  addAnnotation(
    signal: AbortSignal,
    annotation: Omit<Annotation, "createdAt" | "updatedAt" | "__kind">
  ): Promise<Article>;
  /** Replaces an annotation under a new id in one atomic write — the color-change path
   * (a color change is delete/create, not an update). `annotation.id` is the new id. */
  replaceAnnotation(
    signal: AbortSignal,
    oldId: string,
    annotation: Omit<Annotation, "createdAt" | "updatedAt" | "__kind">
  ): Promise<Article>;
  removeAnnotation(
    signal: AbortSignal,
    annotation: Pick<Annotation, "id" | "reference">
  ): Promise<Article>;
  listAnnotations(signal: AbortSignal): Promise<PartialAnnotation[]>;

  // Topics
  createTopic(
    signal: AbortSignal,
    topic: Omit<
      Topic,
      "id" | "createdAt" | "unreadCount" | "entryCount" | "isPinned" | "__kind" | "members"
    >
  ): Promise<Topic>;
  saveTopic(signal: AbortSignal, topic: Topic): Promise<Topic>;
  deleteTopic(signal: AbortSignal, id: string): Promise<void>;
  listTopics(signal: AbortSignal): Promise<Topic[]>;
  readTopic(signal: AbortSignal, id: string): Promise<Topic>;
  listTopicItems(
    signal: AbortSignal,
    id: string,
    filter: Filter,
    sort: TopicSort
  ): Promise<ListItem[]>;
  addTopicItem(
    signal: AbortSignal,
    topicId: string,
    item: Entry | Annotation
  ): Promise<Topic>;
  removeTopicItem(
    signal: AbortSignal,
    topicId: string,
    itemId: string
  ): Promise<Topic>;
  listEntryTopics(signal: AbortSignal, entryId: string): Promise<string[]>;
  rankTopicsRelatedToItem(
    signal: AbortSignal,
    itemId: string,
    minSimilarity?: number
  ): Promise<RankedItem[]>;

  // Bookmarks
  addBookmark(signal: AbortSignal, id: string): Promise<Entry>;
  removeBookmark(signal: AbortSignal, id: string): Promise<Entry>;

  // Embedding queries
  queryByItem(
    signal: AbortSignal,
    id: string,
    filter?: Filter,
    minSimilarity?: number | null,
    maxSimilarity?: number | null,
    limit?: number | null
  ): Promise<Ranked<Item>[]>;

  queryByText(
    signal: AbortSignal,
    text: string,
    filter?: Filter,
    minSimilarity?: number | null,
    maxSimilarity?: number | null,
    limit?: number | null
  ): Promise<Ranked<Item>[]>;

  // Theme
  getTheme(): Promise<string>;
  setTheme(theme: string): Promise<void>;

  // Settings
  getImportMaxBytes(): Promise<number>;
  setImportMaxBytes(value: number): Promise<void>;
  getRefreshOnStartup(): Promise<boolean>;
  setRefreshOnStartup(value: boolean): Promise<void>;
  getHtmlReaderPrefs(): Promise<{
    fontFamily: string;
    fontSize: number;
    lineHeight: number;
  }>;
  setHtmlReaderFontFamily(value: string): Promise<void>;
  setHtmlReaderFontSize(value: number): Promise<void>;
  setHtmlReaderLineHeight(value: number): Promise<void>;
  getShowSidebar(): Promise<boolean>;
  setShowSidebar(value: boolean): Promise<void>;

  // Events
  onOpenLibrary(callback: () => void): Promise<UnlistenFn>;
  onOpenSettings(callback: () => void): Promise<UnlistenFn>;
  onFeedsImported(
    callback: (outcome: ImportOutcome<Feed>) => void
  ): Promise<UnlistenFn>;
  onFeedsExported(
    callback: (outcome: ExportOutcome) => void
  ): Promise<UnlistenFn>;
  onBookmarksImported(callback: (outcome: BookmarkImportOutcome) => void): Promise<UnlistenFn>;
  onBookmarksExported(callback: (outcome: ExportOutcome) => void): Promise<UnlistenFn>;
  onFeedRefreshed(callback: (feed: Feed) => void): Promise<UnlistenFn>;
  /** Fires once after `refresh_all_feeds` completes (not per feed). */
  onFeedsRefreshed(callback: () => void): Promise<UnlistenFn>;
  onLibraryModeChanged(
    callback: (isLibraryReadOnly: boolean) => void
  ): Promise<UnlistenFn>;
  onUnreadChanged(callback: (entry: Entry) => void): Promise<UnlistenFn>;
  onBookmarkChanged(callback: (entry: Entry) => void): Promise<UnlistenFn>;
  onAnnotationChanged(callback: (item: ListItem) => void): Promise<UnlistenFn>;
  onAnnotationRemoved(callback: (id: string) => void): Promise<UnlistenFn>;
  onFaviconChanged(callback: (article: Article) => void): Promise<UnlistenFn>;
  onEmbeddingsChanged(callback: () => void): Promise<UnlistenFn>;
  onSetTheme(callback: (theme: string) => void): Promise<UnlistenFn>;
  onShowSidebar(callback: (show: boolean) => void): Promise<UnlistenFn>;
  onLoaderClosed(
    callback: (data: {
      loaderUrl: string;
      pageUrl: string;
      samePage: boolean;
      html: string;
      mimeType: string;
      favicon?: string | null;
      faviconData?: string | null;
    }) => void
  ): Promise<UnlistenFn>;
  onLoaderPageLoaded(
    callback: (data: {
      url: string;
      html: string;
      mimeType: string;
      favicon?: string | null;
      faviconData?: string | null;
    }) => void
  ): Promise<UnlistenFn>;
  onFind(callback: () => void): Promise<UnlistenFn>;
  setFindEnabled(enabled: boolean): Promise<void>;
  onShutdown(callback: () => void): Promise<UnlistenFn>;
  onClearHistory(callback: () => void): Promise<UnlistenFn>;
  clearWebviewHistory(): Promise<void>;

  // Privacy
  getAllowedDomains(): Promise<string[]>;
  setAllowedDomains(domains: string[]): Promise<void>;
  onAllowedDomainsChanged(
    callback: (domains: string[]) => void
  ): Promise<UnlistenFn>;

  // HTML reader view preference
  getHtmlReaderView(): Promise<string>;
  setHtmlReaderView(view: HtmlReaderView): Promise<void>;
  onHtmlReaderView(
    callback: (view: HtmlReaderView) => void
  ): Promise<UnlistenFn>;
  onHtmlReaderViewChangedFromSettings(
    callback: (view: HtmlReaderView) => void
  ): Promise<UnlistenFn>;

  // Pinned state lives in the content files (feed `.atom` category / topic `.md` tag);
  // surfaced on the Feed/Topic types (`isPinned`) and toggled per-item here.
  setFeedPinned(id: string, pinned: boolean): Promise<void>;
  setTopicPinned(id: string, pinned: boolean): Promise<void>;

  // Persisted preferences
  getLoadRemoteImage(): Promise<boolean>;
  setLoadRemoteImage(value: boolean): Promise<void>;
  getInlinePdfReader(): Promise<boolean>;
  setInlinePdfReader(value: boolean): Promise<void>;
  onInlinePdfReaderChanged(
    callback: (value: boolean) => void
  ): Promise<UnlistenFn>;

  // Settings sync (main window receives these when settings window saves)
  onThemeChangedFromSettings(
    callback: (theme: string) => void
  ): Promise<UnlistenFn>;
  onHtmlReaderFontFamilyChanged(
    callback: (value: string) => void
  ): Promise<UnlistenFn>;
  onHtmlReaderFontSizeChanged(
    callback: (value: number) => void
  ): Promise<UnlistenFn>;
  onHtmlReaderLineHeightChanged(
    callback: (value: number) => void
  ): Promise<UnlistenFn>;
  onRefreshOnStartupChanged(
    callback: (value: boolean) => void
  ): Promise<UnlistenFn>;
}

export const PlatformContext = createContext<Platform | null>(null);

export function usePlatform(): Platform {
  const context = useContext(PlatformContext);

  if (!context) {
    throw new Error("usePlatform must be used inside PlatformContext.Provider");
  }

  return context;
}
