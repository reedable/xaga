// Derived types. The domain model lives in ./domain; this module re-exports it and adds the
// cache/query (derived) and UI/navigation shapes built on top of it.

export * from "./domain";

import type { AppError } from "./platform/errors";
import {
  isManagedEntity,
  type Entry,
  type Annotation,
  type Topic,
  type Feed,
  type Site,
  type ManagedEntity,
  type Countable,
} from "./domain";

// ── Utils ────────────────────────────────────────────────────────────────────

export type AbortError = DOMException;

export function isAbortError(value?: unknown): value is AbortError {
  return value instanceof DOMException && value?.name === "AbortError";
}

// ── Query results (derived) ────────────────────────────────────────────────────

/** A managed entity carrying its similarity score from an embedding query. */
export type Ranked<T extends ManagedEntity> = T & { similarity: number };

export type RankedEntity = Ranked<Entry> | Ranked<Topic> | Ranked<Feed>;

export function isRanked<T extends ManagedEntity>(
  o: T | null | undefined
): o is Ranked<T> {
  return !!o && typeof (o as Ranked<T>).similarity === "number";
}

// ── Entities list (derived) ────────────────────────────────────────────────────────

/** An entity in the list returned by listAllEntities: an entry (any state) or an annotation. */
export type ListEntity = Entry | PartialAnnotation;

export type AllEntitiesCounts = {
  entryCount: number;
  unreadCount: number;
};

// ── Cache projection (derived) ──────────────────────────────────────────────────

/** An annotation record as stored in the embeddings DB — no text[]/occurrence[]/content. */
export type PartialAnnotation = Pick<
  Annotation,
  "id" | "title" | "createdAt" | "reference" | "color"
> & { url: string };

// An annotation in list form (saved list or embedding-query result). Identified by reference +
// color + url; a full `Annotation` carries no url, which is what separates the two forms.
export function isPartialAnnotation(
  entity?: unknown
): entity is PartialAnnotation {
  return (
    isManagedEntity(entity) &&
    "color" in entity &&
    typeof (entity as PartialAnnotation).color === "string" &&
    "reference" in entity &&
    typeof (entity as PartialAnnotation).reference === "string" &&
    "url" in entity &&
    typeof (entity as PartialAnnotation).url === "string"
  );
}

// ── Presentation ──────────────────────────────────────────────────────────────

const FILTERS = [
  "recommended",
  "all",
  "unread",
  "bookmarked",
  "saved",
  "archived",
] as const;

export type Filter = (typeof FILTERS)[number];

export function isFilter(value?: unknown): value is Filter {
  return (
    !!value &&
    typeof value === "string" &&
    (FILTERS as readonly string[]).includes(value)
  );
}

const TOPIC_SORTS: string[] = ["relevance", "timestamp"] as const;

export type TopicSort = (typeof TOPIC_SORTS)[number];

export function isTopicSort(value?: unknown): value is TopicSort {
  return !!value && typeof value === "string" && TOPIC_SORTS.includes(value);
}

const HTML_READER_VIEWS: string[] = ["reader", "summary"] as const;

export type HtmlReaderView = (typeof HTML_READER_VIEWS)[number];

export function isHtmlReaderView(
  value?: string | null
): value is HtmlReaderView {
  return !!value && HTML_READER_VIEWS.includes(value);
}

/** A saved filter view: a count-bearing managed entity that carries a Filter (e.g. Unread (42)). */
export type Shortcut = ManagedEntity & Countable & { filter: Filter };

export function isShortcut(entity?: unknown): entity is Shortcut {
  return (
    isManagedEntity(entity) &&
    "filter" in entity &&
    isFilter((entity as Shortcut).filter)
  );
}

// ── Import ───────────────────────────────────────────────────────────────────

/** Feed import result. Failed entries are counted; raw details stay in native logs. */
export type ImportResult<T> = {
  added: T[];
  skipped: string[];
  failed: number;
};

/**
 * Payload of a native-triggered import/export completion event. Import/export
 * are orchestrated on the native side (no webview command — see
 * docs/architecture/security/command_surface.md); the outcome is delivered by event, with
 * `result` present on success and `error` present on failure.
 */
export type ImportOutcome<T> = {
  result: ImportResult<T> | null;
  error: AppError | null;
};

export type BookmarkImportOutcome = {
  result: { added: string[]; updated: string[]; failed: number } | null;
  error: AppError | null;
};

export type ExportOutcome = {
  error: AppError | null;
};

// ── Navigation ───────────────────────────────────────────────────────────────

export type NavigationState = {
  /** Undefined: another scope; []: all Entries in Tags browsing. */
  tagIds?: string[];
  /** Selection-only identity update; list membership refresh waits for explicit navigation. */
  tagSelectionRevision?: number;
  topicId?: Topic["id"];
  feedId?: Feed["id"];
  siteHostname?: Site["id"];
  shortcutId?: Shortcut["id"];
  articleScopeId?: Entry["id"];
  entryId?: Entry["id"];
  annotationId?: Annotation["id"];
  url?: Entry["url"];
  mimeType?: string;
};

// ── Article diagnostics ──────────────────────────────────────────────────────

export type ArticleDiagnosticOutcome =
  | "started"
  | "succeeded"
  | "failed"
  | "warning"
  | "skipped"
  | "cancelled";

export type ArticleDiagnosticStep =
  | "attempt"
  | "lookup"
  | "read"
  | "fetch"
  | "parse"
  | "readability"
  | "images"
  | "sanitize"
  | "markdown"
  | "create"
  | "save"
  | "readback"
  | "pdf_decode"
  | "render"
  | "extract";

export type ArticleDiagnostic = {
  id: string;
  startedAtMs: number;
  mode: string;
  outcome: ArticleDiagnosticOutcome | null;
  events: {
    elapsedMs: number;
    source: string;
    step: string;
    outcome: ArticleDiagnosticOutcome;
    code: string;
    detail: string;
    download?: {
      loaderLabel: string;
      sourceUrl: string;
      currentUrl: string | null;
      downloadUrl: string;
      destination: string | null;
      success: boolean | null;
    };
  }[];
  droppedEvents: number;
};

export type FeedRefreshFailure = {
  id: string;
  title: string;
  code: AppError["code"];
  message: string;
};
export type FeedRefreshOutcome = {
  feeds: Feed[];
  failures: FeedRefreshFailure[];
};
