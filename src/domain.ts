// Domain model. Pure: no persistence (.md/.atom), cache (DB row), query (similarity/counts),
// or UI (filter/nav) concern appears here. types.ts derives its shapes from these.

// ── Entities ────────────────────────────────────────────────────────────────────
export type Entity = {
  id: string;
  title: string;
};

// An entity with an independent managed record and lifecycle.
export type ManagedEntity = Entity & {
  createdAt: string;
  updatedAt?: string;
  description?: string; // any managed entity may have one
  publishedAt?: string; // any managed entity may be published
};

// ── Capabilities ─────────────────────────────────────────────────────────────────
export type WebResource = ManagedEntity & { url: string }; // has a URL
export type Document = ManagedEntity & { content: string }; // has body content
export type Countable = { entryCount: number; unreadCount: number }; // authoritative summary stats
export type Pinnable = ManagedEntity & { isPinned: boolean };
export type Colorable = ManagedEntity & { color?: string }; // Annotation (highlight), bookmarked Entry, Note

// A typed, branded pointer to an entity (the brand matches the referenced entity's __kind).
// Value object — NOT an entity. (`Reference`, not `Ref` — avoids React's ref.)
export type Reference<T extends ManagedEntity & { __kind: string }> = {
  readonly __kind: T["__kind"];
  id: T["id"];
  articleId?: string; // Containing article when the target is an annotation.
};

// Common browsing contract for supplied or derived membership. A collection does not
// require an independent persisted record, creation timestamp, or loaded member list.
export type Collection = Entity & Countable & {
  readonly __kind: "feed" | "topic" | "site";
};

// Managed collections have their own identity and lifecycle independent of their members.
// They carry BOTH typed member references AND authoritative summary counts (Countable).
// The two need not agree: with live fetch the loaded `members` won't always match the
// authoritative count, so counts are real data — NOT derived from `members.length`.
export type ManagedCollection<T extends ManagedEntity & { __kind: string }> =
  Collection & ManagedEntity & {
    readonly __kind: "feed" | "topic";
    members: Reference<T>[];
  };

// Committed assignment intent carried to consumers.
export type EntryTagChange = { entryId: string; tagId: string; assigned: boolean };

// Library-scoped organizational label; identity is independent of display name.
export type Tag = { id: string; name: string };
export type TagMerge = { sourceId: string; destinationId: string; survivorId: string; destinationName: string };
export type TagSnapshot = { tags: Tag[]; renaming: string[]; deleting: string[]; merging: TagMerge[] };

// ── Domain types (all branded) ──────────────────────────────────────────────────
export type Entry = WebResource &
  Pinnable &
  Colorable & {
    readonly __kind: "entry";
    isUnread?: boolean;
    // description? / publishedAt? from ManagedEntity; color? (from Colorable) = the bookmark color
  };

// Feed and Topic are pinnable ManagedCollections with different member types.
// Feed adds a URL; Topic adds a semantic threshold and curated membership.
export type Feed = ManagedCollection<Entry> &
  WebResource &
  Pinnable & {
    readonly __kind: "feed"; // description? (from ManagedEntity) = the .atom subtitle
    // createdAt (from ManagedEntity) is NOT a real creation time for a Feed: Atom has no feed-level creation
    // field (`atom:feed` has `<updated>`, no `<published>`), so the backend mirrors `<updated>` into
    // it for now. Tech debt — don't rely on Feed.createdAt as a true creation timestamp.
  };

export type Topic = ManagedCollection<Entry | Annotation | Note> &
  Pinnable & {
    readonly __kind: "topic";
    threshold?: number; // semantic threshold (Topic only)
    // NOT a Document — a Topic has no body content; its blurb is description? (from ManagedEntity)
  };

// A Collection whose membership derives from stored article URLs, without its own record.
export type Site = Collection & {
  readonly __kind: "site";
  id: string; // The normalized hostname; also used as the displayed title.
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
    reference: ManagedEntity["id"]; // parent article id
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

export function isEntity(o: unknown): o is Entity {
  return (
    !!o &&
    typeof o === "object" &&
    typeof (o as Entity).id === "string" &&
    typeof (o as Entity).title === "string"
  );
}

export function isManagedEntity(o: unknown): o is ManagedEntity {
  return isEntity(o) && typeof (o as ManagedEntity).createdAt === "string";
}

export function isEntry(o: unknown): o is Entry {
  return isManagedEntity(o) && brandOf(o) === "entry" && typeof (o as Entry).url === "string";
}

export function isFeed(o: unknown): o is Feed {
  return isManagedEntity(o) && brandOf(o) === "feed" && Array.isArray((o as Feed).members);
}

export function isTopic(o: unknown): o is Topic {
  return isManagedEntity(o) && brandOf(o) === "topic" && Array.isArray((o as Topic).members);
}

export function isNote(o: unknown): o is Note {
  return isManagedEntity(o) && brandOf(o) === "note" && typeof (o as Note).content === "string";
}

export function isAnnotation(o: unknown): o is Annotation {
  return isManagedEntity(o) && brandOf(o) === "annotation" && Array.isArray((o as Annotation).text);
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
  return isManagedEntity(o) && typeof (o as Document).content === "string";
}

export function isManagedCollection(o: unknown): o is Feed | Topic {
  return (
    (isFeed(o) || isTopic(o)) &&
    typeof (o as Countable).entryCount === "number" &&
    typeof (o as Countable).unreadCount === "number"
  );
}

export function isSite(o: unknown): o is Site {
  return (
    isEntity(o) &&
    brandOf(o) === "site" &&
    typeof (o as Site).entryCount === "number" &&
    typeof (o as Site).unreadCount === "number"
  );
}

export function isCollection(o: unknown): o is Feed | Topic | Site {
  return isManagedCollection(o) || isSite(o);
}
