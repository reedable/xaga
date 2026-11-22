import {
  isNativeAppError,
  ArticleDiagnosticSchema,
  FeedRefreshOutcomeSchema,
  DiagnosticIdSchema,
  EntriesSchema,
  ListEntitiesSchema,
  TagsSchema,
  FeedSchema,
  toFeed,
  EntrySchema,
  BookmarkResultSchema,
  ReadEntryImagesSchema,
  AllEntitiesCountsSchema,
  SitesSchema,
  toEntry,
  TopicSchema,
  toTopic,
  toTopicEntity,
  ListEntitySchema,
  toListEntity,
  PartialAnnotationSchema,
  toPartialAnnotation,
  ArticleSchema,
  toArticle,
  AnyEmbeddedEntitySchema,
  toAnyEmbeddedEntity,
  entryTagChangeSchema,
  annotationTagChangeSchema,
  tagSchema,
  tagSnapshotSchema,
  PinsSchema,
  AnnotationReplacementSchema,
  SidebarSortSchema,
  TagIdsSchema,
  CountSchema,
} from "./native/payloads";
import type { FetchSettings, FetchSettingChange } from "./fetchSettings";
import { type SidebarSortMode } from "../sidebarSort";
import type {
  AnnotationTagChange,
  EntryTagChange,
  Tag,
  TagSnapshot,
} from "../domain";
import type {
  PinTarget,
  PinNode,
  PinEdit,
  PinChange,
  ShortcutMenuItem,
} from "../pins";
import type { ImageRecovery } from "../text/imageReferences";
import type { AppError as SerializedAppError } from "./errors";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";

import type { AppEvent, DomainEvent } from "./events";
import type {
  ArticleDiagnostic,
  ArticleDiagnosticOutcome,
  ArticleDiagnosticStep,
  Annotation,
  PartialAnnotation,
  Article,
  Ranked,
  RankedEntity,
  Entry,
  Feed,
  FeedRefreshOutcome,
  Filter,
  HtmlReaderView,
  ImportOutcome,
  BookmarkImportOutcome,
  ExportOutcome,
  ManagedEntity,
  ListEntity,
  AllEntitiesCounts,
  Site,
  Topic,
  TopicSort,
} from "../types";
import { abortable, decorate, logged } from "../utils/Decorator";
import Logger from "../utils/Logger";
import type { CreateTopicInput, Platform } from "./Platform";

const logger = Logger.getLogger("TauriPlatform");

listen<string>("loader:will-unload" satisfies AppEvent, (e) => {
  console.log("[diagnostic] loader:will-unload", e.payload);
});

export class TauriPlatform implements Platform {
  async onError(
    callback: (error: SerializedAppError) => void
  ): Promise<UnlistenFn> {
    const seen = new Set<string>();
    const deliver = (error: unknown) => {
      if (!isNativeAppError(error)) return;
      if (error.diagnosticId) {
        if (seen.has(error.diagnosticId)) return;
        seen.add(error.diagnosticId);
        if (seen.size > 64) seen.delete(seen.values().next().value!);
      }
      callback(error);
    };
    const name: AppEvent = "app:error";
    const unlisten = await listen<SerializedAppError>(
      name,
      (event) => deliver(event.payload),
      { target: { kind: "WebviewWindow", label: getCurrentWindow().label } }
    );
    try {
      const pending = await invoke<SerializedAppError[]>("take_ui_errors");
      pending.forEach(deliver);
    } catch (error) {
      unlisten();
      throw error;
    }
    return unlisten;
  }

  // Window --------------------------------------------------------------------

  async onWindowCloseRequested(
    callback: () => Promise<boolean>
  ): Promise<UnlistenFn> {
    return getCurrentWindow().onCloseRequested(async (event) => {
      if (!(await callback())) event.preventDefault();
    });
  }

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

  async embeddingStartupState(): Promise<
    import("./Platform").EmbeddingStartupState
  > {
    const name = "embedding_startup_state";
    const invoker = decorate(invoke).with(logged, { logger, name });
    return invoker<import("./Platform").EmbeddingStartupState>(name);
  }

  async quitApp(): Promise<void> {
    await invoke("quit_app");
  }

  async getLibraryStatus(): Promise<import("./Platform").LibraryStatus> {
    return invoke("get_library_status");
  }
  async continueLibraryReadOnly(): Promise<void> {
    return invoke("continue_library_read_only");
  }
  async getLibraryUpgrade(): Promise<import("./Platform").LibraryUpgrade> {
    return invoke("get_library_upgrade");
  }
  async startLibraryUpgrade(
    path: string
  ): Promise<import("./Platform").LibraryUpgrade> {
    return invoke("start_library_upgrade", { path });
  }
  async restartAfterLibraryUpgrade(): Promise<string | null> {
    return invoke("restart_after_library_upgrade");
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
      .filter((e) => !e.error) // FIXME Silently dropping invalid entities
      .map((e) => e.data);
  }

  async addFeed(signal: AbortSignal, feedUrl: string): Promise<Feed> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "add_feed" });
    const result = await invoker("add_feed", { url: feedUrl });
    return FeedSchema.transform(toFeed).parse(result);
  }

  async refreshAllFeeds(signal: AbortSignal): Promise<FeedRefreshOutcome> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "refresh_all_feeds" });
    const result = await invoker("refresh_all_feeds", {});
    return FeedRefreshOutcomeSchema.parse(result);
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

  // Sites ---------------------------------------------------------------------

  async listSites(signal: AbortSignal): Promise<Site[]> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "list_sites" });
    return SitesSchema.parse(await invoker("list_sites"));
  }

  async listSiteEntries(
    signal: AbortSignal,
    hostname: string,
    entityFilter: Filter = "unread"
  ): Promise<Entry[]> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "list_site_entries" });
    const results = await invoker<unknown[]>("list_site_entries", {
      hostname,
      entityFilter,
    });
    return results.map((entry) => EntrySchema.transform(toEntry).parse(entry));
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
      entityFilter: filter,
    });

    return results
      .map((e) => EntrySchema.transform(toEntry).safeParse(e))
      .filter((e) => !e.error) // FIXME silently dropping invalid entities
      .map((e) => e.data);
  }

  async listAllEntities(
    signal: AbortSignal,
    entityFilter: Filter = "unread"
  ): Promise<ListEntity[]> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "list_all_entities" });

    const results = await invoker<unknown[]>("list_all_entities", {
      entityFilter,
    });

    return results
      .map((e) => ListEntitySchema.transform(toListEntity).safeParse(e))
      .filter((e) => !e.error) // FIXME silently dropping invalid entities
      .map((e) => e.data);
  }

  async getAllEntitiesCounts(): Promise<AllEntitiesCounts | null> {
    const name = "get_all_entities_counts";
    const invoker = decorate(invoke).with(logged, { logger, name });
    const result = await invoker<unknown>(name);
    if (result === null) return null;
    const counts = AllEntitiesCountsSchema.parse(result);
    return {
      entryCount: counts.entry_count,
      unreadCount: counts.unread_count,
    };
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

  async createArticleDiagnostic(mode: string): Promise<string> {
    const name = "create_article_diagnostic";
    const invoker = decorate(invoke).with(logged, { logger, name });

    const result = await invoker<unknown>(name, { mode });
    return DiagnosticIdSchema.parse(result);
  }

  addArticleDiagnosticEvent(
    id: string,
    step: ArticleDiagnosticStep,
    outcome: ArticleDiagnosticOutcome,
    code: string
  ): Promise<void> {
    const name = "add_article_diagnostic_event";
    const invoker = decorate(invoke).with(logged, { logger, name });

    return invoker<void>(name, { id, step, outcome, code });
  }

  async getArticleDiagnostic(id: string): Promise<ArticleDiagnostic | null> {
    const name = "get_article_diagnostic";
    const invoker = decorate(invoke).with(logged, { logger, name });

    const result = await invoker<unknown>(name, { id });
    return ArticleDiagnosticSchema.nullable().parse(result);
  }

  async openStoredPdf(url: string): Promise<void> {
    await invoke("open_stored_pdf", { url });
  }

  async readStoredPdf(
    signal: AbortSignal,
    url: string
  ): Promise<{ pdfBase64?: string | null; pdfEntryId?: string } | null> {
    return decorate(invoke).with(abortable, { signal })<{
      pdfBase64?: string | null;
      pdfEntryId?: string;
    } | null>("read_stored_pdf", { url });
  }

  async loadArticle(
    signal: AbortSignal,
    url: string,
    keepOpen?: boolean,
    diagnosticId?: string
  ): Promise<{
    mimeType: string;
    html: string;
    pdfBase64?: string | null;
    pdfEntryId?: string;
    favicon?: string | null;
    faviconData?: string | null;
  }> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "load_article" });

    const cancel = () => {
      if (diagnosticId)
        void invoke("cancel_article_load", { diagnosticId }).catch(() => {});
    };
    signal.throwIfAborted();
    signal.addEventListener("abort", cancel, { once: true });
    try {
      return await invoker<{
        mimeType: string;
        html: string;
        pdfBase64?: string | null;
        pdfEntryId?: string;
        favicon?: string | null;
        faviconData?: string | null;
      }>("load_article", {
        url,
        keepOpen: keepOpen ?? false,
        diagnosticId,
      });
    } finally {
      signal.removeEventListener("abort", cancel);
    }
  }

  async findArticle(
    signal: AbortSignal,
    url: string,
    diagnosticId?: string
  ): Promise<Article | null> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "find_article" });

    const result = await invoker<unknown | null>("find_article", {
      diagnosticId,
      url,
    });
    if (result === null || result === undefined) return null;
    return ArticleSchema.transform(toArticle).parse(result);
  }

  async saveArticle(
    signal: AbortSignal,
    article: Article,
    imageData: Record<string, string> = {},
    diagnosticId?: string
  ): Promise<Article> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "save_article" });

    const result = await invoker<unknown>("save_article", {
      diagnosticId,
      id: article.id,
      url: article.url,
      title: article.title,
      publishedAt: article.publishedAt,
      updatedAt: article.updatedAt,
      summary: article.description,
      favicon: article.favicon ?? null,
      markdown: article.content,
      imageData,
    });

    return ArticleSchema.transform(toArticle).parse(result);
  }

  async setEntryPinned(id: string, pinned: boolean): Promise<Entry> {
    const result = await invoke("set_entry_pinned", { id, pinned });
    return EntrySchema.transform(toEntry).parse(result);
  }

  async listPinnedEntries(signal: AbortSignal): Promise<Entry[]> {
    const result = await decorate(invoke).with(abortable, { signal })<unknown>(
      "list_pinned_entries"
    );
    return EntriesSchema.parse(result);
  }

  async readArticleScope(signal: AbortSignal, id: string): Promise<Article> {
    const result = await decorate(invoke).with(abortable, { signal })<unknown>(
      "read_article_scope",
      { id }
    );
    return ArticleSchema.transform(toArticle).parse(result);
  }

  onEntryPinChanged(callback: (entry: Entry) => void): Promise<UnlistenFn> {
    const name: DomainEvent = "entry-pin:changed";
    return listen<unknown>(name, (event) => {
      const result = EntrySchema.transform(toEntry).safeParse(event.payload);
      if (result.success) callback(result.data);
    });
  }

  async readEntry(signal: AbortSignal, id: string): Promise<Entry> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "read_entry" });

    const result = await invoker<unknown>("read_entry", { id });
    return EntrySchema.transform(toEntry).parse(result);
  }

  async readArticle(
    signal: AbortSignal,
    id: string,
    diagnosticId?: string
  ): Promise<Article> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "read_article" });

    const result = await invoker<unknown>("read_article", { diagnosticId, id });
    return ArticleSchema.transform(toArticle).parse(result);
  }

  async recoverEntryImages(
    signal: AbortSignal,
    id: string,
    recovery: ImageRecovery,
    diagnosticId?: string
  ): Promise<Article> {
    const invoker = decorate(invoke).with(abortable, { signal });
    const cancel = () => {
      if (diagnosticId)
        void invoke("cancel_article_load", { diagnosticId }).catch(() => {});
    };
    signal.throwIfAborted();
    signal.addEventListener("abort", cancel, { once: true });
    try {
      const result = await invoker<unknown>("recover_entry_images", {
        id,
        recovery,
        diagnosticId,
      });
      return ArticleSchema.transform(toArticle).parse(result);
    } finally {
      signal.removeEventListener("abort", cancel);
    }
  }

  async readEntryImages(
    signal: AbortSignal,
    id: string,
    refs: string[]
  ): Promise<(string | null)[]> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "read_entry_images" });

    const result = await invoker<(string | null)[]>("read_entry_images", {
      id,
      refs,
    });
    return ReadEntryImagesSchema.parse(result);
  }

  // `__kind` is minted on the way back by ArticleSchema.transform(toArticle)
  // below, so it's omitted from the input. Tactical fix — see TECH_DEBT.md
  // "Frontend type branding (`__kind`)".
  async createArticle(
    signal: AbortSignal,
    article: Omit<Article, "id" | "createdAt" | "__kind" | "isPinned">,
    imageData: Record<string, string> = {},
    diagnosticId?: string
  ): Promise<Article> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "create_article" });

    const result = await invoker<unknown>("create_article", {
      diagnosticId,
      url: article.url,
      title: article.title,
      publishedAt: article.publishedAt,
      updatedAt: article.updatedAt,
      summary: article.description,
      favicon: article.favicon ?? null,
      markdown: article.content,
      imageData,
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
    topic: CreateTopicInput
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

  async createTopicWithMember(
    signal: AbortSignal,
    topic: CreateTopicInput,
    member: Entry | Annotation
  ): Promise<Topic> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "create_topic_with_member" });
    const result = await invoker<unknown>("create_topic_with_member", {
      title: topic.title,
      description: topic.description ?? null,
      threshold: topic.threshold ?? null,
      entity: toTopicEntity(member),
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

  async listTopicEntities(
    signal: AbortSignal,
    id: string,
    filter: Filter,
    sort: TopicSort = "relevance"
  ): Promise<ListEntity[]> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "list_topic_entities" });
    const results = await invoker<unknown[]>("list_topic_entities", {
      id,
      entityFilter: filter,
      sort,
    });
    return results
      .map((e) => ListEntitySchema.transform(toListEntity).safeParse(e))
      .filter((e) => !e.error)
      .map((e) => e.data);
  }

  async addTopicEntity(
    signal: AbortSignal,
    topicId: string,
    entity: Entry | Annotation
  ): Promise<Topic> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "add_topic_entity" });
    const result = await invoker<unknown>("add_topic_entity", {
      topicId,
      entity: toTopicEntity(entity),
    });
    return TopicSchema.transform(toTopic).parse(result);
  }

  async removeTopicEntity(
    signal: AbortSignal,
    topicId: string,
    entityId: string
  ): Promise<Topic> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "remove_topic_entity" });
    const result = await invoker<unknown>("remove_topic_entity", {
      topicId,
      entityId,
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

  async rankTopicsRelatedToEntity(
    signal: AbortSignal,
    entityId: string,
    minSimilarity?: number
  ): Promise<RankedEntity[]> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "rank_topics_by_entity" });
    return invoker<RankedEntity[]>("rank_topics_by_entity", {
      entityId,
      minSimilarity: minSimilarity ?? null,
    });
  }

  // Bookmarks -----------------------------------------------------------------

  async addBookmark(
    signal: AbortSignal,
    id: string,
    color: string
  ): Promise<{ entry: Entry; indexUpdated: boolean }> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "add_bookmark" });

    const result = await invoker<unknown>("add_bookmark", { id, color });
    return BookmarkResultSchema.parse(result);
  }

  async removeBookmark(
    signal: AbortSignal,
    id: string
  ): Promise<{ entry: Entry; indexUpdated: boolean }> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "remove_bookmark" });

    const result = await invoker<unknown>("remove_bookmark", { id });
    return BookmarkResultSchema.parse(result);
  }

  // Embedding queries ---------------------------------------------------------

  async queryByEntity(
    signal: AbortSignal,
    id: string,
    entityFilter: Filter = "all",
    minSimilarity: number | null = null,
    maxSimilarity: number | null = null,
    limit: number | null = null
  ): Promise<Ranked<ManagedEntity>[]> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "search_by_entity" });

    const results = await invoker<unknown[]>("search_by_entity", {
      id,
      limit,
      minSimilarity,
      maxSimilarity,
      entityFilter,
    });

    return results
      .map((e) =>
        AnyEmbeddedEntitySchema.transform(toAnyEmbeddedEntity).safeParse(e)
      )
      .filter((e) => !e.error) // FIXME Dropping failed entities silently!
      .map((e) => e.data);
  }

  async queryByText(
    signal: AbortSignal,
    text: string,
    entityFilter?: Filter,
    minSimilarity?: number | null,
    maxSimilarity?: number | null,
    limit?: number | null
  ): Promise<Ranked<ManagedEntity>[]> {
    const invoker = decorate(invoke)
      .with(abortable, { signal })
      .with(logged, { logger, name: "search_by_text" });

    const results = await invoker<unknown[]>("search_by_text", {
      text,
      limit,
      minSimilarity,
      maxSimilarity,
      entityFilter,
    });

    return results
      .map((e) =>
        AnyEmbeddedEntitySchema.transform(toAnyEmbeddedEntity).safeParse(e)
      )
      .filter((e) => !e.error) // FIXME Dropping failed entities silently!
      .map((e) => e.data);
  }

  async searchSimilarTopics(
    signal: AbortSignal,
    title: string,
    description: string
  ): Promise<Topic[]> {
    const invokeSearch = decorate(invoke).with(abortable, { signal });
    const results = await invokeSearch<unknown[]>("search_similar_topics", {
      title,
      description,
    });
    return results.map((result) =>
      TopicSchema.transform(toTopic).parse(result)
    );
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

  onBookmarksImported(
    callback: (outcome: BookmarkImportOutcome) => void
  ): Promise<UnlistenFn> {
    return listen<BookmarkImportOutcome>("bookmarks:imported", (e) =>
      callback(e.payload)
    );
  }

  onBookmarksExported(
    callback: (outcome: ExportOutcome) => void
  ): Promise<UnlistenFn> {
    return listen<ExportOutcome>("bookmarks:exported", (e) =>
      callback(e.payload)
    );
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
    callback: (entity: ListEntity) => void
  ): Promise<UnlistenFn> {
    const name: DomainEvent = "annotation:changed";
    const listener = decorate((e) => {
      const result = ListEntitySchema.transform(toListEntity).safeParse(
        e.payload
      );
      if (result.success) callback(result.data);
    }).with(logged, { logger, name });

    return listen<unknown>(name, listener);
  }

  onAnnotationReplaced(
    callback: (replacement: { oldId: string; entity: ListEntity }) => void
  ): Promise<UnlistenFn> {
    const name: DomainEvent = "annotation:replaced";
    const listener = decorate((e) => {
      const payload = AnnotationReplacementSchema.safeParse(e.payload);
      if (payload.success) callback(payload.data);
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

  onFaviconChanged(callback: (article: Article) => void): Promise<UnlistenFn> {
    const name: DomainEvent = "favicon:changed";
    const listener = decorate((e) => {
      const result = ArticleSchema.transform(toArticle).safeParse(e.payload);
      if (result.success) callback(result.data);
    }).with(logged, { logger, name });
    return listen<unknown>(name, listener);
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

  async getFetchSettings(): Promise<FetchSettings> {
    const name = "get_fetch_settings";
    const invoker = decorate(invoke).with(logged, { logger, name });
    return invoker<FetchSettings>(name);
  }

  async setFetchSetting(change: FetchSettingChange): Promise<void> {
    const name = "set_fetch_setting";
    const invoker = decorate(invoke).with(logged, { logger, name });
    return invoker<void>(name, { change });
  }

  async getImportMaxBytes(): Promise<number> {
    const name = "get_import_max_bytes";
    const invoker = decorate(invoke).with(logged, { logger, name });
    return invoker<number>(name);
  }

  async setImportMaxBytes(value: number): Promise<void> {
    const name = "set_import_max_bytes";
    const invoker = decorate(invoke).with(logged, { logger, name });
    return invoker<void>(name, { value });
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

  async getSidebarSort(): Promise<SidebarSortMode> {
    return SidebarSortSchema.parse(await invoke("get_sidebar_sort"));
  }
  async setSidebarSort(value: SidebarSortMode): Promise<void> {
    return invoke("set_sidebar_sort", { value });
  }
  onSidebarSortChanged(
    callback: (value: SidebarSortMode) => void
  ): Promise<UnlistenFn> {
    const name: AppEvent = "settings:sidebar-sort";
    return listen<unknown>(name, (event) => {
      const result = SidebarSortSchema.safeParse(event.payload);
      if (result.success) callback(result.data);
      else logger.warn("Invalid sidebar sort event", result.error);
    });
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
      favicon?: string | null;
      faviconData?: string | null;
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
          favicon?: string | null;
          faviconData?: string | null;
        };
      }) => callback(e.payload)
    ).with(logged, { logger, name });
    return listen<{
      loaderUrl: string;
      pageUrl: string;
      samePage: boolean;
      html: string;
      mimeType: string;
      favicon?: string | null;
      faviconData?: string | null;
    }>(name, listener);
  }

  onLoaderPageLoaded(
    callback: (data: {
      url: string;
      html: string;
      mimeType: string;
      favicon?: string | null;
      faviconData?: string | null;
    }) => void
  ): Promise<UnlistenFn> {
    const name: AppEvent = "loader:page-loaded";
    const listener = decorate(
      (e: {
        payload: {
          url: string;
          html: string;
          mimeType: string;
          favicon?: string | null;
          faviconData?: string | null;
        };
      }) => callback(e.payload)
    ).with(logged, { logger, name });
    return listen<{
      url: string;
      html: string;
      mimeType: string;
      favicon?: string | null;
      faviconData?: string | null;
    }>(name, listener);
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

  setShortcutsMenu(items: ShortcutMenuItem[]): Promise<void> {
    return invoke("set_shortcuts_menu", { items });
  }

  onShortcutMenuSelected(callback: (key: string) => void): Promise<UnlistenFn> {
    const name: AppEvent = "menu:shortcut";
    return listen<string>(name, (event) => callback(event.payload));
  }

  clearWebviewHistory(): Promise<void> {
    const name = "clear_webview_history";
    const invoker = decorate(invoke).with(logged, { logger, name });
    return invoker<void>(name);
  }

  async readAnnotationTags(
    signal: AbortSignal,
    entryId: string,
    annotationId: string
  ): Promise<string[]> {
    const result = await invoke("read_annotation_tags", {
      entryId,
      annotationId,
    });
    signal.throwIfAborted();
    return TagIdsSchema.parse(result);
  }

  async setAnnotationTag(
    entryId: string,
    annotationId: string,
    tagId: string,
    assigned: boolean
  ): Promise<AnnotationTagChange> {
    return annotationTagChangeSchema.parse(
      await invoke("set_annotation_tag", {
        entryId,
        annotationId,
        tagId,
        assigned,
      })
    );
  }

  onAnnotationTagsChanged(
    callback: (change: AnnotationTagChange) => void
  ): Promise<UnlistenFn> {
    return listen<unknown>(
      "annotation-tags:changed" satisfies DomainEvent,
      (event) => callback(annotationTagChangeSchema.parse(event.payload))
    );
  }

  async readEntryTags(signal: AbortSignal, id: string): Promise<string[]> {
    const result = await invoke("read_entry_tags", { id });
    signal.throwIfAborted();
    return TagIdsSchema.parse(result);
  }

  async setEntryTag(
    id: string,
    tagId: string,
    assigned: boolean
  ): Promise<EntryTagChange> {
    return entryTagChangeSchema.parse(
      await invoke("set_entry_tag", { id, tagId, assigned })
    );
  }

  onEntryTagsChanged(
    callback: (change: EntryTagChange) => void
  ): Promise<UnlistenFn> {
    return listen<unknown>(
      "entry-tags:changed" satisfies DomainEvent,
      (event) => callback(entryTagChangeSchema.parse(event.payload))
    );
  }

  async listTagEntities(
    signal: AbortSignal,
    tagIds: string[],
    filter: Filter = "unread"
  ): Promise<ListEntity[]> {
    const result = await invoke("list_tag_entities", {
      tagIds,
      entityFilter: filter,
    });
    signal.throwIfAborted();
    return ListEntitiesSchema.parse(result);
  }

  async countTagUnread(signal: AbortSignal, tagIds: string[]): Promise<number> {
    const result = await invoke("count_tag_unread", { tagIds });
    signal.throwIfAborted();
    return CountSchema.parse(result);
  }

  async readTagStatus(signal: AbortSignal): Promise<TagSnapshot> {
    const result = await invoke("read_tag_status");
    signal.throwIfAborted();
    return tagSnapshotSchema.parse(result);
  }

  async renameTag(id: string, name: string): Promise<TagSnapshot> {
    return tagSnapshotSchema.parse(await invoke("rename_tag", { id, name }));
  }

  async deleteTag(id: string): Promise<TagSnapshot> {
    return tagSnapshotSchema.parse(await invoke("delete_tag", { id }));
  }

  async mergeTags(
    sourceId: string,
    destinationId: string
  ): Promise<TagSnapshot> {
    return tagSnapshotSchema.parse(
      await invoke("merge_tags", { sourceId, destinationId })
    );
  }

  onTagsChanged(
    callback: (snapshot: TagSnapshot) => void
  ): Promise<UnlistenFn> {
    return listen<unknown>("tags:changed" satisfies DomainEvent, (event) =>
      callback(tagSnapshotSchema.parse(event.payload))
    );
  }

  async listTags(signal: AbortSignal): Promise<Tag[]> {
    const result = await invoke("list_tags");
    signal.throwIfAborted();
    return TagsSchema.parse(result);
  }

  async createTag(name: string): Promise<Tag> {
    return tagSchema.parse(await invoke("create_tag", { name }));
  }

  onTagCreated(callback: (tag: Tag) => void): Promise<UnlistenFn> {
    const name: DomainEvent = "tag:created";
    return listen<unknown>(name, (event) =>
      callback(tagSchema.parse(event.payload))
    );
  }

  async listPins(signal: AbortSignal): Promise<PinNode[]> {
    const result = await invoke("list_pins");
    signal.throwIfAborted();
    return PinsSchema.parse(result).pins;
  }

  organizePins(edit: PinEdit): Promise<void> {
    return invoke("organize_pins", { edit });
  }

  setPin(target: PinTarget, pinned: boolean): Promise<void> {
    return invoke("set_pin", { target, pinned });
  }

  async onPinChanged(
    callback: (change: PinChange) => void
  ): Promise<UnlistenFn> {
    const stop = await listen<PinChange>("pin:changed", (event) =>
      callback(event.payload)
    );
    try {
      const organized = await listen<PinEdit>("pins:organized", (event) =>
        callback({ edit: event.payload })
      );
      return () => {
        stop();
        organized();
      };
    } catch (error) {
      stop();
      throw error;
    }
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

  getPublisherJavascriptAllowed(): Promise<boolean> {
    return invoke<boolean>("get_publisher_javascript_allowed");
  }

  setPublisherJavascriptAllowed(value: boolean): Promise<void> {
    return invoke<void>("set_publisher_javascript_allowed", { value });
  }

  setLoadRemoteImage(value: boolean): Promise<void> {
    return invoke<void>("set_load_remote_image", { value });
  }

  getLoadRemoteImage(): Promise<boolean> {
    return invoke<boolean>("get_load_remote_image");
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
