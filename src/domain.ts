// Domain model. Pure: no persistence (.md/.atom), cache (DB row), query (similarity/counts),
// or UI (filter/nav) concern appears here. types.ts derives its shapes from these.

// ── Item ────────────────────────────────────────────────────────────────────────
export type Item = {
  id: string;
  title: string;
  createdAt: string;
  updatedAt?: string;
  description?: string; // any item may have one
  publishedAt?: string; // any item may be published
};

// ── Capabilities ─────────────────────────────────────────────────────────────────
export type WebResource = Item & { url: string }; // has a URL
export type Document = Item & { content: string }; // has body content
export type Countable = Item & { entryCount: number; unreadCount: number }; // authoritative summary stats
export type Pinnable = Item & { isPinned: boolean };
export type Colorable = Item & { color?: string }; // Annotation (highlight), bookmarked Entry, Note

// A typed, branded pointer to an item (the brand matches the referenced entity's __kind).
// Value object — NOT an entity, NOT an Item. (`Reference`, not `Ref` — avoids React's ref.)
export type Reference<T extends Item & { __kind: string }> = {
  readonly __kind: T["__kind"];
  id: T["id"];
};

// A collection carries BOTH its typed member references AND its summary counts (Countable).
// The two need not agree: with live fetch the loaded `members` won't always match the
// authoritative count, so counts are real data — NOT derived from `members.length`.
export type Collection<T extends Item & { __kind: string }> = Countable & {
  members: Reference<T>[];
};

// Library-scoped organizational label; identity is independent of display name.
export type Tag = { id: string; name: string };

// ── Entities (all branded) ──────────────────────────────────────────────────────
export type Entry = WebResource &
  Pinnable &
  Colorable & {
    readonly __kind: "entry";
    isUnread?: boolean;
    // description? / publishedAt? from Item; color? (from Colorable) = the bookmark color
  };

// Feed and Topic are the same kind of thing — a pinnable Collection — differing only in member
// type, Topic's threshold + self-curation, and Feed's URL vs Topic's content.
export type Feed = Collection<Entry> &
  WebResource &
  Pinnable & {
    readonly __kind: "feed"; // description? (from Item) = the .atom subtitle
    // createdAt (from Item) is NOT a real creation time for a Feed: Atom has no feed-level creation
    // field (`atom:feed` has `<updated>`, no `<published>`), so the backend mirrors `<updated>` into
    // it for now. Tech debt — don't rely on Feed.createdAt as a true creation timestamp.
  };

export type Topic = Collection<Entry | Annotation | Note> &
  Pinnable & {
    readonly __kind: "topic";
    threshold?: number; // semantic threshold (Topic only)
    // NOT a Document — a Topic has no body content; its blurb is description? (from Item)
  };

// A loaded, content-bearing entry — Entry & Document; __kind stays "entry" (an Article IS an Entry).
export type Article = Entry &
  Document & {
    annotations: Annotation[];
    favicon?: string;
  };

// The content entity with NO web presence — the counterpart to Article.
export type Note = Document &
  Colorable & {
    readonly __kind: "note";
  };

export type Annotation = Document &
  Colorable & {
    readonly __kind: "annotation";
    reference: Item["id"]; // parent article id
    text: string[]; // highlighted span(s); the title derives from these
    occurrence: number[];
    color: string; // narrows Colorable's optional — a highlight always has one
    // content (from Document) = the user's comment on the highlight (was `note`); mandatory,
    // "" for a plain highlight. NOT the `Note` entity.
  };

// ── Guards (identity = structural shape AND the __kind brand, together) ──────────

function brandOf(o: unknown): string | undefined {
  return (o as { __kind?: unknown } | null | undefined)?.__kind as string | undefined;
}

export function isItem(o: unknown): o is Item {
  return (
    !!o &&
    typeof o === "object" &&
    typeof (o as Item).id === "string" &&
    typeof (o as Item).title === "string" &&
    typeof (o as Item).createdAt === "string"
  );
}

export function isEntry(o: unknown): o is Entry {
  return isItem(o) && brandOf(o) === "entry" && typeof (o as Entry).url === "string";
}

export function isFeed(o: unknown): o is Feed {
  return isItem(o) && brandOf(o) === "feed" && Array.isArray((o as Feed).members);
}

export function isTopic(o: unknown): o is Topic {
  return isItem(o) && brandOf(o) === "topic" && Array.isArray((o as Topic).members);
}

export function isNote(o: unknown): o is Note {
  return isItem(o) && brandOf(o) === "note" && typeof (o as Note).content === "string";
}

export function isAnnotation(o: unknown): o is Annotation {
  return isItem(o) && brandOf(o) === "annotation" && Array.isArray((o as Annotation).text);
}

// An Article is an Entry (brand "entry") that also carries content + annotations.
export function isArticle(o: unknown): o is Article {
  return (
    isEntry(o) &&
    typeof (o as Article).content === "string" &&
    Array.isArray((o as Article).annotations)
  );
}

export function isDocument(o: unknown): o is Document {
  return isItem(o) && typeof (o as Document).content === "string";
}

export function isCollection(o: unknown): o is Feed | Topic {
  return (
    isItem(o) &&
    Array.isArray((o as { members?: unknown }).members) &&
    typeof (o as Countable).entryCount === "number" &&
    typeof (o as Countable).unreadCount === "number"
  );
}
