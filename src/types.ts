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

export function isFilter(value?: string | null): value is Filter {
  return !!value && FILTERS.includes(value);
}

const VIEWS: string[] = ["full", "summary"] as const;

export type View = (typeof VIEWS)[number];

export function isView(value?: string | null): value is View {
  return !!value && VIEWS.includes(value);
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

/** Items that carry full content and can be indexed by the backend. */
export type Document = Item & {
  content: string;
};

export function isDocument(item?: Item | null): item is Document {
  return !!item && "content" in item && typeof item.content === "string";
}

/** A fully loaded saved entry, including markdown content. */
export type Article = Entry & Document;

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

/** Result returned by the backend after importing an OPML file. */
export type FeedImportResult = {
  added: number;
  skipped: number;
  failed: number;
};
