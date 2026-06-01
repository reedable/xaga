import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open, save } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { z } from "zod";
import { isAnnotation } from "../types";
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
  is_bookmarked: z.boolean(),
});

const toEntry = (data: z.infer<typeof EntrySchema>): Entry => {
  return {
    ...toItem(data),
    url: data.url,
    publishedAt: data.published_at ?? undefined,
    summary: data.summary ?? undefined,
    isUnread: data.is_unread,
    isBookmarked: data.is_bookmarked,
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
  is_unread: z.boolean(),
  is_bookmarked: z.boolean(),
  reference: z.string().optional(),
  color: z.string().optional(),
});

const toSavedItem = (data: z.infer<typeof SavedItemSchema>): SavedItem => ({
  id: data.id,
  title: data.title,
  createdAt: data.created_at,
  url: data.url,
  isUnread: data.is_unread,
  isBookmarked: data.is_bookmarked,
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
  data: z.infer<typeof AnnotationSummarySchema>
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
  data: z.infer<typeof EmbeddedAnnotationResultSchema>
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
  favicon: z.string().nullable().optional(),
});

const toArticle = (data: z.infer<typeof ArticleSchema>): Article => ({
  ...toEntry(data),
  content: data.content,
  annotations: data.annotations.map(toAnnotation),
  favicon: data.favicon ?? undefined,
});

const EmbeddedSchema = z.object({
  similarity: z.number(),
});

function toEmbedded(data: z.infer<typeof EmbeddedSchema>): Embedded {
  return { similarity: data.similarity };
}

const EmbeddedEntrySchema = EntrySchema.extend(EmbeddedSchema.shape);
const EmbeddedTopicSchema = TopicSchema.extend(EmbeddedSchema.shape);
const EmbeddedItemSchema = ItemSchema.extend(EmbeddedSchema.shape);
// Detection order:
// 1. Annotation: has `color` + `reference` (entries never have these)
// 2. Entry: has `is_unread` (topics don't — they have no url/is_unread fields)
// 3. Topic: has `content` + `entry_count` + `unread_count`
// 4. Generic Item: fallback
const AnyEmbeddedItemSchema =
  EmbeddedAnnotationResultSchema
    .or(EmbeddedEntrySchema)
    .or(EmbeddedTopicSchema)
    .or(EmbeddedItemSchema);

const toAnyEmbeddedItem = (data: z.infer<typeof AnyEmbeddedItemSchema>) => {
  if ("color" in data) {
    return toEmbeddedAnnotationResult(
      data as z.infer<typeof EmbeddedAnnotationResultSchema>
    );
  }
  if ("is_unread" in data) {
    return {
      ...toEntry(data as z.infer<typeof EmbeddedEntrySchema>),
      ...toEmbedded(data as z.infer<typeof EmbeddedSchema>),
    };
  }
  if ("content" in data) {
    return {
      ...toTopic(data as z.infer<typeof TopicSchema>),
      ...toEmbedded(data as z.infer<typeof EmbeddedSchema>),
    };
  }
  return {
    ...toItem(data as z.infer<typeof EmbeddedItemSchema>),
    ...toEmbedded(data as z.infer<typeof EmbeddedSchema>),
  };
};

listen<string>("loader:will-unload", (e) => {
  console.log("[diagnostic] loader:will-unload", e.payload);
});

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

  async isCacheSeeded(): Promise<boolean> {
    const name = "is_cache_seeded";
    const invoker = decorate(invoke).with(logged, { logger, name });
    return invoker<boolean>(name);
  }

  async getLibrary(): Promise<string> {
    const name = "get_library";
    const invoker = decorate(invoke).with(logged, { logger, name });
    return invoker<string>(name);
  }

  async setLibrary(path: string): Promise<string> {
    const name = "set_library";
    const invoker = decorate(invoke).with(logged, { logger, name });
    return invoker<string>(name, { path });
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
    title: string
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

    const name = "import_feeds";
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name });

    return invoker<ImportResult<Feed>>(name, { filePath });
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
    filter: Filter = "unread"
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

  async listAllItems(
    signal: AbortSignal,
    itemFilter: Filter = "unread"
  ): Promise<SavedItem[]> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "list_all_items" });

    const results = await invoker<unknown[]>("list_all_items", { itemFilter });

    return results
      .map((e) => SavedItemSchema.transform(toSavedItem).safeParse(e))
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

  async loadArticle(
    signal: AbortSignal,
    url: string,
    keepOpen?: boolean
  ): Promise<{ mimeType: string; html: string }> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "load_article" });

    return invoker<{ mimeType: string; html: string }>("load_article", {
      url,
      keepOpen: keepOpen ?? false,
    });
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
      favicon: article.favicon ?? null,
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
    article: Omit<Article, "id" | "createdAt">
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
      favicon: article.favicon ?? null,
      markdown: article.content,
    });

    return ArticleSchema.transform(toArticle).parse(result);
  }

  // Annotations ---------------------------------------------------------------

  async addAnnotation(
    signal: AbortSignal,
    annotation: Omit<Annotation, "createdAt" | "updatedAt">
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
    annotation: Pick<Annotation, "id" | "reference">
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
      .map((e) =>
        AnnotationSummarySchema.transform(toAnnotationSummary).safeParse(e)
      )
      .filter((e) => !e.error)
      .map((e) => e.data);
  }

  // Topics --------------------------------------------------------------------

  async createTopic(
    signal: AbortSignal,
    topic: Omit<Topic, "id" | "createdAt" | "unreadCount">
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

  async listTopicItems(
    signal: AbortSignal,
    id: string,
    filter: Filter
  ): Promise<SavedItem[]> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "list_topic_items" });
    const results = await invoker<unknown[]>("list_topic_items", { id, itemFilter: filter });
    return results
      .map((e) => SavedItemSchema.transform(toSavedItem).safeParse(e))
      .filter((e) => !e.error)
      .map((e) => e.data);
  }

  async addTopicItem(
    signal: AbortSignal,
    topicId: string,
    item: Entry | Annotation
  ): Promise<Topic> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "add_topic_item" });
    const topicItem = isAnnotation(item)
      ? { type: "annotation", id: item.id, reference: item.reference }
      : { type: "entry", id: item.id };
    const result = await invoker<unknown>("add_topic_item", { topicId, item: topicItem });
    return TopicSchema.transform(toTopic).parse(result);
  }

  async removeTopicItem(
    signal: AbortSignal,
    topicId: string,
    itemId: string
  ): Promise<Topic> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "remove_topic_item" });
    const result = await invoker<unknown>("remove_topic_item", { topicId, itemId });
    return TopicSchema.transform(toTopic).parse(result);
  }

  async listEntryTopics(signal: AbortSignal, entryId: string): Promise<string[]> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "list_entry_topics" });
    return invoker<string[]>("list_entry_topics", { entryId });
  }

  async rankTopicsRelatedToItem(
    signal: AbortSignal,
    itemId: string,
    minSimilarity?: number,
  ): Promise<Array<{ id: string } & Embedded>> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "rank_topics_by_item" });
    return invoker<Array<{ id: string } & Embedded>>("rank_topics_by_item", {
      itemId,
      minSimilarity: minSimilarity ?? null,
    });
  }

  // Bookmarks -----------------------------------------------------------------

  async addBookmark(signal: AbortSignal, id: string): Promise<Entry> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "add_bookmark" });

    const result = await invoker<unknown>("add_bookmark", { id });
    return EntrySchema.transform(toEntry).parse(result);
  }

  async removeBookmark(signal: AbortSignal, id: string): Promise<Entry> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "remove_bookmark" });

    const result = await invoker<unknown>("remove_bookmark", { id });
    return EntrySchema.transform(toEntry).parse(result);
  }

  async importBookmarks(
    signal: AbortSignal
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
    limit: number | null = null
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
        AnyEmbeddedItemSchema.transform(toAnyEmbeddedItem).safeParse(e)
      )
      .filter((e) => !e.error) // FIXME Dropping failed items silently!
      .map((e) => e.data);
  }

  async queryByBookmarks(
    signal: AbortSignal,
    itemFilter?: Filter,
    minSimilarity?: number | null,
    maxSimilarity?: number | null,
    limit?: number | null
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
        AnyEmbeddedItemSchema.transform(toAnyEmbeddedItem).safeParse(e)
      )
      .filter((e) => !e.error) // FIXME Dropping failed items silently!
      .map((e) => e.data);
  }

  async queryByTopics(
    signal: AbortSignal,
    itemFilter?: Filter,
    minSimilarity?: number | null,
    maxSimilarity?: number | null,
    limit?: number | null
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
        AnyEmbeddedItemSchema.transform(toAnyEmbeddedItem).safeParse(e)
      )
      .filter((e) => !e.error) // FIXME Dropping failed items silently!
      .map((e) => e.data);
  }

  async queryByAnnotations(
    signal: AbortSignal,
    itemFilter?: Filter,
    minSimilarity?: number | null,
    maxSimilarity?: number | null,
    limit?: number | null
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
        AnyEmbeddedItemSchema.transform(toAnyEmbeddedItem).safeParse(e)
      )
      .filter((e) => !e.error) // FIXME Dropping failed items silently!
      .map((e) => e.data);
  }

  async queryBySaved(
    signal: AbortSignal,
    itemFilter?: Filter,
    minSimilarity?: number | null,
    maxSimilarity?: number | null,
    limit?: number | null
  ): Promise<(Embedded & Item)[]> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "query_by_saved" });

    const results = await invoker<unknown[]>("query_by_saved", {
      limit,
      minSimilarity,
      maxSimilarity,
      itemFilter,
    });

    return results
      .map((e) =>
        AnyEmbeddedItemSchema.transform(toAnyEmbeddedItem).safeParse(e)
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
    limit?: number | null
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
        AnyEmbeddedItemSchema.transform(toAnyEmbeddedItem).safeParse(e)
      )
      .filter((e) => !e.error) // FIXME Dropping failed items silently!
      .map((e) => e.data);
  }

  // Events --------------------------------------------------------------------

  onOpenLibrary(callback: () => void): Promise<UnlistenFn> {
    const name = "menu:open-library";
    const listener = decorate(callback).with(logged, { logger, name });
    return listen(name, listener);
  }

  onOpenSettings(callback: () => void): Promise<UnlistenFn> {
    const name = "menu:open-settings";
    const listener = decorate(callback).with(logged, { logger, name });
    return listen(name, listener);
  }

  onImportFeeds(callback: () => void): Promise<UnlistenFn> {
    const name = "menu:import-feeds";
    const listener = decorate(callback).with(logged, { logger, name });
    return listen(name, listener);
  }

  onExportFeeds(callback: () => void): Promise<UnlistenFn> {
    const name = "menu:export-feeds";
    const listener = decorate(callback).with(logged, { logger, name });
    return listen(name, listener);
  }

  onFeedRefreshed(callback: (feed: Feed) => void): Promise<UnlistenFn> {
    const name = "feed:refreshed";
    const listener = decorate((e) => {
      const result = FeedSchema.transform(toFeed).safeParse(e.payload);
      if (result.success) callback(result.data);
    }).with(logged, { logger, name });

    return listen<unknown>(name, listener);
  }

  onUnreadChanged(callback: (entry: Entry) => void): Promise<UnlistenFn> {
    const name = "unread:changed";
    const listener = decorate((e) => {
      const result = EntrySchema.transform(toEntry).safeParse(e.payload);
      if (result.success) callback(result.data);
    }).with(logged, { logger, name });

    return listen<unknown>(name, listener);
  }

  onBookmarkChanged(callback: (entry: Entry) => void): Promise<UnlistenFn> {
    const name = "bookmark:changed";
    const listener = decorate((e) => {
      const result = EntrySchema.transform(toEntry).safeParse(e.payload);
      if (result.success) callback(result.data);
    }).with(logged, { logger, name });

    return listen<unknown>(name, listener);
  }

  onAnnotationChanged(
    callback: (item: SavedItem) => void
  ): Promise<UnlistenFn> {
    const name = "annotation:changed";
    const listener = decorate((e) => {
      const result = SavedItemSchema.transform(toSavedItem).safeParse(
        e.payload
      );
      if (result.success) callback(result.data);
    }).with(logged, { logger, name });

    return listen<unknown>(name, listener);
  }

  onAnnotationRemoved(callback: (id: string) => void): Promise<UnlistenFn> {
    const name = "annotation:removed";
    const listener = decorate((e: { payload: string }) => callback(e.payload))
      .with(logged, { logger, name });
    return listen<string>(name, listener);
  }

  onEmbeddingsChanged(callback: () => void): Promise<UnlistenFn> {
    const name = "embedding:changed";
    const listener = decorate(callback).with(logged, { logger, name });
    return listen(name, listener);
  }

  onImportBookmarks(callback: () => void): Promise<UnlistenFn> {
    const name = "menu:import-bookmarks";
    const listener = decorate(callback).with(logged, { logger, name });
    return listen(name, listener);
  }

  onExportBookmarks(callback: () => void): Promise<UnlistenFn> {
    const name = "menu:export-bookmarks";
    const listener = decorate(callback).with(logged, { logger, name });
    return listen(name, listener);
  }

  async getTheme(): Promise<string> {
    const name = "get_theme";
    const invoker = decorate(invoke).with(logged, { logger, name });
    return invoker<string>(name);
  }

  async setTheme(theme: string): Promise<void> {
    const name = "set_theme";
    const invoker = decorate(invoke).with(logged, { logger, name });
    return invoker<void>(name, { theme });
  }

  async getRefreshOnStartup(): Promise<boolean> {
    const name = "get_refresh_on_startup";
    const invoker = decorate(invoke).with(logged, { logger, name });
    return invoker<boolean>(name);
  }

  async setRefreshOnStartup(value: boolean): Promise<void> {
    const name = "set_refresh_on_startup";
    const invoker = decorate(invoke).with(logged, { logger, name });
    return invoker<void>(name, { value });
  }

  async getHtmlReaderPrefs(): Promise<{
    fontFamily: string;
    fontSize: number;
    lineHeight: number;
  }> {
    const name = "get_html_reader_prefs";
    const invoker = decorate(invoke).with(logged, { logger, name });
    return invoker<{
      fontFamily: string;
      fontSize: number;
      lineHeight: number;
    }>(name);
  }

  async setHtmlReaderFontFamily(value: string): Promise<void> {
    const name = "set_html_reader_font_family";
    const invoker = decorate(invoke).with(logged, { logger, name });
    return invoker<void>(name, { value });
  }

  async setHtmlReaderFontSize(value: number): Promise<void> {
    const name = "set_html_reader_font_size";
    const invoker = decorate(invoke).with(logged, { logger, name });
    return invoker<void>(name, { value });
  }

  async setHtmlReaderLineHeight(value: number): Promise<void> {
    const name = "set_html_reader_line_height";
    const invoker = decorate(invoke).with(logged, { logger, name });
    return invoker<void>(name, { value });
  }

  async getShowSidebar(): Promise<boolean> {
    const name = "get_show_sidebar";
    const invoker = decorate(invoke).with(logged, { logger, name });
    return invoker<boolean>(name);
  }

  async setShowSidebar(value: boolean): Promise<void> {
    const name = "set_show_sidebar";
    const invoker = decorate(invoke).with(logged, { logger, name });
    return invoker<void>(name, { value });
  }

  onSetTheme(callback: (theme: string) => void): Promise<UnlistenFn> {
    const name = "menu:set-theme";
    const listener = decorate((e: { payload: string }) =>
      callback(e.payload)
    ).with(logged, { logger, name });
    return listen<string>(name, listener);
  }

  onShowArticleSummary(callback: (show: boolean) => void): Promise<UnlistenFn> {
    const name = "menu:show-article-summary";
    const listener = decorate((e: { payload: boolean }) =>
      callback(e.payload)
    ).with(logged, { logger, name });
    return listen<boolean>(name, listener);
  }

  onShowSidebar(callback: (show: boolean) => void): Promise<UnlistenFn> {
    const name = "menu:show-sidebar";
    const listener = decorate((e: { payload: boolean }) =>
      callback(e.payload)
    ).with(logged, { logger, name });
    return listen<boolean>(name, listener);
  }

  onLoaderClosed(
    callback: (data: { url: string; html: string; mimeType: string }) => void
  ): Promise<UnlistenFn> {
    const name = "loader:closed";
    const listener = decorate((e: { payload: { url: string; html: string; mimeType: string } }) =>
      callback(e.payload)
    ).with(logged, { logger, name });
    return listen<{ url: string; html: string; mimeType: string }>(name, listener);
  }

  onLoaderPageLoaded(
    callback: (data: { url: string; html: string; mimeType: string }) => void
  ): Promise<UnlistenFn> {
    const name = "loader:page-loaded";
    const listener = decorate((e: { payload: { url: string; html: string; mimeType: string } }) =>
      callback(e.payload)
    ).with(logged, { logger, name });
    return listen<{ url: string; html: string; mimeType: string }>(name, listener);
  }

  onFind(callback: () => void): Promise<UnlistenFn> {
    const name = "menu:find";
    const listener = decorate(callback).with(logged, { logger, name });
    return listen(name, listener);
  }

  onClearHistory(callback: () => void): Promise<UnlistenFn> {
    const name = "menu:clear-history";
    const listener = decorate(callback).with(logged, { logger, name });
    return listen(name, listener);
  }

  clearWebviewHistory(): Promise<void> {
    const name = "clear_webview_history";
    const invoker = decorate(invoke).with(logged, { logger, name });
    return invoker<void>(name);
  }

  getUiPref<T>(key: string): Promise<T | null> {
    const name = "get_ui_pref";
    const invoker = decorate(invoke).with(logged, { logger, name });
    return invoker<T | null>(name, { key });
  }

  setUiPref<T>(key: string, value: T): Promise<void> {
    const name = "set_ui_pref";
    const invoker = decorate(invoke).with(logged, { logger, name });
    return invoker<void>(name, { key, value });
  }

  getAllowedDomains(): Promise<string[]> {
    const name = "get_allowed_domains";
    const invoker = decorate(invoke).with(logged, { logger, name });
    return invoker<string[]>(name);
  }

  setAllowedDomains(domains: string[]): Promise<void> {
    const name = "set_allowed_domains";
    const invoker = decorate(invoke).with(logged, { logger, name });
    return invoker<void>(name, { domains });
  }

  onAllowedDomainsChanged(callback: (domains: string[]) => void): Promise<UnlistenFn> {
    const name = "settings:allowed-domains";
    const listener = decorate((e: { payload: string[] }) =>
      callback(e.payload)
    ).with(logged, { logger, name });
    return listen<string[]>(name, listener);
  }

  setFindEnabled(enabled: boolean): Promise<void> {
    const name = "set_find_enabled";
    const invoker = decorate(invoke).with(logged, { logger, name });
    return invoker<void>(name, { enabled });
  }

  onShutdown(callback: () => void): Promise<UnlistenFn> {
    const name = "embedding:shutdown";
    const listener = decorate(callback).with(logged, { logger, name });
    return listen(name, listener);
  }

  onThemeChangedFromSettings(
    callback: (theme: string) => void
  ): Promise<UnlistenFn> {
    const name = "settings:theme";
    const listener = decorate((e: { payload: string }) =>
      callback(e.payload)
    ).with(logged, { logger, name });
    return listen<string>(name, listener);
  }

  onHtmlReaderFontFamilyChanged(
    callback: (value: string) => void
  ): Promise<UnlistenFn> {
    const name = "settings:html-reader-font-family";
    const listener = decorate((e: { payload: string }) =>
      callback(e.payload)
    ).with(logged, { logger, name });
    return listen<string>(name, listener);
  }

  onHtmlReaderFontSizeChanged(
    callback: (value: number) => void
  ): Promise<UnlistenFn> {
    const name = "settings:html-reader-font-size";
    const listener = decorate((e: { payload: number }) =>
      callback(e.payload)
    ).with(logged, { logger, name });
    return listen<number>(name, listener);
  }

  onHtmlReaderLineHeightChanged(
    callback: (value: number) => void
  ): Promise<UnlistenFn> {
    const name = "settings:html-reader-line-height";
    const listener = decorate((e: { payload: number }) =>
      callback(e.payload)
    ).with(logged, { logger, name });
    return listen<number>(name, listener);
  }

  onRefreshOnStartupChanged(
    callback: (value: boolean) => void
  ): Promise<UnlistenFn> {
    const name = "settings:refresh-on-startup";
    const listener = decorate((e: { payload: boolean }) =>
      callback(e.payload)
    ).with(logged, { logger, name });
    return listen<boolean>(name, listener);
  }
}
