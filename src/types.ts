// ── Utils ────────────────────────────────────────────────────────────────────

export type AbortError = DOMException;

export function isAbortError(value?: unknown): value is AbortError {
  return value instanceof DOMException && value?.name === "AbortError";
}

// ── Presentation ─────────────────────────────────────────────────────────────

const FILTERS: string[] = [
  "recommended",
  "all",
  "unread",
  "bookmarked",
  "saved",
  "archived",
] as const;

export type Filter = (typeof FILTERS)[number];

export function isFilter(value?: unknown): value is Filter {
  return !!value && typeof value === "string" && FILTERS.includes(value);
}

const HTML_READER_VIEWS: string[] = ["full", "summary"] as const;

export type HtmlReaderView = (typeof HTML_READER_VIEWS)[number];

export function isHtmlReaderView(value?: string | null): value is HtmlReaderView {
  return !!value && HTML_READER_VIEWS.includes(value);
}

/** Minimal contract shared by every displayable item. */
export type Item = {
  id: string;
  title: string;
  createdAt: string;
  updatedAt?: string;
};

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function isItem(obj: any): obj is Item {
  return (
    obj &&
    typeof obj === "object" &&
    typeof obj.id === "string" &&
    typeof obj.title === "string" &&
    typeof obj.createdAt === "string"
  );
}

export type Collection = Item & {
  entryCount: number;
  unreadCount: number;
};

export function isCollection(item?: Item | null): item is Collection {
  return (
    isItem(item) &&
    "entryCount" in item &&
    typeof item.entryCount === "number" &&
    "unreadCount" in item &&
    typeof item.unreadCount === "number"
  );
}

export type Embedded = {
  similarity: number;
};

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function isEmbedded(obj: any): obj is Embedded {
  return obj && typeof obj === "object" && typeof obj.similarity === "number";
}

export type Shortcut = Collection & {
  filter: Filter;
};

export function isShortcut(item: Item): item is Shortcut {
  return isItem(item) && "filter" in item && isFilter(item.filter);
}

// ── Entry ────────────────────────────────────────────────────────────────────

/** Abstract representation of any feed entry, regardless of source. */
export type Entry = Item & {
  url: string;
  publishedAt?: string;
  summary?: string;
  isUnread?: boolean;
  isBookmarked?: boolean;
};

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function isEntry(item?: any): item is Entry {
  return isItem(item) && "url" in item && typeof item.url === "string";
}

/** Items that carry full content. */
export type Document = Item & {
  content: string;
};

export function isDocument(item?: Item | null): item is Document {
  return !!item && "content" in item && typeof item.content === "string";
}

// ── Annotation ───────────────────────────────────────────────────────────────

/** A saved highlight or annotation on a span of article text. */
export type Annotation = Item & {
  // The ID of the article this annotation belongs to.
  reference: Item["id"];

  // One text segment per block-level element covered by the selection.
  // Single-paragraph selections produce a one-element array.
  text: string[];

  // 0-based occurrence index for each corresponding entry in `text`.
  occurrence: number[];

  color: string;
  note?: string;
};

export function isAnnotation(item?: Item | null): item is Annotation {
  return (
    !!item &&
    "color" in item &&
    typeof item.color === "string" &&
    "reference" in item &&
    typeof item.reference == "string"
  );
}

// ── SavedItem ────────────────────────────────────────────────────────────────

/** A bookmark or annotation returned by list_saved. Annotations have reference and color set. */
export type SavedItem = Item & {
  url: string;
  isUnread?: boolean;
  isBookmarked?: boolean;
  /** Present for annotations (parent article ID); absent for bookmarks. */
  reference?: string;
  /** Present for annotations; absent for bookmarks. */
  color?: string;
};

export function isSavedAnnotation(
  item: SavedItem
): item is SavedItem & { reference: string; color: string } {
  return typeof item.reference === "string" && typeof item.color === "string";
}

// ── AnnotationSummary ────────────────────────────────────────────────────────

/** Lightweight annotation record from the embeddings DB — no text[] or occurrence[]. */
export type AnnotationSummary = Item & {
  reference: string;
  color: string;
  /** The canonical URL of the parent article. */
  url: string;
};

export function isAnnotationSummary(
  item?: Item | null
): item is AnnotationSummary {
  return (
    !!item &&
    "color" in item &&
    typeof item.color === "string" &&
    "reference" in item &&
    typeof item.reference === "string" &&
    "url" in item &&
    typeof item.url === "string"
  );
}

// ── Article ──────────────────────────────────────────────────────────────────

/** A fully loaded saved entry, including markdown content and annotations. */
export type Article = Entry &
  Document & {
    annotations: Annotation[];
    favicon?: string;
  };

export function isArticle(item?: Item | null): item is Article {
  return !!item && isEntry(item) && isDocument(item);
}

// ── Topic ────────────────────────────────────────────────────────────────────

export type Topic = Document & Collection & { threshold?: number };

export function isTopic(item?: Item | null): item is Topic {
  return isDocument(item) && isCollection(item);
}

// ── Feed ─────────────────────────────────────────────────────────────────────

/** A subscribed RSS/Atom feed. */
export type Feed = Collection & {
  url: string;
};

export function isFeed(item?: Item | null): item is Feed {
  return isCollection(item) && "url" in item && typeof item.url === "string";
}

// ── Import ───────────────────────────────────────────────────────────────────

// ── CollectionItem ────────────────────────────────────────────────────────────

/** A single member of a Collection. Mirrors the Rust `CollectionItem` enum. */
export type CollectionItem =
  | { type: "entry"; id: string }
  | { type: "annotation"; id: string };

/** Result returned by the backend after any bulk import operation. */
export type ImportResult<T> = {
  added: T[];
  skipped: string[];
  failed: string[];
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
};
