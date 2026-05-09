import type { UnlistenFn } from "@tauri-apps/api/event";
import { createContext, useContext } from "react";
import type {
  Annotation,
  AnnotationSummary,
  Article,
  Embedded,
  Entry,
  Feed,
  Filter,
  ImportResult,
  Item,
  SavedItem,
  Topic,
} from "../types";

export interface Platform {
  // Window
  setWindowTitle(title: string): Promise<void>;

  openFileDialog(
    directory: boolean,
    multiple?: boolean,
  ): Promise<string | null>;

  openUrl(url: string): Promise<void>;

  // Library
  getLibrary(): Promise<string>;
  setLibrary(path: string): Promise<string>;

  // Feeds
  listFeeds(signal: AbortSignal): Promise<Feed[]>;
  addFeed(signal: AbortSignal, feedUrl: string): Promise<Feed>;
  importFeeds(signal: AbortSignal): Promise<ImportResult<Feed> | null>;
  exportFeeds(signal: AbortSignal): Promise<void>;
  refreshFeed(signal: AbortSignal, feedId: string): Promise<Feed>;
  renameFeed(signal: AbortSignal, feedId: string, title: string): Promise<Feed>;
  // refreshAllFeeds(signal: AbortSignal): Promise<PromiseSettledResult<Feed>[]>;
  removeFeed(signal: AbortSignal, feedUrl: string): Promise<string>;

  // Startup
  isCacheSeeded(): Promise<boolean>;

  // Entries
  listEntries(
    signal: AbortSignal,
    feedUrl: string,
    filter: Filter,
  ): Promise<Entry[]>;
  listAllEntries(signal: AbortSignal, filter: Filter): Promise<Entry[]>;
  markEntryRead(signal: AbortSignal, id: string): Promise<Entry>;
  markEntryUnread(signal: AbortSignal, id: string): Promise<Entry>;

  // Articles
  loadArticle(signal: AbortSignal, url: string, keepOpen?: boolean): Promise<string>;
  findArticle(signal: AbortSignal, url: string): Promise<Article | null>;
  saveArticle(signal: AbortSignal, article: Article): Promise<Article>;
  readArticle(signal: AbortSignal, id: string): Promise<Article>;
  createArticle(
    signal: AbortSignal,
    article: Omit<Article, "id" | "createdAt">,
  ): Promise<Article>;

  // Annotations
  addAnnotation(
    signal: AbortSignal,
    annotation: Omit<Annotation, "createdAt" | "updatedAt">,
  ): Promise<Article>;
  removeAnnotation(
    signal: AbortSignal,
    annotation: Pick<Annotation, "id" | "reference">,
  ): Promise<Article>;
  listAnnotations(signal: AbortSignal): Promise<AnnotationSummary[]>;
  listSaved(signal: AbortSignal): Promise<SavedItem[]>;

  // Topics
  createTopic(
    signal: AbortSignal,
    topic: Omit<Topic, "id" | "createdAt" | "unreadCount" | "entryCount">,
  ): Promise<Topic>;
  saveTopic(signal: AbortSignal, topic: Topic): Promise<Topic>;
  deleteTopic(signal: AbortSignal, id: string): Promise<void>;
  listTopics(signal: AbortSignal): Promise<Topic[]>;
  readTopic(signal: AbortSignal, id: string): Promise<Topic>;

  // Bookmarks
  addBookmark(signal: AbortSignal, id: string): Promise<Entry>;
  removeBookmark(signal: AbortSignal, id: string): Promise<Entry>;
  listBookmarks(signal: AbortSignal): Promise<Entry[]>;
  importBookmarks(signal: AbortSignal): Promise<ImportResult<Entry> | null>;
  exportBookmarks(signal: AbortSignal): Promise<void>;

  // Embedding queries
  queryByItem(
    signal: AbortSignal,
    id: string,
    filter?: Filter,
    minSimilarity?: number | null,
    maxSimilarity?: number | null,
    limit?: number | null,
  ): Promise<(Embedded & Item)[]>;

  queryByBookmarks(
    signal: AbortSignal,
    filter?: Filter,
    minSimilarity?: number | null,
    maxSimilarity?: number | null,
    limit?: number | null,
  ): Promise<(Embedded & Item)[]>;

  queryByTopics(
    signal: AbortSignal,
    filter?: Filter,
    minSimilarity?: number | null,
    maxSimilarity?: number | null,
    limit?: number | null,
  ): Promise<(Embedded & Item)[]>;

  queryByAnnotations(
    signal: AbortSignal,
    filter?: Filter,
    minSimilarity?: number | null,
    maxSimilarity?: number | null,
    limit?: number | null,
  ): Promise<(Embedded & Item)[]>;

  queryBySaved(
    signal: AbortSignal,
    filter?: Filter,
    minSimilarity?: number | null,
    maxSimilarity?: number | null,
    limit?: number | null,
  ): Promise<(Embedded & Item)[]>;

  queryByText(
    signal: AbortSignal,
    text: string,
    filter?: Filter,
    minSimilarity?: number | null,
    maxSimilarity?: number | null,
    limit?: number | null,
  ): Promise<(Embedded & Item)[]>;

  // Theme
  getTheme(): Promise<string>;
  setTheme(theme: string): Promise<void>;

  // Settings
  getRefreshOnStartup(): Promise<boolean>;
  setRefreshOnStartup(value: boolean): Promise<void>;
  getReaderPrefs(): Promise<{
    fontFamily: string;
    fontSize: number;
    lineHeight: number;
  }>;
  setReaderFontFamily(value: string): Promise<void>;
  setReaderFontSize(value: number): Promise<void>;
  setReaderLineHeight(value: number): Promise<void>;

  // Events
  onOpenLibrary(callback: () => void): Promise<UnlistenFn>;
  onOpenSettings(callback: () => void): Promise<UnlistenFn>;
  onImportFeeds(callback: () => void): Promise<UnlistenFn>;
  onExportFeeds(callback: () => void): Promise<UnlistenFn>;
  onFeedRefreshed(callback: (feed: Feed) => void): Promise<UnlistenFn>;
  onUnreadChanged(callback: (entry: Entry) => void): Promise<UnlistenFn>;
  onBookmarkChanged(callback: (entry: Entry) => void): Promise<UnlistenFn>;
  onEmbeddingsChanged(callback: () => void): Promise<UnlistenFn>;
  onImportBookmarks(callback: () => void): Promise<UnlistenFn>;
  onExportBookmarks(callback: () => void): Promise<UnlistenFn>;
  onSetTheme(callback: (theme: string) => void): Promise<UnlistenFn>;
  onShowArticleSummary(callback: (show: boolean) => void): Promise<UnlistenFn>;
  onFind(callback: () => void): Promise<UnlistenFn>;
  setFindEnabled(enabled: boolean): Promise<void>;
  onShutdown(callback: () => void): Promise<UnlistenFn>;

  // Settings sync (main window receives these when settings window saves)
  onThemeChangedFromSettings(
    callback: (theme: string) => void,
  ): Promise<UnlistenFn>;
  onReaderFontFamilyChanged(
    callback: (value: string) => void,
  ): Promise<UnlistenFn>;
  onReaderFontSizeChanged(
    callback: (value: number) => void,
  ): Promise<UnlistenFn>;
  onReaderLineHeightChanged(
    callback: (value: number) => void,
  ): Promise<UnlistenFn>;
  onRefreshOnStartupChanged(
    callback: (value: boolean) => void,
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
