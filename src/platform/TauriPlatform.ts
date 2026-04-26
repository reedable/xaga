import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open, save } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { z } from "zod";
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
import { abortable, decorate, logged } from "../utils/Decorator";
import Logger from "../utils/Logger";
import type { Platform } from "./Platform";

const logger = Logger.getLogger("TauriPlatform");

const ItemSchema = z.object({
  id: z.string(),
  title: z.string(),
  created_at: z.string(),
  updated_at: z.string().nullable(),
});

const toItem = (data: z.infer<typeof ItemSchema>): Item => ({
  id: data.id,
  title: data.title,
  createdAt: data.created_at,
  updatedAt: data.updated_at ?? undefined,
});

const FeedSchema = ItemSchema.extend({
  url: z.string(),
  entry_count: z.number(),
  unread_count: z.number(),
});

const toFeed = (data: z.infer<typeof FeedSchema>): Feed => {
  return {
    ...toItem(data),
    url: data.url,
    entryCount: data.entry_count,
    unreadCount: data.unread_count,
  };
};

const EntrySchema = ItemSchema.extend({
  url: z.string(),
  published_at: z.string().nullable(),
  summary: z.string().nullable(),
  is_unread: z.boolean(),
});

const toEntry = (data: z.infer<typeof EntrySchema>): Entry => {
  return {
    ...toItem(data),
    url: data.url,
    publishedAt: data.published_at ?? undefined,
    summary: data.summary ?? undefined,
    isUnread: data.is_unread,
  };
};

const TopicSchema = ItemSchema.extend({
  content: z.string(),
  threshold: z.number().nullable().optional(),
  entry_count: z.number(),
  unread_count: z.number(),
});

const toTopic = (data: z.infer<typeof TopicSchema>): Topic => ({
  ...toItem(data),
  content: data.content,
  threshold: data.threshold ?? undefined,
  entryCount: data.entry_count,
  unreadCount: data.unread_count,
});

const AnnotationSchema = z.object({
  id: z.string(),
  reference: z.string(),
  text: z.array(z.string()),
  occurrence: z.array(z.number()),
  color: z.string(),
  note: z.string().nullable().optional(),
  created_at: z.string(),
});

const toAnnotation = (data: z.infer<typeof AnnotationSchema>): Annotation => ({
  id: data.id,
  title: data.text.join(" "),
  reference: data.reference,
  text: data.text,
  occurrence: data.occurrence,
  color: data.color,
  note: data.note ?? undefined,
  createdAt: data.created_at,
});

const SavedItemSchema = z.object({
  id: z.string(),
  title: z.string(),
  created_at: z.string(),
  url: z.string(),
  reference: z.string().optional(),
  color: z.string().optional(),
});

const toSavedItem = (data: z.infer<typeof SavedItemSchema>): SavedItem => ({
  id: data.id,
  title: data.title,
  createdAt: data.created_at,
  url: data.url,
  reference: data.reference,
  color: data.color,
});

const AnnotationSummarySchema = z.object({
  id: z.string(),
  reference: z.string(),
  title: z.string(),
  color: z.string(),
  created_at: z.string(),
  url: z.string(),
});

const toAnnotationSummary = (
  data: z.infer<typeof AnnotationSummarySchema>,
): AnnotationSummary => ({
  id: data.id,
  reference: data.reference,
  title: data.title,
  color: data.color,
  createdAt: data.created_at,
  url: data.url,
});

// Schema for annotation rows returned from embedding queries.
// These carry only the fields stored in LanceDB — no text[] or occurrence[].
const EmbeddedAnnotationResultSchema = z.object({
  id: z.string(),
  title: z.string(),
  created_at: z.string(),
  updated_at: z.string().nullable().optional(),
  color: z.string(),
  reference: z.string(),
  similarity: z.number(),
});

const toEmbeddedAnnotationResult = (
  data: z.infer<typeof EmbeddedAnnotationResultSchema>,
) => ({
  id: data.id,
  title: data.title,
  createdAt: data.created_at,
  updatedAt: data.updated_at ?? undefined,
  color: data.color,
  reference: data.reference,
  similarity: data.similarity,
});

const ArticleSchema = EntrySchema.extend({
  content: z.string(),
  annotations: z.array(AnnotationSchema).default([]),
});

const toArticle = (data: z.infer<typeof ArticleSchema>): Article => ({
  ...toEntry(data),
  content: data.content,
  annotations: data.annotations.map(toAnnotation),
});

// const AnyItemSchema = EntrySchema.or(ItemSchema);

// const toAnyItem = (data: z.infer<typeof AnyItemSchema>) => {
//   return "is_unread" in data ? toEntry(data) : toItem(data);
// };

const EmbeddedSchema = z.object({
  similarity: z.number(),
});

function toEmbedded(data: z.infer<typeof EmbeddedSchema>): Embedded {
  return { similarity: data.similarity };
}

const EmbeddedEntrySchema = EntrySchema.extend(EmbeddedSchema.shape);
const EmbeddedItemSchema = ItemSchema.extend(EmbeddedSchema.shape);
// EmbeddedAnnotationResultSchema is checked first: it requires `color` and `reference`,
// which regular entries never have, so it fails fast for non-annotation rows.
const AnyEmbeddedItemSchema =
  EmbeddedAnnotationResultSchema.or(EmbeddedEntrySchema).or(EmbeddedItemSchema);

const toAnyEmbeddedItem = (data: z.infer<typeof AnyEmbeddedItemSchema>) => {
  if ("color" in data) {
    return toEmbeddedAnnotationResult(
      data as z.infer<typeof EmbeddedAnnotationResultSchema>,
    );
  }
  if ("is_unread" in data) {
    return {
      ...toEntry(data as z.infer<typeof EmbeddedEntrySchema>),
      ...toEmbedded(data as z.infer<typeof EmbeddedSchema>),
    };
  }
  return {
    ...toItem(data as z.infer<typeof EmbeddedItemSchema>),
    ...toEmbedded(data as z.infer<typeof EmbeddedSchema>),
  };
};

export class TauriPlatform implements Platform {
  // Window --------------------------------------------------------------------

  async setWindowTitle(title: string): Promise<void> {
    try {
      const win = getCurrentWindow();
      await win.setTitle(title);
    } catch (e) {
      logger.error(e);
      throw e;
    }
  }

  openFileDialog(directory: boolean, multiple = false): Promise<string | null> {
    try {
      return open({ directory, multiple });
    } catch (e) {
      logger.error(e);
      throw e;
    }
  }

  async openUrl(url: string) {
    try {
      return openUrl(url);
    } catch (e) {
      logger.error(e);
      throw e;
    }
  }

  // Library -------------------------------------------------------------------

  async getLibrary(): Promise<string> {
    const invoker = decorate(invoke).with(logged, {
      logger,
      name: "getLibrary",
    });

    return invoker<string>("get_library");
  }

  async setLibrary(path: string): Promise<string> {
    const invoker = decorate(invoke).with(logged, {
      logger,
      name: "setLibrary",
    });

    return invoker<string>("set_library", { path });
  }

  // Feeds ---------------------------------------------------------------------

  async listFeeds(signal: AbortSignal): Promise<Feed[]> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "list_feeds" });

    const results = await invoker<unknown[]>("list_feeds");
    return results
      .map((e) => FeedSchema.transform(toFeed).safeParse(e))
      .filter((e) => !e.error) // FIXME Silently dropping invalid items
      .map((e) => e.data);
  }

  async addFeed(signal: AbortSignal, feedUrl: string): Promise<Feed> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "add_feed" });
    const result = await invoker("add_feed", { url: feedUrl });
    return FeedSchema.transform(toFeed).parse(result);
  }

  async refreshFeed(signal: AbortSignal, feedId: string): Promise<Feed> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "refresh_feed" });
    const result = await invoker("refresh_feed", { id: feedId });
    return FeedSchema.transform(toFeed).parse(result);
  }

  async renameFeed(
    signal: AbortSignal,
    feedId: string,
    title: string,
  ): Promise<Feed> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "rename_feed" });
    const result = await invoker("rename_feed", { id: feedId, title });
    return FeedSchema.transform(toFeed).parse(result);
  }

  async removeFeed(signal: AbortSignal, feedUrl: string): Promise<string> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "remove_feed" });

    return await invoker("remove_feed", { url: feedUrl });
  }

  async importFeeds(signal: AbortSignal): Promise<ImportResult<Feed> | null> {
    const filePath = (await open({
      filters: [{ name: "OPML", extensions: ["opml", "xml"] }],
    })) as string | null;
    if (!filePath) return null;

    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "import_feeds" });

    return invoker<ImportResult<Feed>>("import_feeds", { filePath });
  }

  async exportFeeds(signal: AbortSignal): Promise<void> {
    const filePath = (await save({
      filters: [{ name: "OPML", extensions: ["opml"] }],
      defaultPath: "feeds.opml",
    })) as string | null;
    if (!filePath) return;

    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "export_feeds" });

    await invoker("export_feeds", { filePath });
  }

  // Entries -------------------------------------------------------------------

  async listEntries(
    signal: AbortSignal,
    feedUrl: string,
    filter: Filter = "unread",
  ): Promise<Entry[]> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "list_entries" });

    const results = await invoker<unknown[]>("list_entries", {
      url: feedUrl,
      itemFilter: filter,
    });

    return results
      .map((e) => EntrySchema.transform(toEntry).safeParse(e))
      .filter((e) => !e.error) // FIXME silently dropping invalid items
      .map((e) => e.data);
  }

  async listAllEntries(
    signal: AbortSignal,
    itemFilter: Filter = "unread",
  ): Promise<Entry[]> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "list_entries" });

    const results = await invoker<unknown[]>("list_entries", { itemFilter });

    return results
      .map((e) => EntrySchema.transform(toEntry).safeParse(e))
      .filter((e) => !e.error) // FIXME silently dropping invalid items
      .map((e) => e.data);
  }

  async markEntryRead(signal: AbortSignal, id: string): Promise<Entry> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "mark_entry_read" });

    const result = await invoker<unknown>("mark_entry_read", { id });
    return EntrySchema.transform(toEntry).parse(result);
  }

  async markEntryUnread(signal: AbortSignal, id: string): Promise<Entry> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "mark_entry_unread" });

    const result = await invoker<unknown>("mark_entry_unread", { id });
    return EntrySchema.transform(toEntry).parse(result);
  }

  // Articles ------------------------------------------------------------------

  async loadArticle(signal: AbortSignal, url: string): Promise<string> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "load_article" });

    return invoker<string>("load_article", { url });
  }

  async findArticle(signal: AbortSignal, url: string): Promise<Article | null> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "find_article" });

    const result = await invoker<unknown | null>("find_article", { url });
    if (result === null || result === undefined) return null;
    return ArticleSchema.transform(toArticle).parse(result);
  }

  async saveArticle(signal: AbortSignal, article: Article): Promise<Article> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "save_article" });

    const result = await invoker<unknown>("save_article", {
      id: article.id,
      url: article.url,
      title: article.title,
      publishedAt: article.publishedAt,
      updatedAt: article.updatedAt,
      summary: article.summary,
      markdown: article.content,
    });

    return ArticleSchema.transform(toArticle).parse(result);
  }

  async readArticle(signal: AbortSignal, id: string): Promise<Article> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "read_article" });

    const result = await invoker<unknown>("read_article", { id });
    return ArticleSchema.transform(toArticle).parse(result);
  }

  async createArticle(
    signal: AbortSignal,
    article: Omit<Article, "id" | "createdAt">,
  ): Promise<Article> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "create_article" });

    const result = await invoker<unknown>("create_article", {
      url: article.url,
      title: article.title,
      publishedAt: article.publishedAt,
      updatedAt: article.updatedAt,
      summary: article.summary,
      markdown: article.content,
    });

    return ArticleSchema.transform(toArticle).parse(result);
  }

  // Annotations ---------------------------------------------------------------

  async addAnnotation(
    signal: AbortSignal,
    annotation: Omit<Annotation, "createdAt" | "updatedAt">,
  ): Promise<Article> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "add_annotation" });

    const result = await invoker<unknown>("add_annotation", {
      articleId: annotation.reference,
      id: annotation.id,
      text: annotation.text,
      occurrence: annotation.occurrence,
      color: annotation.color,
      note: annotation.note ?? null,
    });

    return ArticleSchema.transform(toArticle).parse(result);
  }

  async removeAnnotation(
    signal: AbortSignal,
    annotation: Pick<Annotation, "id" | "reference">,
  ): Promise<Article> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "remove_annotation" });

    const result = await invoker<unknown>("remove_annotation", {
      articleId: annotation.reference,
      annotationId: annotation.id,
    });
    return ArticleSchema.transform(toArticle).parse(result);
  }

  async listSaved(signal: AbortSignal): Promise<SavedItem[]> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "list_saved" });

    const results = await invoker<unknown[]>("list_saved");

    return results
      .map((e) => SavedItemSchema.transform(toSavedItem).safeParse(e))
      .filter((e) => !e.error)
      .map((e) => e.data);
  }

  async listAnnotations(signal: AbortSignal): Promise<AnnotationSummary[]> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "list_annotations" });

    const results = await invoker<unknown[]>("list_annotations");

    return results
      .map((e) => AnnotationSummarySchema.transform(toAnnotationSummary).safeParse(e))
      .filter((e) => !e.error)
      .map((e) => e.data);
  }

  // Topics --------------------------------------------------------------------

  async createTopic(
    signal: AbortSignal,
    topic: Omit<Topic, "id" | "createdAt" | "unreadCount">,
  ): Promise<Topic> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "create_topic" });

    const result = await invoker<unknown>("create_topic", topic);
    return TopicSchema.transform(toTopic).parse(result);
  }

  async saveTopic(signal: AbortSignal, topic: Topic): Promise<Topic> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "save_topic" });

    const result = await invoker<unknown>("save_topic", {
      id: topic.id,
      title: topic.title,
      content: topic.content,
      threshold: topic.threshold ?? null,
    });

    return TopicSchema.transform(toTopic).parse(result);
  }

  async deleteTopic(signal: AbortSignal, id: string): Promise<void> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "delete_topic" });

    await invoker<void>("delete_topic", { id });
  }

  async listTopics(signal: AbortSignal): Promise<Topic[]> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "list_topics" });

    const results = await invoker<unknown[]>("list_topics");

    return results
      .map((e) => TopicSchema.transform(toTopic).safeParse(e))
      .filter((e) => !e.error)
      .map((e) => e.data);
  }

  async readTopic(signal: AbortSignal, id: string): Promise<Topic> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "read_topic" });

    const result = await invoker<unknown>("read_topic", { id });
    return TopicSchema.transform(toTopic).parse(result);
  }

  // Bookmarks -----------------------------------------------------------------

  async addBookmark(signal: AbortSignal, id: string): Promise<Entry> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "add_bookmark" });

    const result = await invoker<unknown>("add_bookmark", { id });
    return EntrySchema.transform(toEntry).parse(result);
  }

  async removeBookmark(signal: AbortSignal, id: string): Promise<string> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "remove_bookmark" });

    return invoker("remove_bookmark", { id });
  }

  async importBookmarks(
    signal: AbortSignal,
  ): Promise<ImportResult<Entry> | null> {
    const filePath = (await open({
      filters: [{ name: "Bookmarks", extensions: ["html", "htm"] }],
    })) as string | null;
    if (!filePath) return null;

    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "import_bookmarks" });

    return invoker<ImportResult<Entry>>("import_bookmarks", { filePath });
  }

  async exportBookmarks(signal: AbortSignal): Promise<void> {
    const filePath = (await save({
      filters: [{ name: "Bookmarks", extensions: ["html"] }],
      defaultPath: "bookmarks.html",
    })) as string | null;
    if (!filePath) return;

    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "export_bookmarks" });

    await invoker("export_bookmarks", { filePath });
  }

  async listBookmarks(signal: AbortSignal): Promise<Entry[]> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "list_bookmarks" });

    const results = await invoker<unknown[]>("list_bookmarks");

    return results
      .map((e) => EntrySchema.transform(toEntry).safeParse(e))
      .filter((e) => !e.error) // FIXME silently dropping invalid items
      .map((e) => e.data);
  }

  // Embedding queries ---------------------------------------------------------

  async queryByItem(
    signal: AbortSignal,
    id: string,
    itemFilter: Filter = "all",
    minSimilarity: number | null = null,
    maxSimilarity: number | null = null,
    limit: number | null = null,
  ): Promise<(Embedded & Item)[]> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "query_by_item" });

    const results = await invoker<unknown[]>("query_by_item", {
      id,
      limit,
      minSimilarity,
      maxSimilarity,
      itemFilter,
    });

    return results
      .map((e) =>
        AnyEmbeddedItemSchema.transform(toAnyEmbeddedItem).safeParse(e),
      )
      .filter((e) => !e.error) // FIXME Dropping failed items silently!
      .map((e) => e.data);
  }

  async queryByBookmarks(
    signal: AbortSignal,
    itemFilter?: Filter,
    minSimilarity?: number | null,
    maxSimilarity?: number | null,
    limit?: number | null,
  ): Promise<(Embedded & Item)[]> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "query_by_bookmarks" });

    const results = await invoker<unknown[]>("query_by_bookmarks", {
      limit,
      minSimilarity,
      maxSimilarity,
      itemFilter,
    });

    return results
      .map((e) =>
        AnyEmbeddedItemSchema.transform(toAnyEmbeddedItem).safeParse(e),
      )
      .filter((e) => !e.error) // FIXME Dropping failed items silently!
      .map((e) => e.data);
  }

  async queryByTopics(
    signal: AbortSignal,
    itemFilter?: Filter,
    minSimilarity?: number | null,
    maxSimilarity?: number | null,
    limit?: number | null,
  ): Promise<(Embedded & Item)[]> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "query_by_topics" });

    const results = await invoker<unknown[]>("query_by_topics", {
      limit,
      minSimilarity,
      maxSimilarity,
      itemFilter,
    });

    return results
      .map((e) =>
        AnyEmbeddedItemSchema.transform(toAnyEmbeddedItem).safeParse(e),
      )
      .filter((e) => !e.error) // FIXME Dropping failed items silently!
      .map((e) => e.data);
  }

  async queryByAnnotations(
    signal: AbortSignal,
    itemFilter?: Filter,
    minSimilarity?: number | null,
    maxSimilarity?: number | null,
    limit?: number | null,
  ): Promise<(Embedded & Item)[]> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "query_by_annotations" });

    const results = await invoker<unknown[]>("query_by_annotations", {
      limit,
      minSimilarity,
      maxSimilarity,
      itemFilter,
    });

    return results
      .map((e) =>
        AnyEmbeddedItemSchema.transform(toAnyEmbeddedItem).safeParse(e),
      )
      .filter((e) => !e.error) // FIXME Dropping failed items silently!
      .map((e) => e.data);
  }

  async queryByText(
    signal: AbortSignal,
    text: string,
    itemFilter?: Filter,
    minSimilarity?: number | null,
    maxSimilarity?: number | null,
    limit?: number | null,
  ): Promise<(Embedded & Item)[]> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "query_by_text" });

    const results = await invoker<unknown[]>("query_by_text", {
      text,
      limit,
      minSimilarity,
      maxSimilarity,
      itemFilter,
    });

    return results
      .map((e) =>
        AnyEmbeddedItemSchema.transform(toAnyEmbeddedItem).safeParse(e),
      )
      .filter((e) => !e.error) // FIXME Dropping failed items silently!
      .map((e) => e.data);
  }

  // Events --------------------------------------------------------------------

  onOpenLibrary(callback: () => void): Promise<UnlistenFn> {
    return listen("menu:open-library", callback);
  }

  onOpenSettings(callback: () => void): Promise<UnlistenFn> {
    return listen("menu:open-settings", callback);
  }

  onImportFeeds(callback: () => void): Promise<UnlistenFn> {
    return listen("menu:import-feeds", callback);
  }

  onExportFeeds(callback: () => void): Promise<UnlistenFn> {
    return listen("menu:export-feeds", callback);
  }

  onFeedRefreshed(callback: (feed: Feed) => void): Promise<UnlistenFn> {
    return listen<unknown>("feed:refreshed", (e) => {
      const result = FeedSchema.transform(toFeed).safeParse(e.payload);
      if (result.success) callback(result.data);
    });
  }

  onUnreadChanged(callback: () => void): Promise<UnlistenFn> {
    return listen("unread:changed", callback);
  }

  onEmbeddingsChanged(callback: () => void): Promise<UnlistenFn> {
    return listen("embedding:changed", callback);
  }

  onImportBookmarks(callback: () => void): Promise<UnlistenFn> {
    return listen("menu:import-bookmarks", callback);
  }

  onExportBookmarks(callback: () => void): Promise<UnlistenFn> {
    return listen("menu:export-bookmarks", callback);
  }

  async getTheme(): Promise<string> {
    return invoke<string>("get_theme");
  }

  async setTheme(theme: string): Promise<void> {
    return invoke<void>("set_theme", { theme });
  }

  async getRefreshOnStartup(): Promise<boolean> {
    return invoke<boolean>("get_refresh_on_startup");
  }

  async setRefreshOnStartup(value: boolean): Promise<void> {
    return invoke<void>("set_refresh_on_startup", { value });
  }

  async getReaderPrefs(): Promise<{
    fontFamily: string;
    fontSize: number;
    lineHeight: number;
  }> {
    return invoke<{ fontFamily: string; fontSize: number; lineHeight: number }>(
      "get_reader_prefs",
    );
  }

  async setReaderFontFamily(value: string): Promise<void> {
    return invoke<void>("set_reader_font_family", { value });
  }

  async setReaderFontSize(value: number): Promise<void> {
    return invoke<void>("set_reader_font_size", { value });
  }

  async setReaderLineHeight(value: number): Promise<void> {
    return invoke<void>("set_reader_line_height", { value });
  }

  onSetTheme(callback: (theme: string) => void): Promise<UnlistenFn> {
    return listen<string>("menu:set-theme", (e) => callback(e.payload));
  }

  onShowArticleSummary(callback: (show: boolean) => void): Promise<UnlistenFn> {
    return listen<boolean>("menu:show-article-summary", (e) =>
      callback(e.payload),
    );
  }

  onShutdown(callback: () => void): Promise<UnlistenFn> {
    return listen("embedding:shutdown", callback);
  }

  onThemeChangedFromSettings(
    callback: (theme: string) => void,
  ): Promise<UnlistenFn> {
    return listen<string>("settings:theme", (e) => callback(e.payload));
  }

  onReaderFontFamilyChanged(
    callback: (value: string) => void,
  ): Promise<UnlistenFn> {
    return listen<string>("settings:reader-font-family", (e) =>
      callback(e.payload),
    );
  }

  onReaderFontSizeChanged(
    callback: (value: number) => void,
  ): Promise<UnlistenFn> {
    return listen<number>("settings:reader-font-size", (e) =>
      callback(e.payload),
    );
  }

  onReaderLineHeightChanged(
    callback: (value: number) => void,
  ): Promise<UnlistenFn> {
    return listen<number>("settings:reader-line-height", (e) =>
      callback(e.payload),
    );
  }

  onRefreshOnStartupChanged(
    callback: (value: boolean) => void,
  ): Promise<UnlistenFn> {
    return listen<boolean>("settings:refresh-on-startup", (e) =>
      callback(e.payload),
    );
  }
}
