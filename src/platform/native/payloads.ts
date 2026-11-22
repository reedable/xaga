import { ERROR_CODE, isAppError, type AppError } from "../errors";
import { z } from "zod";
import { SIDEBAR_SORT_MODES } from "../../sidebarSort";
import type { PinNode } from "../../pins";
import type {
  ArticleDiagnostic,
  ManagedEntity,
  Feed,
  Site,
  Entry,
  Topic,
  Annotation,
  ListEntity,
  PartialAnnotation,
  Article,
} from "../../types";

const ArticleDiagnosticOutcomeSchema = z.enum([
  "started",
  "succeeded",
  "failed",
  "warning",
  "skipped",
  "cancelled",
]);

export const ArticleDiagnosticSchema: z.ZodType<ArticleDiagnostic> = z.object({
  id: z.string(),
  startedAtMs: z.number(),
  mode: z.string(),
  outcome: ArticleDiagnosticOutcomeSchema.nullable(),
  events: z.array(
    z.object({
      elapsedMs: z.number(),
      source: z.string(),
      step: z.string(),
      outcome: ArticleDiagnosticOutcomeSchema,
      code: z.string(),
      detail: z.string(),
      download: z
        .object({
          loaderLabel: z.string(),
          sourceUrl: z.string(),
          currentUrl: z.string().nullable(),
          downloadUrl: z.string(),
          destination: z.string().nullable(),
          success: z.boolean().nullable(),
        })
        .optional(),
    })
  ),
  droppedEvents: z.number(),
});

const ManagedEntitySchema = z.object({
  id: z.string(),
  title: z.string(),
  created_at: z.string(),
  updated_at: z.string().nullable(),
});

const toManagedEntity = (
  data: z.infer<typeof ManagedEntitySchema>
): ManagedEntity => ({
  id: data.id,
  title: data.title,
  createdAt: data.created_at,
  updatedAt: data.updated_at ?? undefined,
});

export const FeedSchema = ManagedEntitySchema.extend({
  url: z.string(),
  entry_count: z.number(),
  unread_count: z.number(),
  is_pinned: z.boolean(),
});

const ReferenceSchema = z.object({
  __kind: z.enum(["entry", "annotation", "note"]),
  id: z.string(),
  articleId: z.string().optional(),
});

export const toFeed = (data: z.infer<typeof FeedSchema>): Feed => {
  return {
    ...toManagedEntity(data),
    __kind: "feed",
    url: data.url,
    entryCount: data.entry_count,
    unreadCount: data.unread_count,
    isPinned: data.is_pinned,
    members: [],
  };
};

export const EntrySchema = ManagedEntitySchema.extend({
  url: z.string(),
  published_at: z.string().nullable(),
  description: z.string().nullable(),
  is_unread: z.boolean(),
  is_pinned: z.boolean().optional(),
  color: z.string().nullable().optional(),
});

// read_entry_images returns a positional array aligned with the requested refs:
// a resolved `data:` URI, or null where the reference did not resolve.
export const ReadEntryImagesSchema = z.array(z.string().nullable());

export const AllEntitiesCountsSchema = z.object({
  entry_count: z.number(),
  unread_count: z.number(),
});

const SiteSchema = z.object({
  hostname: z.string(),
  entry_count: z.number(),
  unread_count: z.number(),
});

const toSite = (data: z.infer<typeof SiteSchema>): Site => ({
  __kind: "site",
  id: data.hostname,
  title: data.hostname,
  entryCount: data.entry_count,
  unreadCount: data.unread_count,
});

export const toEntry = (data: z.infer<typeof EntrySchema>): Entry => {
  return {
    ...toManagedEntity(data),
    __kind: "entry",
    url: data.url,
    publishedAt: data.published_at ?? undefined,
    description: data.description ?? undefined,
    isUnread: data.is_unread,
    isPinned: data.is_pinned ?? false,
    color: data.color ?? undefined,
  };
};

export const TopicSchema = ManagedEntitySchema.extend({
  description: z.string().nullable().optional(),
  threshold: z.number().nullable().optional(),
  entry_count: z.number(),
  unread_count: z.number(),
  is_pinned: z.boolean(),
  members: z.array(ReferenceSchema).optional(),
});

export const toTopic = (data: z.infer<typeof TopicSchema>): Topic => ({
  ...toManagedEntity(data),
  __kind: "topic",
  description: data.description ?? undefined,
  threshold: data.threshold ?? undefined,
  entryCount: data.entry_count,
  unreadCount: data.unread_count,
  isPinned: data.is_pinned,
  members: data.members ?? [],
});

export const toTopicEntity = (entity: Entry | Annotation) =>
  entity.__kind === "annotation"
    ? { __kind: "annotation", id: entity.id, articleId: entity.reference }
    : { __kind: "entry", id: entity.id };

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

export const ListEntitySchema = z.object({
  id: z.string(),
  title: z.string(),
  created_at: z.string(),
  url: z.string(),
  is_unread: z.boolean(),
  is_pinned: z.boolean().optional(),
  reference: z.string().optional(),
  color: z.string().optional(),
});

export const toListEntity = (
  data: z.infer<typeof ListEntitySchema>
): ListEntity =>
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
        isPinned: data.is_pinned ?? false,
        color: data.color || undefined,
      };

export const PartialAnnotationSchema = z.object({
  id: z.string(),
  reference: z.string(),
  title: z.string(),
  color: z.string(),
  created_at: z.string(),
  url: z.string(),
});

export const toPartialAnnotation = (
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

export const ArticleSchema = EntrySchema.extend({
  content: z.string(),
  annotations: z.array(AnnotationSchema).default([]),
  favicon: z.string().nullable().optional(),
});

export const toArticle = (data: z.infer<typeof ArticleSchema>): Article => ({
  ...toEntry(data),
  content: data.content,
  annotations: data.annotations.map(toAnnotation),
  favicon: data.favicon ?? undefined,
});

const EmbeddedSchema = z.object({
  similarity: z.number(),
});

function toEmbedded(data: z.infer<typeof EmbeddedSchema>): {
  similarity: number;
} {
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
const EmbeddedEntitySchema = ManagedEntitySchema.extend(EmbeddedSchema.shape);
// Detection order (each branch identified by brand AND shape together):
// 1. Annotation: `kind === "annotation"`; shape guaranteed by the schema's required `reference`
//    (entries/feeds/topics never serialize it, so coloring an entry can't collide here)
// 2. Feed: `kind === "feed"` (feed/entry share shape — only the brand separates them)
// 3. Topic: `kind === "topic"` (a Topic has no url/is_unread/content — only the brand is reliable)
// 4. Entry: has `is_unread` (topics don't — they have no url/is_unread fields)
// 5. Generic ManagedEntity: fallback
export const AnyEmbeddedEntitySchema = EmbeddedAnnotationResultSchema.or(
  EmbeddedFeedSchema
)
  .or(EmbeddedEntrySchema)
  .or(EmbeddedTopicSchema)
  .or(EmbeddedEntitySchema);

export const toAnyEmbeddedEntity = (
  data: z.infer<typeof AnyEmbeddedEntitySchema>
) => {
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
    ...toManagedEntity(data as z.infer<typeof EmbeddedEntitySchema>),
    ...toEmbedded(data as z.infer<typeof EmbeddedSchema>),
  };
};

export const entryTagChangeSchema = z.object({
  entryId: z.string(),
  tagId: z.uuid({ version: "v4" }),
  assigned: z.boolean(),
});
export const annotationTagChangeSchema = entryTagChangeSchema.extend({
  annotationId: z.string(),
});
export const tagSchema = z.object({
  id: z.uuid({ version: "v4" }),
  name: z.string().min(1),
});
export const tagSnapshotSchema = z.object({
  tags: z.array(tagSchema),
  renaming: z.array(z.uuid({ version: "v4" })),
  deleting: z.array(z.uuid({ version: "v4" })),
  merging: z.array(
    z.object({
      sourceId: z.uuid({ version: "v4" }),
      destinationId: z.uuid({ version: "v4" }),
      survivorId: z.uuid({ version: "v4" }),
      destinationName: z.string(),
    })
  ),
});

const target = z.union([
  z
    .object({ type: z.enum(["entry", "feed", "topic"]), id: z.string() })
    .strict(),
  z.object({ type: z.literal("site"), hostname: z.string() }).strict(),
]);
const node: z.ZodType<PinNode> = z.lazy(() =>
  z.union([
    z
      .object({
        type: z.literal("group"),
        id: z.string(),
        title: z.string(),
        children: z.array(node),
      })
      .strict(),
    target,
  ])
);

export const PinsSchema = z.object({ pins: z.array(node) }).strict();

export const AnnotationReplacementSchema = z.object({
  oldId: z.string(),
  entity: ListEntitySchema.transform(toListEntity),
});

export const SidebarSortSchema = z.enum(SIDEBAR_SORT_MODES);
export const TagIdsSchema = z.array(z.uuid({ version: "v4" }));
export const CountSchema = z.number().int().nonnegative();

export const FeedRefreshOutcomeSchema = z.object({
  feeds: z.array(FeedSchema.transform(toFeed)),
  failures: z.array(
    z.object({
      id: z.string(),
      title: z.string(),
      code: z.enum(ERROR_CODE),
      message: z.string(),
    })
  ),
});
export const DiagnosticIdSchema = z.string();
export const EntriesSchema = z.array(EntrySchema.transform(toEntry));
export const ListEntitiesSchema = z.array(
  ListEntitySchema.transform(toListEntity)
);
export const TagsSchema = z.array(tagSchema);

export const SitesSchema = z.array(SiteSchema.transform(toSite));

export function isNativeAppError(error: unknown): error is AppError {
  return (
    isAppError(error) &&
    typeof error.message === "string" &&
    typeof error.detail === "string" &&
    (error.diagnosticId === undefined || typeof error.diagnosticId === "string")
  );
}

export const BookmarkResultSchema = z.object({
  entry: EntrySchema.transform(toEntry),
  indexUpdated: z.boolean(),
});
