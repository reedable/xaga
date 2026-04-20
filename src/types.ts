// ── Presentation ─────────────────────────────────────────────────────────────

const FILTERS: string[] = [
  "recommended",
  "all",
  "unread",
  "bookmarked",
  "upcoming",
  "archived",
] as const;

export type Filter = (typeof FILTERS)[number];

export function isFilter(value?: unknown): value is Filter {
  return !!value && typeof value === "string" && FILTERS.includes(value);
}

const READER_VIEWS: string[] = ["full", "summary"] as const;

export type ReaderView = (typeof READER_VIEWS)[number];

export function isReaderView(value?: string | null): value is ReaderView {
  return !!value && READER_VIEWS.includes(value);
}

/** Minimal contract shared by every displayable item (entries, notes, …). */
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

export type Embedded = {
  similarity: number;
};

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function isEmbedded(obj: any): obj is Embedded {
  return obj && typeof obj === "object" && typeof obj.similarity === "number";
}

export type Shortcut = Item & {
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
};

export function isEntry(item?: Item | null): item is Entry {
  return !!item && "url" in item && typeof item.url === "string";
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
  reference: string;
  // One text segment per block-level element covered by the selection.
  // Single-paragraph selections produce a one-element array.
  text: string[];
  // 0-based occurrence index for each corresponding entry in `text`.
  occurrence: number[];
  color: string;
  note?: string;
};

export function isAnnotation(item?: Item | null): item is Annotation {
  return !!item && "color" in item;
}

// ── Article ──────────────────────────────────────────────────────────────────

/** A fully loaded saved entry, including markdown content and annotations. */
export type Article = Entry &
  Document & {
    annotations: Annotation[];
  };

export function isArticle(item?: Item | null): item is Article {
  return !!item && isEntry(item) && isDocument(item);
}

// ── Topic ────────────────────────────────────────────────────────────────────

export type Topic = Document;

export function isTopic(item?: Item | null): item is Topic {
  return isDocument(item);
}

// ── Note ─────────────────────────────────────────────────────────────────────

/** A user-written markdown note (not URL-derived). */
export type Note = Document;

export function isNote(item?: Item | null): item is Note {
  return isDocument(item);
}

// ── Feed ─────────────────────────────────────────────────────────────────────

/** A subscribed RSS/Atom feed. */
export type Feed = Item & {
  url: string;
  entry_count: number;
};

export function isFeed(item?: Item | null): item is Feed {
  return (
    !!item && "entry_count" in item && typeof item.entry_count === "number"
  );
}

// ── Import ───────────────────────────────────────────────────────────────────

/** Result returned by the backend after any bulk import operation. */
export type ImportResult<T> = {
  added: T[];
  skipped: string[];
  failed: string[];
};
