// Derived types. The domain model lives in ./domain; this module re-exports it and adds the
// cache/query (derived) and UI/navigation shapes built on top of it.

export * from "./domain";

import type { AppError } from "./platform/errors";
import {
  isItem,
  type Entry,
  type Annotation,
  type Topic,
  type Feed,
  type Item,
  type Countable,
} from "./domain";

// ── Utils ────────────────────────────────────────────────────────────────────

export type AbortError = DOMException;

export function isAbortError(value?: unknown): value is AbortError {
  return value instanceof DOMException && value?.name === "AbortError";
}

// ── Query results (derived) ────────────────────────────────────────────────────

/** A domain item carrying its similarity score from an embedding query. */
export type Ranked<T extends Item> = T & { similarity: number };

export type RankedItem = Ranked<Entry> | Ranked<Topic> | Ranked<Feed>;

export function isRanked<T extends Item>(o: T | null | undefined): o is Ranked<T> {
  return !!o && typeof (o as Ranked<T>).similarity === "number";
}

// ── Items list (derived) ────────────────────────────────────────────────────────

/** An item in the list returned by listAllItems: an entry (any state) or an annotation. */
export type ListItem = Entry | PartialAnnotation;

// ── Cache projection (derived) ──────────────────────────────────────────────────

/** An annotation record as stored in the embeddings DB — no text[]/occurrence[]/content. */
export type PartialAnnotation = Pick<
  Annotation,
  "id" | "title" | "createdAt" | "reference" | "color"
> & { url: string };

// An annotation in list form (saved list or embedding-query result). Identified by reference +
// color + url; a full `Annotation` carries no url, which is what separates the two forms.
export function isPartialAnnotation(item?: Item | null): item is PartialAnnotation {
  return (
    isItem(item) &&
    "color" in item &&
    typeof (item as PartialAnnotation).color === "string" &&
    "reference" in item &&
    typeof (item as PartialAnnotation).reference === "string" &&
    "url" in item &&
    typeof (item as PartialAnnotation).url === "string"
  );
}

// ── Presentation ──────────────────────────────────────────────────────────────

const FILTERS = ["recommended", "all", "unread", "bookmarked", "saved", "archived"] as const;

export type Filter = (typeof FILTERS)[number];

export function isFilter(value?: unknown): value is Filter {
  return (
    !!value && typeof value === "string" && (FILTERS as readonly string[]).includes(value)
  );
}

const TOPIC_SORTS: string[] = ["relevance", "timestamp"] as const;

export type TopicSort = (typeof TOPIC_SORTS)[number];

export function isTopicSort(value?: unknown): value is TopicSort {
  return !!value && typeof value === "string" && TOPIC_SORTS.includes(value);
}

const HTML_READER_VIEWS: string[] = ["reader", "summary"] as const;

export type HtmlReaderView = (typeof HTML_READER_VIEWS)[number];

export function isHtmlReaderView(value?: string | null): value is HtmlReaderView {
  return !!value && HTML_READER_VIEWS.includes(value);
}

/** A saved filter view: a count-bearing item that carries a Filter (e.g. Unread (42)). */
export type Shortcut = Countable & { filter: Filter };

export function isShortcut(item?: Item | null): item is Shortcut {
  return isItem(item) && "filter" in item && isFilter((item as Shortcut).filter);
}

// ── Import ───────────────────────────────────────────────────────────────────

/** Result returned by the backend after any bulk import operation. */
export type ImportResult<T> = {
  added: T[];
  skipped: string[];
  failed: string[];
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

export type ExportOutcome = {
  error: AppError | null;
};

// ── Navigation ───────────────────────────────────────────────────────────────

export type NavigationState = {
  topicId?: Topic["id"];
  feedId?: Feed["id"];
  shortcutId?: Shortcut["id"];
  entryId?: Entry["id"];
  annotationId?: Annotation["id"];
  url?: Entry["url"];
  mimeType?: string;
  filter?: Filter;
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
  }[];
  droppedEvents: number;
};
