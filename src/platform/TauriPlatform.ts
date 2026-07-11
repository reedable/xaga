import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { z } from "zod";

import type { AppEvent, DomainEvent } from "./events";
import type {
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
  ExportOutcome,
  Item,
  ListItem,
  Topic,
  TopicSort,
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
  is_pinned: z.boolean(),
});

const toFeed = (data: z.infer<typeof FeedSchema>): Feed => {
  return {
    ...toItem(data),
    __kind: "feed",
    url: data.url,
    entryCount: data.entry_count,
    unreadCount: data.unread_count,
    isPinned: data.is_pinned,
    members: [],
  };
};

const EntrySchema = ItemSchema.extend({
  url: z.string(),
  published_at: z.string().nullable(),
  description: z.string().nullable(),
  is_unread: z.boolean(),
  is_bookmarked: z.boolean(),
  color: z.string().nullable().optional(),
});

const toEntry = (data: z.infer<typeof EntrySchema>): Entry => {
  return {
    ...toItem(data),
    __kind: "entry",
    url: data.url,
    publishedAt: data.published_at ?? undefined,
    description: data.description ?? undefined,
    isUnread: data.is_unread,
    isBookmarked: data.is_bookmarked,
    color: data.color ?? undefined,
  };
};

const TopicSchema = ItemSchema.extend({
  description: z.string().nullable().optional(),
  threshold: z.number().nullable().optional(),
  entry_count: z.number(),
  unread_count: z.number(),
  is_pinned: z.boolean(),
});

const toTopic = (data: z.infer<typeof TopicSchema>): Topic => ({
  ...toItem(data),
  __kind: "topic",
  description: data.description ?? undefined,
  threshold: data.threshold ?? undefined,
  entryCount: data.entry_count,
  unreadCount: data.unread_count,
  isPinned: data.is_pinned,
  members: [],
});

const AnnotationSchema = z.object({
  id: z.string(),
  title: z.string(),
  reference: z.string(),
  text: z.array(z.string()),
  occurrence: z.array(z.number()),
  color: z.string(),
  content: z.string(),
  created_at: z.string(),
});

const toAnnotation = (data: z.infer<typeof AnnotationSchema>): Annotation => ({
  __kind: "annotation",
  id: data.id,
  title: data.title,
  reference: data.reference,
  text: data.text,
  occurrence: data.occurrence,
  color: data.color,
  content: data.content,
  createdAt: data.created_at,
});

const ListItemSchema = z.object({
  id: z.string(),
  title: z.string(),
  created_at: z.string(),
  url: z.string(),
  is_unread: z.boolean(),
  is_bookmarked: z.boolean(),
  reference: z.string().optional(),
  color: z.string().optional(),
});

const toListItem = (data: z.infer<typeof ListItemSchema>): ListItem =>
  data.reference !== undefined
    ? // annotation row -> PartialAnnotation
      {
        id: data.id,
        reference: data.reference,
        title: data.title,
        color: data.color ?? "",
        createdAt: data.created_at,
        url: data.url,
      }
    : // bookmark row -> Entry
      {
        __kind: "entry",
        id: data.id,
        title: data.title,
        createdAt: data.created_at,
        url: data.url,
        isUnread: data.is_unread,
        isBookmarked: data.is_bookmarked,
      };

const PartialAnnotationSchema = z.object({
  id: z.string(),
  reference: z.string(),
  title: z.string(),
  color: z.string(),
  created_at: z.string(),
  url: z.string(),
});

const toPartialAnnotation = (
  data: z.infer<typeof PartialAnnotationSchema>
): PartialAnnotation => ({
  id: data.id,
  reference: data.reference,
  title: data.title,
  color: data.color,
  createdAt: data.created_at,
  url: data.url,
});

// Schema for annotation rows returned from embedding queries.
// These carry only the fields stored in LanceDB — no text[] or occurrence[].
// `url` is the row's `source` column (the parent article url), flattened from the
// EmbeddedEntry `entry.url` by the Rust query path.
const EmbeddedAnnotationResultSchema = z.object({
  id: z.string(),
  title: z.string(),
  created_at: z.string(),
  updated_at: z.string().nullable().optional(),
  color: z.string(),
  reference: z.string(),
  url: z.string(),
  kind: z.string(), // the row brand (RowKind::as_str); "annotation" — used to dispatch below
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
  url: data.url,
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

function toEmbedded(data: z.infer<typeof EmbeddedSchema>): { similarity: number } {
  return { similarity: data.similarity };
}

// `kind` is the backend discriminator (RowKind::as_str); minted into `__kind` by
// toEntry/toFeed at this boundary.
const EmbeddedEntrySchema = EntrySchema.extend(EmbeddedSchema.shape).extend({
  kind: z.string(),
});
const EmbeddedFeedSchema = FeedSchema.extend(EmbeddedSchema.shape).extend({
  kind: z.string(),
});
const EmbeddedTopicSchema = TopicSchema.extend(EmbeddedSchema.shape).extend({
  kind: z.string(),
});
const EmbeddedItemSchema = ItemSchema.extend(EmbeddedSchema.shape);
// Detection order (each branch identified by brand AND shape together):
// 1. Annotation: `kind === "annotation"`; shape guaranteed by the schema's required `reference`
//    (entries/feeds/topics never serialize it, so coloring an entry can't collide here)
// 2. Feed: `kind === "feed"` (feed/entry share shape — only the brand separates them)
// 3. Topic: `kind === "topic"` (a Topic has no url/is_unread/content — only the brand is reliable)
// 4. Entry: has `is_unread` (topics don't — they have no url/is_unread fields)
// 5. Generic Item: fallback
const AnyEmbeddedItemSchema = EmbeddedAnnotationResultSchema.or(EmbeddedFeedSchema)
  .or(EmbeddedEntrySchema)
  .or(EmbeddedTopicSchema)
  .or(EmbeddedItemSchema);

const toAnyEmbeddedItem = (data: z.infer<typeof AnyEmbeddedItemSchema>) => {
  if ("kind" in data && data.kind === "annotation") {
    return toEmbeddedAnnotationResult(
      data as z.infer<typeof EmbeddedAnnotationResultSchema>
    );
  }
  if ("kind" in data && data.kind === "feed") {
    return {
      ...toFeed(data as z.infer<typeof EmbeddedFeedSchema>),
      ...toEmbedded(data as z.infer<typeof EmbeddedSchema>),
    };
  }
  if ("kind" in data && data.kind === "topic") {
    return {
      ...toTopic(data as z.infer<typeof TopicSchema>),
      ...toEmbedded(data as z.infer<typeof EmbeddedSchema>),
    };
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

listen<string>("loader:will-unload" satisfies AppEvent, (e) => {
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
      // Hand only web/mail URLs to the system opener. file:, javascript:, data:
      // and custom app schemes must never reach `open`, which would launch
      // Finder or a registered handler for attacker-controlled links.
      const scheme = new URL(url).protocol;
      if (scheme !== "http:" && scheme !== "https:" && scheme !== "mailto:") {
        logger.error(`refusing to open URL with disallowed scheme: ${scheme}`);
        return;
      }
      return openUrl(url);
    } catch (e) {
      logger.error(e);
      throw e;
    }
  }

  // Library -------------------------------------------------------------------

  async isCacheReady(): Promise<boolean> {
    const name = "is_cache_ready";
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

  async isLibraryReadOnly(): Promise<boolean> {
    const name = "is_library_read_only";
    const invoker = decorate(invoke).with(logged, { logger, name });
    return invoker<boolean>(name);
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

  async refreshAllFeeds(signal: AbortSignal): Promise<Feed[]> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "refresh_all_feeds" });
    const result = await invoker("refresh_all_feeds", {});
    return z.array(FeedSchema.transform(toFeed)).parse(result);
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
  ): Promise<ListItem[]> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "list_all_items" });

    const results = await invoker<unknown[]>("list_all_items", { itemFilter });

    return results
      .map((e) => ListItemSchema.transform(toListItem).safeParse(e))
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
  ): Promise<{
    mimeType: string;
    html: string;
    pdfBase64?: string | null;
  }> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "load_article" });

    return invoker<{
      mimeType: string;
      html: string;
      pdfBase64?: string | null;
    }>("load_article", {
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
      summary: article.description,
      favicon: article.favicon ?? null,
      markdown: article.content,
    });

    return ArticleSchema.transform(toArticle).parse(result);
  }

  async readEntry(signal: AbortSignal, id: string): Promise<Entry> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "read_entry" });

    const result = await invoker<unknown>("read_entry", { id });
    return EntrySchema.transform(toEntry).parse(result);
  }

  async readArticle(signal: AbortSignal, id: string): Promise<Article> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "read_article" });

    const result = await invoker<unknown>("read_article", { id });
    return ArticleSchema.transform(toArticle).parse(result);
  }

  // `__kind` is minted on the way back by ArticleSchema.transform(toArticle)
  // below, so it's omitted from the input. Tactical fix — see TECH_DEBT.md
  // "Frontend type branding (`__kind`)".
  async createArticle(
    signal: AbortSignal,
    article: Omit<Article, "id" | "createdAt" | "__kind" | "isBookmarked">
  ): Promise<Article> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "create_article" });

    const result = await invoker<unknown>("create_article", {
      url: article.url,
      title: article.title,
      publishedAt: article.publishedAt,
      updatedAt: article.updatedAt,
      summary: article.description,
      favicon: article.favicon ?? null,
      markdown: article.content,
    });

    return ArticleSchema.transform(toArticle).parse(result);
  }

  // Annotations ---------------------------------------------------------------

  async addAnnotation(
    signal: AbortSignal,
    annotation: Omit<Annotation, "createdAt" | "updatedAt" | "__kind">
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
      note: annotation.content || null,
    });

    return ArticleSchema.transform(toArticle).parse(result);
  }

  async replaceAnnotation(
    signal: AbortSignal,
    oldId: string,
    annotation: Omit<Annotation, "createdAt" | "updatedAt" | "__kind">
  ): Promise<Article> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "replace_annotation" });

    const result = await invoker<unknown>("replace_annotation", {
      articleId: annotation.reference,
      oldId,
      id: annotation.id,
      text: annotation.text,
      occurrence: annotation.occurrence,
      color: annotation.color,
      note: annotation.content || null,
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

  async listAnnotations(signal: AbortSignal): Promise<PartialAnnotation[]> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "list_annotations" });

    const results = await invoker<unknown[]>("list_annotations");

    return results
      .map((e) =>
        PartialAnnotationSchema.transform(toPartialAnnotation).safeParse(e)
      )
      .filter((e) => !e.error)
      .map((e) => e.data);
  }

  // Topics --------------------------------------------------------------------

  async createTopic(
    signal: AbortSignal,
    topic: Omit<
      Topic,
      "id" | "createdAt" | "unreadCount" | "entryCount" | "isPinned" | "__kind" | "members"
    >
  ): Promise<Topic> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "create_topic" });

    const result = await invoker<unknown>("create_topic", {
      title: topic.title,
      description: topic.description ?? null,
      threshold: topic.threshold ?? null,
    });
    return TopicSchema.transform(toTopic).parse(result);
  }

  async saveTopic(signal: AbortSignal, topic: Topic): Promise<Topic> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "save_topic" });

    const result = await invoker<unknown>("save_topic", {
      id: topic.id,
      title: topic.title,
      description: topic.description ?? null,
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
    filter: Filter,
    sort: TopicSort = "relevance"
  ): Promise<ListItem[]> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "list_topic_items" });
    const results = await invoker<unknown[]>("list_topic_items", {
      id,
      itemFilter: filter,
      sort,
    });
    return results
      .map((e) => ListItemSchema.transform(toListItem).safeParse(e))
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
    // `Entry | Annotation` is a discriminated union on `__kind`; dispatch on the brand.
    const topicItem =
      item.__kind === "annotation"
        ? { type: "annotation", id: item.id, reference: item.reference }
        : { type: "entry", id: item.id };
    const result = await invoker<unknown>("add_topic_item", {
      topicId,
      item: topicItem,
    });
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
    const result = await invoker<unknown>("remove_topic_item", {
      topicId,
      itemId,
    });
    return TopicSchema.transform(toTopic).parse(result);
  }

  async listEntryTopics(
    signal: AbortSignal,
    entryId: string
  ): Promise<string[]> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "list_entry_topics" });
    return invoker<string[]>("list_entry_topics", { entryId });
  }

  async rankTopicsRelatedToItem(
    signal: AbortSignal,
    itemId: string,
    minSimilarity?: number
  ): Promise<RankedItem[]> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "rank_topics_by_item" });
    return invoker<RankedItem[]>("rank_topics_by_item", {
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

  // Embedding queries ---------------------------------------------------------

  async queryByItem(
    signal: AbortSignal,
    id: string,
    itemFilter: Filter = "all",
    minSimilarity: number | null = null,
    maxSimilarity: number | null = null,
    limit: number | null = null
  ): Promise<Ranked<Item>[]> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "search_by_item" });

    const results = await invoker<unknown[]>("search_by_item", {
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

  async queryByText(
    signal: AbortSignal,
    text: string,
    itemFilter?: Filter,
    minSimilarity?: number | null,
    maxSimilarity?: number | null,
    limit?: number | null
  ): Promise<Ranked<Item>[]> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "search_by_text" });

    const results = await invoker<unknown[]>("search_by_text", {
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
    const name: AppEvent = "menu:open-library";
    const listener = decorate(callback).with(logged, { logger, name });
    return listen(name, listener);
  }

  onOpenSettings(callback: () => void): Promise<UnlistenFn> {
    const name = "menu:open-settings";
    const listener = decorate(callback).with(logged, { logger, name });
    return listen(name, listener);
  }

  onFeedsImported(
    callback: (outcome: ImportOutcome<Feed>) => void
  ): Promise<UnlistenFn> {
    const name: AppEvent = "feeds:imported";
    const listener = decorate((e: { payload: ImportOutcome<Feed> }) =>
      callback(e.payload)
    ).with(logged, { logger, name });
    return listen<ImportOutcome<Feed>>(name, listener);
  }

  onFeedsExported(
    callback: (outcome: ExportOutcome) => void
  ): Promise<UnlistenFn> {
    const name: AppEvent = "feeds:exported";
    const listener = decorate((e: { payload: ExportOutcome }) =>
      callback(e.payload)
    ).with(logged, { logger, name });
    return listen<ExportOutcome>(name, listener);
  }

  onFeedRefreshed(callback: (feed: Feed) => void): Promise<UnlistenFn> {
    const name: DomainEvent = "feed:refreshed";
    const listener = decorate((e) => {
      const result = FeedSchema.transform(toFeed).safeParse(e.payload);
      if (result.success) callback(result.data);
    }).with(logged, { logger, name });

    return listen<unknown>(name, listener);
  }

  onFeedsRefreshed(callback: () => void): Promise<UnlistenFn> {
    const name: DomainEvent = "feeds:refreshed";
    const listener = decorate(() => callback()).with(logged, { logger, name });
    return listen(name, listener);
  }

  onLibraryModeChanged(
    callback: (isLibraryReadOnly: boolean) => void
  ): Promise<UnlistenFn> {
    const name: DomainEvent = "library:mode-changed";
    const listener = decorate((e: { payload: unknown }) => {
      if (typeof e.payload === "boolean") callback(e.payload);
    }).with(logged, { logger, name });
    return listen<unknown>(name, listener);
  }

  onUnreadChanged(callback: (entry: Entry) => void): Promise<UnlistenFn> {
    const name: DomainEvent = "unread:changed";
    const listener = decorate((e) => {
      const result = EntrySchema.transform(toEntry).safeParse(e.payload);
      if (result.success) callback(result.data);
    }).with(logged, { logger, name });

    return listen<unknown>(name, listener);
  }

  onBookmarkChanged(callback: (entry: Entry) => void): Promise<UnlistenFn> {
    const name: DomainEvent = "bookmark:changed";
    const listener = decorate((e) => {
      const result = EntrySchema.transform(toEntry).safeParse(e.payload);
      if (result.success) callback(result.data);
    }).with(logged, { logger, name });

    return listen<unknown>(name, listener);
  }

  onAnnotationChanged(
    callback: (item: ListItem) => void
  ): Promise<UnlistenFn> {
    const name: DomainEvent = "annotation:changed";
    const listener = decorate((e) => {
      const result = ListItemSchema.transform(toListItem).safeParse(
        e.payload
      );
      if (result.success) callback(result.data);
    }).with(logged, { logger, name });

    return listen<unknown>(name, listener);
  }

  onAnnotationRemoved(callback: (id: string) => void): Promise<UnlistenFn> {
    const name: DomainEvent = "annotation:removed";
    const listener = decorate((e: { payload: string }) =>
      callback(e.payload)
    ).with(logged, { logger, name });
    return listen<string>(name, listener);
  }

  onEmbeddingsChanged(callback: () => void): Promise<UnlistenFn> {
    const name: DomainEvent = "embedding:changed";
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
    const name: AppEvent = "menu:set-theme";
    const listener = decorate((e: { payload: string }) =>
      callback(e.payload)
    ).with(logged, { logger, name });
    return listen<string>(name, listener);
  }

  getHtmlReaderView(): Promise<string> {
    return decorate(invoke).with(logged, {
      logger,
      name: "get_html_reader_view",
    })<string>("get_html_reader_view");
  }

  setHtmlReaderView(view: HtmlReaderView): Promise<void> {
    return decorate(invoke).with(logged, {
      logger,
      name: "set_html_reader_view",
    })<void>("set_html_reader_view", { value: view });
  }

  onHtmlReaderView(
    callback: (view: HtmlReaderView) => void
  ): Promise<UnlistenFn> {
    const name: AppEvent = "menu:html-reader-view";
    const listener = decorate(callback).with(logged, { logger, name });
    return listen(name, (e) => listener(e.payload as HtmlReaderView));
  }

  onHtmlReaderViewChangedFromSettings(
    callback: (view: HtmlReaderView) => void
  ): Promise<UnlistenFn> {
    const name: AppEvent = "settings:html-reader-view";
    const listener = decorate(callback).with(logged, { logger, name });
    return listen(name, (e) => listener(e.payload as HtmlReaderView));
  }

  onShowSidebar(callback: (show: boolean) => void): Promise<UnlistenFn> {
    const name: AppEvent = "menu:show-sidebar";
    const listener = decorate((e: { payload: boolean }) =>
      callback(e.payload)
    ).with(logged, { logger, name });
    return listen<boolean>(name, listener);
  }

  onLoaderClosed(
    callback: (data: {
      loaderUrl: string;
      pageUrl: string;
      samePage: boolean;
      html: string;
      mimeType: string;
    }) => void
  ): Promise<UnlistenFn> {
    const name: AppEvent = "loader:closed";
    const listener = decorate(
      (e: {
        payload: {
          loaderUrl: string;
          pageUrl: string;
          samePage: boolean;
          html: string;
          mimeType: string;
        };
      }) => callback(e.payload)
    ).with(logged, { logger, name });
    return listen<{
      loaderUrl: string;
      pageUrl: string;
      samePage: boolean;
      html: string;
      mimeType: string;
    }>(name, listener);
  }

  onLoaderPageLoaded(
    callback: (data: { url: string; html: string; mimeType: string }) => void
  ): Promise<UnlistenFn> {
    const name: AppEvent = "loader:page-loaded";
    const listener = decorate(
      (e: { payload: { url: string; html: string; mimeType: string } }) =>
        callback(e.payload)
    ).with(logged, { logger, name });
    return listen<{ url: string; html: string; mimeType: string }>(
      name,
      listener
    );
  }

  onFind(callback: () => void): Promise<UnlistenFn> {
    const name: AppEvent = "menu:find";
    const listener = decorate(callback).with(logged, { logger, name });
    return listen(name, listener);
  }

  onClearHistory(callback: () => void): Promise<UnlistenFn> {
    const name: AppEvent = "menu:clear-history";
    const listener = decorate(callback).with(logged, { logger, name });
    return listen(name, listener);
  }

  clearWebviewHistory(): Promise<void> {
    const name = "clear_webview_history";
    const invoker = decorate(invoke).with(logged, { logger, name });
    return invoker<void>(name);
  }

  setFeedPinned(id: string, pinned: boolean): Promise<void> {
    return decorate(invoke).with(logged, {
      logger,
      name: "set_feed_pinned",
    })<void>("set_feed_pinned", { id, pinned });
  }

  setTopicPinned(id: string, pinned: boolean): Promise<void> {
    return decorate(invoke).with(logged, {
      logger,
      name: "set_topic_pinned",
    })<void>("set_topic_pinned", { id, pinned });
  }

  getInlinePdfReader(): Promise<boolean> {
    return decorate(invoke).with(logged, {
      logger,
      name: "get_inline_pdf_reader",
    })<boolean>("get_inline_pdf_reader");
  }

  setInlinePdfReader(value: boolean): Promise<void> {
    return decorate(invoke).with(logged, {
      logger,
      name: "set_inline_pdf_reader",
    })<void>("set_inline_pdf_reader", { value });
  }

  onInlinePdfReaderChanged(
    callback: (value: boolean) => void
  ): Promise<UnlistenFn> {
    const name: AppEvent = "settings:inline-pdf-reader";
    const listener = decorate(callback).with(logged, { logger, name });
    return listen(name, (e) => listener(e.payload as boolean));
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

  onAllowedDomainsChanged(
    callback: (domains: string[]) => void
  ): Promise<UnlistenFn> {
    const name: AppEvent = "settings:allowed-domains";
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
    const name: DomainEvent = "embedding:shutdown";
    const listener = decorate(callback).with(logged, { logger, name });
    return listen(name, listener);
  }

  onThemeChangedFromSettings(
    callback: (theme: string) => void
  ): Promise<UnlistenFn> {
    const name: AppEvent = "settings:theme";
    const listener = decorate((e: { payload: string }) =>
      callback(e.payload)
    ).with(logged, { logger, name });
    return listen<string>(name, listener);
  }

  onHtmlReaderFontFamilyChanged(
    callback: (value: string) => void
  ): Promise<UnlistenFn> {
    const name: AppEvent = "settings:html-reader-font-family";
    const listener = decorate((e: { payload: string }) =>
      callback(e.payload)
    ).with(logged, { logger, name });
    return listen<string>(name, listener);
  }

  onHtmlReaderFontSizeChanged(
    callback: (value: number) => void
  ): Promise<UnlistenFn> {
    const name: AppEvent = "settings:html-reader-font-size";
    const listener = decorate((e: { payload: number }) =>
      callback(e.payload)
    ).with(logged, { logger, name });
    return listen<number>(name, listener);
  }

  onHtmlReaderLineHeightChanged(
    callback: (value: number) => void
  ): Promise<UnlistenFn> {
    const name: AppEvent = "settings:html-reader-line-height";
    const listener = decorate((e: { payload: number }) =>
      callback(e.payload)
    ).with(logged, { logger, name });
    return listen<number>(name, listener);
  }

  onRefreshOnStartupChanged(
    callback: (value: boolean) => void
  ): Promise<UnlistenFn> {
    const name: AppEvent = "settings:refresh-on-startup";
    const listener = decorate((e: { payload: boolean }) =>
      callback(e.payload)
    ).with(logged, { logger, name });
    return listen<boolean>(name, listener);
  }
}
