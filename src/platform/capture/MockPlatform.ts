import type { FetchSettings } from "../fetchSettings";
import type { SidebarSortMode } from "../../sidebarSort";
import type { Platform } from "../Platform";
import type { Entry, Filter, ListEntity, Topic, Annotation } from "../../types";
import { pinKey, type PinTarget } from "../../pins";
import {
  createFixtures,
  captureDate,
  additionalFeedUrl,
  captureImages,
} from "./fixtures";

type EventName = {
  [K in keyof Platform]: K extends `on${string}` ? K : never;
}[keyof Platform];
type EventValue<K extends EventName> = Parameters<
  Parameters<Platform[K]>[0]
>[0];

export class MockPlatform implements Platform {
  private data = createFixtures();
  private pins: PinTarget[] = [{ type: "topic", id: "observations" }];
  private sequence = 0;
  private upgrade: import("../Platform").LibraryUpgrade = { state: "idle" };
  private upgradeAttempts = 0;
  private upgradePolls = 0;
  private refreshAttempts = 0;
  private scenario = new URLSearchParams(location.search).get("scenario");
  private listeners = new Map<EventName, Set<(value: never) => void>>();
  private fetchSettings: FetchSettings = {
    refererAllowed: false,
    gzipAllowed: false,
    httpAllowed: false,
    connectNonstandardPortsAllowed: false,
    maxRedirects: "3",
    loaderTimeoutSecs: "10",
    feedTimeoutSecs: "10",
    feedMaxBodyBytes: "10485760",
  };
  private importMaxBytes = 64 * 1024 * 1024;
  private refreshOnStartup = false;
  private settingsAttempts = 0;
  private theme = "light";
  private sidebarSort: SidebarSortMode = "unread";

  constructor(theme: "light" | "dark" = "light") {
    this.theme = theme;
    if (["refresh-failure", "refresh-all-fail"].includes(this.scenario ?? "")) {
      this.data.feeds.push({
        ...this.data.feeds[0],
        id: "workshop",
        title: "Workshop",
        url: additionalFeedUrl,
        members: [],
      });
    }
    if (this.scenario === "refresh-many-failures") {
      for (let i = 1; i < 14; i++)
        this.data.feeds.push({
          ...this.data.feeds[0],
          id: `failed-${i}`,
          title: `Additional feed ${i}`,
          url: `https://feed-${i}.example/feed.xml`,
          members: [],
        });
    }
    if (localStorage.getItem("xaga-capture-sidebar-sort") === "alphabetical")
      this.sidebarSort = "alphabetical";
  }
  private sidebar = true;
  private view = "reader";
  private prefs = { fontFamily: "system", fontSize: 18, lineHeight: 1.6 };

  private subscribe<K extends EventName>(
    name: K,
    callback: Parameters<Platform[K]>[0]
  ) {
    const listeners = this.listeners.get(name) ?? new Set();
    const listener = callback as (value: never) => void;
    listeners.add(listener);
    this.listeners.set(name, listeners);
    return Promise.resolve(() => {
      listeners.delete(listener);
    });
  }
  private emit<K extends EventName>(name: K, value: EventValue<K>) {
    this.listeners
      .get(name)
      ?.forEach((callback) => callback(structuredClone(value) as never));
  }
  private unsupported(operation: string): never {
    throw {
      code: "app:error",
      message: `${operation} is unavailable in browser capture mode.`,
      detail: "Use the native Xaga app for this operation.",
    };
  }
  private article(id: string) {
    const article = this.data.articles.find((value) => value.id === id);
    if (!article)
      throw {
        code: "entry:not-found",
        message: "This example article is unavailable.",
        detail: id,
      };
    return article;
  }
  private topic(id: string) {
    const topic = this.data.topics.find((value) => value.id === id);
    if (!topic) return this.unsupported("Unknown Topic");
    return topic;
  }
  private project<T extends Entry | Topic | (typeof this.data.feeds)[number]>(
    value: T
  ): T {
    return structuredClone({
      ...value,
      isPinned: this.pins.some(
        (pin) => pin.type === value.__kind && "id" in pin && pin.id === value.id
      ),
    });
  }
  private annotations() {
    return this.data.articles.flatMap((article) =>
      article.annotations.map((annotation) => ({
        ...annotation,
        url: article.url,
      }))
    );
  }
  private filter(values: ListEntity[], filter: Filter = "all") {
    return structuredClone(
      values.filter((value) => {
        const entry = "isPinned" in value;
        if (filter === "unread") return entry && value.isUnread;
        if (filter === "bookmarked") return entry && !!value.color;
        if (filter === "saved") return !!value.color;
        if (filter === "archived") return entry && !value.isUnread;
        return true;
      })
    );
  }
  private counts(ids: string[]) {
    return {
      entryCount: ids.length,
      unreadCount: ids.filter((id) =>
        this.data.articles.some((a) => a.id === id && a.isUnread)
      ).length,
    };
  }
  private changed() {
    this.emit("onEmbeddingsChanged", undefined);
  }
  private entryChange(
    id: string,
    patch: Partial<Entry>,
    event: "onUnreadChanged" | "onBookmarkChanged"
  ) {
    Object.assign(this.article(id), patch);
    const value = this.project(this.article(id));
    this.emit(event, value);
    return value;
  }
  getLibraryStatus: Platform["getLibraryStatus"] = async () => {
    const kind =
      this.scenario === "upgrade-status-change" && this.readOnlyContinued
        ? "pending-other"
        : this.scenario?.replace("upgrade-", "");
    const cause: import("../Platform").LibraryReadOnlyCause | null =
      kind === "newer"
        ? "newer"
        : kind === "pending-other"
          ? "pending_other_version"
          : kind === "resume" || kind === "resume-earlier"
            ? "pending_upgrade"
            : kind === "invalid"
              ? "invalid_metadata"
              : kind === "unversioned"
                ? "unversioned"
                : this.scenario?.startsWith("upgrade-")
                  ? "older"
                  : (await this.isLibraryReadOnly())
                    ? "lock_unavailable"
                    : null;
    return {
      path: await this.getLibrary(),
      appVersion: "0.16",
      storedVersion:
        kind === "unversioned" || kind === "invalid"
          ? null
          : kind === "resume-earlier"
            ? "0.9"
            : kind === "newer"
              ? "0.17"
              : cause === "older" || cause?.startsWith("pending")
                ? "0.15"
                : "0.16",
      pendingTarget:
        kind === "pending-other"
          ? "0.17"
          : kind === "resume-earlier"
            ? "0.12"
            : kind === "resume"
              ? "0.16"
              : null,
      readOnlyCause: cause,
      metadataError:
        kind === "invalid"
          ? "The library version metadata is malformed."
          : null,
    };
  };
  startLibraryUpgrade: Platform["startLibraryUpgrade"] = async (path) => {
    if (this.upgrade.state === "running" || this.upgrade.state === "succeeded")
      return structuredClone(this.upgrade);
    if (path !== (await this.getLibrary()))
      throw new Error("Selected library changed");
    this.upgradeAttempts += 1;
    this.upgradePolls = 0;
    this.upgrade = {
      state: "running",
      progress: {
        phase: "converting",
        converted: 2,
        unchanged: 1,
        reconciled: 3,
      },
    };
    return structuredClone(this.upgrade);
  };
  getLibraryUpgrade: Platform["getLibraryUpgrade"] = async () => {
    if (
      this.scenario === "upgrade-poll-failure" &&
      this.upgrade.state === "running" &&
      this.upgradePolls++ === 0
    )
      throw new Error("fixture polling interruption");
    if (this.upgrade.state === "running" && ++this.upgradePolls >= 3) {
      this.upgrade =
        ["upgrade-failure", "upgrade-read-failure"].includes(
          this.scenario ?? ""
        ) && this.upgradeAttempts === 1
          ? {
              state: "failed",
              read_error:
                this.scenario === "upgrade-read-failure"
                  ? "Fixture read handle unavailable"
                  : null,
              message:
                "Library upgrade incomplete. Resolve the reported problems and retry with this Xaga version.",
              errors: [
                "A library file could not be converted. Check its permissions and retry.",
              ],
            }
          : {
              state: "succeeded",
              report: { converted: 2, unchanged: 1, reconciled: 3, errors: [] },
            };
    }
    return structuredClone(this.upgrade);
  };
  restartAfterLibraryUpgrade: Platform["restartAfterLibraryUpgrade"] =
    async () =>
      "The library upgrade completed, but Xaga could not restart. Try again, or close Xaga and reopen this library.";
  getLibrary: Platform["getLibrary"] = async () => "/examples/Capture library";
  isLibraryReadOnly: Platform["isLibraryReadOnly"] = async () => {
    if (this.scenario === "read-only-delayed")
      await new Promise((resolve) => setTimeout(resolve, 500));
    return (
      this.scenario === "read-only" ||
      this.scenario === "read-only-delayed" ||
      !!this.scenario?.startsWith("upgrade-")
    );
  };
  private readOnlyContinued = false;
  continueLibraryReadOnly: Platform["continueLibraryReadOnly"] = async () => {
    this.readOnlyContinued = true;
  };
  embeddingStartupState: Platform["embeddingStartupState"] = async () => {
    if ((await this.isLibraryReadOnly()) && !this.readOnlyContinued)
      return "awaiting_library_choice";
    if (this.scenario === "upgrade-model-delayed")
      await new Promise((resolve) => setTimeout(resolve, 500));
    return this.scenario === "upgrade-model-unavailable"
      ? "unavailable"
      : "available";
  };
  setWindowTitle: Platform["setWindowTitle"] = async (title) => {
    document.title = title;
  };
  setFindEnabled: Platform["setFindEnabled"] = async () => {};
  setShortcutsMenu: Platform["setShortcutsMenu"] = async () => {};
  listFeeds: Platform["listFeeds"] = async () =>
    this.data.feeds.map((f) =>
      this.project({ ...f, ...this.counts(f.members.map((m) => m.id)) })
    );
  addFeed: Platform["addFeed"] = async (signal, url) => {
    signal.throwIfAborted();
    if (url !== additionalFeedUrl)
      return this.unsupported(`Adding feeds other than ${additionalFeedUrl}`);
    let feed = this.data.feeds.find((f) => f.url === url);
    if (!feed) {
      feed = {
        __kind: "feed",
        id: "workshop",
        title: "The workshop",
        url,
        createdAt: captureDate,
        isPinned: false,
        members: [{ __kind: "entry", id: "wood" }],
        ...this.counts(["wood"]),
      };
      this.data.feeds.push(feed);
    }
    this.changed();
    return this.project(feed);
  };
  renameFeed: Platform["renameFeed"] = async (_signal, id, title) => {
    const feed = this.data.feeds.find((f) => f.id === id);
    if (!feed) return this.unsupported("Unknown feed");
    feed.title = title;
    return this.project(feed);
  };
  removeFeed: Platform["removeFeed"] = async (_signal, url) => {
    this.data.feeds = this.data.feeds.filter((f) => f.url !== url);
    this.changed();
    return url;
  };
  listEntries: Platform["listEntries"] = async (_signal, url, filter) => {
    const ids =
      this.data.feeds.find((f) => f.url === url)?.members.map((m) => m.id) ??
      [];
    return this.filter(
      this.data.articles
        .filter((a) => ids.includes(a.id))
        .map((a) => this.project(a)),
      filter
    ) as Entry[];
  };
  listAllEntities: Platform["listAllEntities"] = async (_signal, filter) =>
    this.filter(
      [
        ...this.data.articles.map((a) => this.project(a)),
        ...this.annotations(),
      ],
      filter
    );
  getAllEntitiesCounts: Platform["getAllEntitiesCounts"] = async () =>
    this.scenario === "upgrade-read-failure" && this.upgrade.state === "failed"
      ? null
      : this.counts(this.data.articles.map((a) => a.id));
  readEntry: Platform["readEntry"] = async (_signal, id) =>
    this.project(this.article(id));
  readArticleScope: Platform["readArticleScope"] = async (_signal, id) =>
    this.project(this.article(id));
  readArticle: Platform["readArticle"] = async (_signal, id) =>
    this.scenario === "read-only" ||
    this.scenario === "read-only-delayed" ||
    !!this.scenario?.startsWith("upgrade-")
      ? this.project(this.article(id))
      : this.entryChange(id, { isUnread: false }, "onUnreadChanged");
  findArticle: Platform["findArticle"] = async (_signal, url) => {
    const a = this.data.articles.find((a) => a.url === url);
    return a ? this.project(a) : null;
  };
  markEntryRead: Platform["markEntryRead"] = async (_signal, id) =>
    this.entryChange(id, { isUnread: false }, "onUnreadChanged");
  markEntryUnread: Platform["markEntryUnread"] = async (_signal, id) =>
    this.entryChange(id, { isUnread: true }, "onUnreadChanged");
  addBookmark: Platform["addBookmark"] = async (_signal, id, color) =>
    this.entryChange(id, { color }, "onBookmarkChanged");
  removeBookmark: Platform["removeBookmark"] = async (_signal, id) =>
    this.entryChange(id, { color: undefined }, "onBookmarkChanged");
  saveArticle: Platform["saveArticle"] = async (_signal, article) => {
    Object.assign(this.article(article.id), structuredClone(article));
    this.changed();
    return this.project(this.article(article.id));
  };
  readEntryImages: Platform["readEntryImages"] = async (_signal, _id, refs) =>
    refs.map((ref) => captureImages[ref] ?? null);
  recoverEntryImages: Platform["recoverEntryImages"] = async (_signal, id) =>
    this.project(this.article(id));
  createArticleDiagnostic: Platform["createArticleDiagnostic"] = async () =>
    `capture-diagnostic-${++this.sequence}`;
  addArticleDiagnosticEvent: Platform["addArticleDiagnosticEvent"] =
    async () => {};
  getArticleDiagnostic: Platform["getArticleDiagnostic"] = async () => null;
  addAnnotation: Platform["addAnnotation"] = async (_signal, input) => {
    const article = this.article(input.reference);
    const existing = article.annotations.find((a) => a.id === input.id);
    const annotation: Annotation = {
      ...structuredClone(input),
      __kind: "annotation",
      createdAt: existing?.createdAt ?? captureDate,
      updatedAt: captureDate,
    };
    article.annotations = [
      ...article.annotations.filter((a) => a.id !== input.id),
      annotation,
    ];
    this.emit("onAnnotationChanged", { ...annotation, url: article.url });
    this.changed();
    return this.project(article);
  };
  removeAnnotation: Platform["removeAnnotation"] = async (_signal, input) => {
    const article = this.article(input.reference);
    article.annotations = article.annotations.filter((a) => a.id !== input.id);
    this.data.topics.forEach((t) => {
      t.members = t.members.filter((m) => m.id !== input.id);
    });
    delete this.data.assignments[input.id];
    this.emit("onAnnotationRemoved", input.id);
    this.changed();
    return this.project(article);
  };
  replaceAnnotation: Platform["replaceAnnotation"] = async (
    _signal,
    oldId,
    input
  ) => {
    const article = this.article(input.reference);
    const annotation: Annotation = {
      ...structuredClone(input),
      __kind: "annotation",
      createdAt: captureDate,
      updatedAt: captureDate,
    };
    article.annotations = [
      ...article.annotations.filter((a) => a.id !== oldId),
      annotation,
    ];
    this.data.topics.forEach((topic) => {
      topic.members = topic.members.map((member) =>
        member.id === oldId ? { ...member, id: input.id } : member
      );
    });
    this.data.assignments[input.id] = this.data.assignments[oldId] ?? [];
    delete this.data.assignments[oldId];
    this.emit("onAnnotationReplaced", {
      oldId,
      entity: { ...annotation, url: article.url },
    });
    this.changed();
    return this.project(article);
  };
  listAnnotations: Platform["listAnnotations"] = async () =>
    structuredClone(this.annotations());
  listTopics: Platform["listTopics"] = async () =>
    this.data.topics.map((t) =>
      this.project({ ...t, ...this.counts(t.members.map((m) => m.id)) })
    );
  readTopic: Platform["readTopic"] = async (signal, id) =>
    (await this.listTopics(signal)).find((t) => t.id === id) ??
    this.unsupported("Unknown Topic");
  createTopic: Platform["createTopic"] = async (_signal, input) => {
    const topic: Topic = {
      ...input,
      __kind: "topic",
      id: `capture-topic-${++this.sequence}`,
      createdAt: captureDate,
      members: [],
      isPinned: false,
      entryCount: 0,
      unreadCount: 0,
    };
    this.data.topics.push(topic);
    this.changed();
    return this.project(topic);
  };
  createTopicWithMember: Platform["createTopicWithMember"] = async (
    signal,
    input,
    member
  ) => {
    const topic = await this.createTopic(signal, input);
    return this.addTopicEntity(signal, topic.id, member);
  };
  saveTopic: Platform["saveTopic"] = async (signal, topic) => {
    Object.assign(this.topic(topic.id), structuredClone(topic));
    this.changed();
    return this.readTopic(signal, topic.id);
  };
  deleteTopic: Platform["deleteTopic"] = async (_signal, id) => {
    this.data.topics = this.data.topics.filter((t) => t.id !== id);
    this.changed();
  };
  addTopicEntity: Platform["addTopicEntity"] = async (signal, id, entity) => {
    const topic = this.topic(id);
    if (!topic.members.some((m) => m.id === entity.id))
      topic.members.push({
        __kind: entity.__kind,
        id: entity.id,
        ...("reference" in entity ? { articleId: entity.reference } : {}),
      });
    this.changed();
    return this.readTopic(signal, id);
  };
  removeTopicEntity: Platform["removeTopicEntity"] = async (
    signal,
    id,
    entityId
  ) => {
    const t = this.topic(id);
    t.members = t.members.filter((m) => m.id !== entityId);
    this.changed();
    return this.readTopic(signal, id);
  };
  listEntryTopics: Platform["listEntryTopics"] = async (_signal, id) => {
    if (
      this.scenario === "upgrade-read-failure" &&
      this.upgrade.state === "failed"
    )
      throw new Error("Fixture memberships unavailable");
    return this.data.topics
      .filter((t) => t.members.some((m) => m.id === id))
      .map((t) => t.id);
  };
  listTopicEntities: Platform["listTopicEntities"] = async (
    _signal,
    id,
    filter
  ) => {
    const ids = this.topic(id).members.map((m) => m.id);
    return this.filter(
      [
        ...this.data.articles.map((a) => this.project(a)),
        ...this.annotations(),
      ].filter((a) => ids.includes(a.id)),
      filter
    );
  };
  rankTopicsRelatedToEntity: Platform["rankTopicsRelatedToEntity"] =
    async () => [];
  listSites: Platform["listSites"] = async () =>
    [...new Set(this.data.articles.map((a) => new URL(a.url).hostname))].map(
      (hostname) => ({
        __kind: "site",
        id: hostname,
        title: hostname,
        ...this.counts(
          this.data.articles
            .filter((a) => new URL(a.url).hostname === hostname)
            .map((a) => a.id)
        ),
      })
    );
  listSiteEntries: Platform["listSiteEntries"] = async (
    _signal,
    hostname,
    filter
  ) =>
    this.filter(
      this.data.articles
        .filter((a) => new URL(a.url).hostname === hostname)
        .map((a) => this.project(a)),
      filter
    ) as Entry[];
  listPins: Platform["listPins"] = async () => structuredClone(this.pins);
  setPin: Platform["setPin"] = async (target, pinned) => {
    this.pins = this.pins.filter((p) => pinKey(p) !== pinKey(target));
    if (pinned) this.pins.push(structuredClone(target));
    this.emit("onPinChanged", { target, pinned });
    if (target.type === "entry")
      this.emit("onEntryPinChanged", this.project(this.article(target.id)));
  };
  setFeedPinned: Platform["setFeedPinned"] = async (id, pinned) =>
    this.setPin({ type: "feed", id }, pinned);
  setTopicPinned: Platform["setTopicPinned"] = async (id, pinned) =>
    this.setPin({ type: "topic", id }, pinned);
  setEntryPinned: Platform["setEntryPinned"] = async (id, pinned) => {
    await this.setPin({ type: "entry", id }, pinned);
    return this.project(this.article(id));
  };
  listPinnedEntries: Platform["listPinnedEntries"] = async () =>
    this.data.articles.map((a) => this.project(a)).filter((a) => a.isPinned);
  listTags: Platform["listTags"] = async () => structuredClone(this.data.tags);
  readTagStatus: Platform["readTagStatus"] = async () => ({
    tags: structuredClone(this.data.tags),
    renaming: [],
    deleting: [],
    merging: [],
  });
  createTag: Platform["createTag"] = async (name) => {
    const tag = { id: `capture-tag-${++this.sequence}`, name };
    this.data.tags.push(tag);
    this.emit("onTagCreated", tag);
    return structuredClone(tag);
  };
  readAnnotationTags: Platform["readAnnotationTags"] = async (
    _signal,
    entryId,
    annotationId
  ) => {
    if (!this.article(entryId).annotations.some((a) => a.id === annotationId))
      throw new Error("Annotation unavailable");
    return [...(this.data.assignments[annotationId] ?? [])];
  };
  setAnnotationTag: Platform["setAnnotationTag"] = async (
    entryId,
    annotationId,
    tagId,
    assigned
  ) => {
    await this.readAnnotationTags(
      new AbortController().signal,
      entryId,
      annotationId
    );
    const ids = (this.data.assignments[annotationId] ?? []).filter(
      (id) => id !== tagId
    );
    if (assigned) ids.push(tagId);
    this.data.assignments[annotationId] = ids;
    const change = { entryId, annotationId, tagId, assigned };
    this.emit("onAnnotationTagsChanged", change);
    this.changed();
    return change;
  };
  onAnnotationTagsChanged: Platform["onAnnotationTagsChanged"] = (callback) =>
    this.subscribe("onAnnotationTagsChanged", callback);
  readEntryTags: Platform["readEntryTags"] = async (_signal, id) => [
    ...(this.data.assignments[id] ?? []),
  ];
  setEntryTag: Platform["setEntryTag"] = async (entryId, tagId, assigned) => {
    this.article(entryId);
    const ids = (this.data.assignments[entryId] ?? []).filter(
      (id) => id !== tagId
    );
    if (assigned) ids.push(tagId);
    this.data.assignments[entryId] = ids;
    const change = { entryId, tagId, assigned };
    this.emit("onEntryTagsChanged", change);
    this.changed();
    return change;
  };
  listTagEntities: Platform["listTagEntities"] = async (
    signal,
    ids,
    filter
  ) => {
    const entities = await this.listAllEntities(
      signal,
      ids.length && filter === "unread" ? "all" : (filter ?? "unread")
    );
    return entities.filter((entity) =>
      ids.every((id) => this.data.assignments[entity.id]?.includes(id))
    );
  };
  countTagUnread: Platform["countTagUnread"] = async (signal, ids) => {
    const entities = await this.listAllEntities(signal, "unread");
    return entities.filter((entity) =>
      ids.every((id) => this.data.assignments[entity.id]?.includes(id))
    ).length;
  };
  renameTag: Platform["renameTag"] = async (id, name) => {
    const tag = this.data.tags.find((t) => t.id === id);
    if (!tag) return this.unsupported("Unknown tag");
    tag.name = name;
    const snapshot = await this.readTagStatus(new AbortController().signal);
    this.emit("onTagsChanged", snapshot);
    return snapshot;
  };
  deleteTag: Platform["deleteTag"] = async (id) => {
    this.data.tags = this.data.tags.filter((t) => t.id !== id);
    Object.keys(this.data.assignments).forEach((key) => {
      this.data.assignments[key] = this.data.assignments[key].filter(
        (t) => t !== id
      );
    });
    const snapshot = await this.readTagStatus(new AbortController().signal);
    this.emit("onTagsChanged", snapshot);
    this.changed();
    return snapshot;
  };
  getTheme: Platform["getTheme"] = async () => this.theme;
  setTheme: Platform["setTheme"] = async (theme) => {
    this.theme = theme;
    if (["refresh-failure", "refresh-all-fail"].includes(this.scenario ?? "")) {
      this.data.feeds.push({
        ...this.data.feeds[0],
        id: "workshop",
        title: "Workshop",
        url: additionalFeedUrl,
        members: [],
      });
    }
    if (this.scenario === "refresh-many-failures") {
      for (let i = 1; i < 14; i++)
        this.data.feeds.push({
          ...this.data.feeds[0],
          id: `failed-${i}`,
          title: `Additional feed ${i}`,
          url: `https://feed-${i}.example/feed.xml`,
          members: [],
        });
    }
    if (localStorage.getItem("xaga-capture-sidebar-sort") === "alphabetical")
      this.sidebarSort = "alphabetical";
    this.emit("onThemeChangedFromSettings", theme);
  };
  getSidebarSort: Platform["getSidebarSort"] = async () => this.sidebarSort;
  setSidebarSort: Platform["setSidebarSort"] = async (value) => {
    localStorage.setItem("xaga-capture-sidebar-sort", value);
    this.sidebarSort = value;
    this.emit("onSidebarSortChanged", value);
  };
  onSidebarSortChanged: Platform["onSidebarSortChanged"] = (callback) =>
    this.subscribe("onSidebarSortChanged", callback);
  getShowSidebar: Platform["getShowSidebar"] = async () => this.sidebar;
  setShowSidebar: Platform["setShowSidebar"] = async (show) => {
    this.sidebar = show;
    this.emit("onShowSidebar", show);
  };
  getHtmlReaderView: Platform["getHtmlReaderView"] = async () => this.view;
  setHtmlReaderView: Platform["setHtmlReaderView"] = async (view) => {
    this.view = view;
    this.emit("onHtmlReaderViewChangedFromSettings", view);
  };
  getHtmlReaderPrefs: Platform["getHtmlReaderPrefs"] = async () => ({
    ...this.prefs,
  });
  setHtmlReaderFontFamily: Platform["setHtmlReaderFontFamily"] = async (
    value
  ) => {
    this.prefs.fontFamily = value;
    this.emit("onHtmlReaderFontFamilyChanged", value);
  };
  setHtmlReaderFontSize: Platform["setHtmlReaderFontSize"] = async (value) => {
    this.prefs.fontSize = value;
    this.emit("onHtmlReaderFontSizeChanged", value);
  };
  setHtmlReaderLineHeight: Platform["setHtmlReaderLineHeight"] = async (
    value
  ) => {
    this.prefs.lineHeight = value;
    this.emit("onHtmlReaderLineHeightChanged", value);
  };
  getRefreshOnStartup: Platform["getRefreshOnStartup"] = async () =>
    this.refreshOnStartup || this.scenario === "startup-refresh-failure";
  getInlinePdfReader: Platform["getInlinePdfReader"] = async () => false;
  getLoadRemoteImage: Platform["getLoadRemoteImage"] = async () => false;
  getFetchSettings: Platform["getFetchSettings"] = async () =>
    structuredClone(this.fetchSettings);
  setFetchSetting: Platform["setFetchSetting"] = async (change) => {
    if (
      this.scenario === "settings-save-failure" &&
      this.settingsAttempts++ === 0
    )
      throw {
        code: "settings:busy",
        message: "Another Xaga instance is busy. Please try again.",
        detail: "Synthetic settings refusal",
      };
    this.fetchSettings = {
      ...this.fetchSettings,
      [change.setting]: change.value,
    };
  };
  getImportMaxBytes: Platform["getImportMaxBytes"] = async () =>
    this.importMaxBytes;
  getAllowedDomains: Platform["getAllowedDomains"] = async () => [];
  onError: Platform["onError"] = (callback) =>
    this.subscribe("onError", callback);
  onWindowCloseRequested: Platform["onWindowCloseRequested"] = (callback) =>
    this.subscribe("onWindowCloseRequested", callback);
  openFileDialog: Platform["openFileDialog"] = async () =>
    this.unsupported("Library selection");
  openUrl: Platform["openUrl"] = async () =>
    this.unsupported("Opening external links");
  setLibrary: Platform["setLibrary"] = async () =>
    this.unsupported("Library selection");
  refreshAllFeeds: Platform["refreshAllFeeds"] = async () => {
    await new Promise((resolve) => setTimeout(resolve, 100));
    this.refreshAttempts += 1;
    if (this.scenario === "refresh-refused")
      throw {
        code: "library:read-only",
        message: "This library is read-only.",
        detail: "Synthetic refusal",
      };
    const failures =
      this.refreshAttempts === 1 &&
      [
        "refresh-failure",
        "startup-refresh-failure",
        "refresh-all-fail",
        "refresh-many-failures",
      ].includes(this.scenario ?? "")
        ? this.data.feeds
            .filter(
              (_feed, index) =>
                this.scenario === "refresh-all-fail" ||
                this.scenario === "refresh-many-failures" ||
                index === 0
            )
            .map((feed) => ({
              id: feed.id,
              title: feed.title,
              code: "feed:http" as const,
              message: "Couldn't reach this feed.",
            }))
        : [];
    const feeds =
      this.scenario === "refresh-all-fail" && failures.length
        ? []
        : this.data.feeds.filter(
            (feed) => !failures.some((failure) => failure.id === feed.id)
          );
    feeds.forEach((feed) => this.emit("onFeedRefreshed", feed));
    this.emit("onFeedsRefreshed", undefined);
    return { feeds, failures };
  };
  openStoredPdf: Platform["openStoredPdf"] = async () =>
    this.unsupported("Opening PDFs");
  readStoredPdf: Platform["readStoredPdf"] = async () => null;
  loadArticle: Platform["loadArticle"] = async () =>
    this.unsupported("Downloading articles");
  createArticle: Platform["createArticle"] = async () =>
    this.unsupported("Creating articles from URLs");
  // No similarity relationships are supplied by this fixture dataset.
  queryByEntity: Platform["queryByEntity"] = async () => [];
  queryByText: Platform["queryByText"] = async () =>
    this.unsupported("Text search");
  setImportMaxBytes: Platform["setImportMaxBytes"] = async (value) => {
    this.importMaxBytes = value;
  };
  setRefreshOnStartup: Platform["setRefreshOnStartup"] = async (value) => {
    this.refreshOnStartup = value;
  };
  onOpenLibrary: Platform["onOpenLibrary"] = (callback) =>
    this.subscribe("onOpenLibrary", callback);
  onOpenSettings: Platform["onOpenSettings"] = (callback) =>
    this.subscribe("onOpenSettings", callback);
  onFeedsImported: Platform["onFeedsImported"] = (callback) =>
    this.subscribe("onFeedsImported", callback);
  onFeedsExported: Platform["onFeedsExported"] = (callback) =>
    this.subscribe("onFeedsExported", callback);
  onBookmarksImported: Platform["onBookmarksImported"] = (callback) =>
    this.subscribe("onBookmarksImported", callback);
  onBookmarksExported: Platform["onBookmarksExported"] = (callback) =>
    this.subscribe("onBookmarksExported", callback);
  onFeedRefreshed: Platform["onFeedRefreshed"] = (callback) =>
    this.subscribe("onFeedRefreshed", callback);
  onFeedsRefreshed: Platform["onFeedsRefreshed"] = (callback) =>
    this.subscribe("onFeedsRefreshed", callback);
  onLibraryModeChanged: Platform["onLibraryModeChanged"] = (callback) =>
    this.subscribe("onLibraryModeChanged", callback);
  onUnreadChanged: Platform["onUnreadChanged"] = (callback) =>
    this.subscribe("onUnreadChanged", callback);
  onBookmarkChanged: Platform["onBookmarkChanged"] = (callback) =>
    this.subscribe("onBookmarkChanged", callback);
  onAnnotationChanged: Platform["onAnnotationChanged"] = (callback) =>
    this.subscribe("onAnnotationChanged", callback);
  onAnnotationReplaced: Platform["onAnnotationReplaced"] = (callback) =>
    this.subscribe("onAnnotationReplaced", callback);
  onAnnotationRemoved: Platform["onAnnotationRemoved"] = (callback) =>
    this.subscribe("onAnnotationRemoved", callback);
  onFaviconChanged: Platform["onFaviconChanged"] = (callback) =>
    this.subscribe("onFaviconChanged", callback);
  onEmbeddingsChanged: Platform["onEmbeddingsChanged"] = (callback) =>
    this.subscribe("onEmbeddingsChanged", callback);
  onSetTheme: Platform["onSetTheme"] = (callback) =>
    this.subscribe("onSetTheme", callback);
  onShowSidebar: Platform["onShowSidebar"] = (callback) =>
    this.subscribe("onShowSidebar", callback);
  onLoaderClosed: Platform["onLoaderClosed"] = (callback) =>
    this.subscribe("onLoaderClosed", callback);
  onLoaderPageLoaded: Platform["onLoaderPageLoaded"] = (callback) =>
    this.subscribe("onLoaderPageLoaded", callback);
  onFind: Platform["onFind"] = (callback) => this.subscribe("onFind", callback);
  onShortcutMenuSelected: Platform["onShortcutMenuSelected"] = (callback) =>
    this.subscribe("onShortcutMenuSelected", callback);
  onShutdown: Platform["onShutdown"] = (callback) =>
    this.subscribe("onShutdown", callback);
  onClearHistory: Platform["onClearHistory"] = (callback) =>
    this.subscribe("onClearHistory", callback);
  clearWebviewHistory: Platform["clearWebviewHistory"] = async () =>
    this.unsupported("Clearing native browser history");
  setAllowedDomains: Platform["setAllowedDomains"] = async () =>
    this.unsupported("Privacy settings");
  onAllowedDomainsChanged: Platform["onAllowedDomainsChanged"] = (callback) =>
    this.subscribe("onAllowedDomainsChanged", callback);
  onHtmlReaderView: Platform["onHtmlReaderView"] = (callback) =>
    this.subscribe("onHtmlReaderView", callback);
  onHtmlReaderViewChangedFromSettings: Platform["onHtmlReaderViewChangedFromSettings"] =
    (callback) =>
      this.subscribe("onHtmlReaderViewChangedFromSettings", callback);
  onEntryTagsChanged: Platform["onEntryTagsChanged"] = (callback) =>
    this.subscribe("onEntryTagsChanged", callback);
  mergeTags: Platform["mergeTags"] = async (sourceId, destinationId) => {
    const destination = this.data.tags.find((tag) => tag.id === destinationId);
    if (
      !destination ||
      sourceId === destinationId ||
      !this.data.tags.some((tag) => tag.id === sourceId)
    )
      return this.unsupported("Unknown tag merge");
    Object.keys(this.data.assignments).forEach((id) => {
      this.data.assignments[id] = [
        ...new Set(
          this.data.assignments[id].map((tag) =>
            tag === sourceId ? destinationId : tag
          )
        ),
      ];
    });
    this.data.tags = this.data.tags.filter((tag) => tag.id !== sourceId);
    const snapshot = await this.readTagStatus(new AbortController().signal);
    this.emit("onTagsChanged", {
      ...snapshot,
      merging: [
        {
          sourceId,
          destinationId,
          survivorId: destinationId,
          destinationName: destination.name,
        },
      ],
    });
    this.emit("onTagsChanged", snapshot);
    this.changed();
    return snapshot;
  };
  onTagsChanged: Platform["onTagsChanged"] = (callback) =>
    this.subscribe("onTagsChanged", callback);
  onTagCreated: Platform["onTagCreated"] = (callback) =>
    this.subscribe("onTagCreated", callback);
  organizePins: Platform["organizePins"] = async () =>
    this.unsupported("Shortcut groups");
  onPinChanged: Platform["onPinChanged"] = (callback) =>
    this.subscribe("onPinChanged", callback);
  onEntryPinChanged: Platform["onEntryPinChanged"] = (callback) =>
    this.subscribe("onEntryPinChanged", callback);
  setLoadRemoteImage: Platform["setLoadRemoteImage"] = async () =>
    this.unsupported("Remote image settings");
  setInlinePdfReader: Platform["setInlinePdfReader"] = async () =>
    this.unsupported("PDF settings");
  onInlinePdfReaderChanged: Platform["onInlinePdfReaderChanged"] = (callback) =>
    this.subscribe("onInlinePdfReaderChanged", callback);
  onThemeChangedFromSettings: Platform["onThemeChangedFromSettings"] = (
    callback
  ) => this.subscribe("onThemeChangedFromSettings", callback);
  onHtmlReaderFontFamilyChanged: Platform["onHtmlReaderFontFamilyChanged"] = (
    callback
  ) => this.subscribe("onHtmlReaderFontFamilyChanged", callback);
  onHtmlReaderFontSizeChanged: Platform["onHtmlReaderFontSizeChanged"] = (
    callback
  ) => this.subscribe("onHtmlReaderFontSizeChanged", callback);
  onHtmlReaderLineHeightChanged: Platform["onHtmlReaderLineHeightChanged"] = (
    callback
  ) => this.subscribe("onHtmlReaderLineHeightChanged", callback);
  onRefreshOnStartupChanged: Platform["onRefreshOnStartupChanged"] = (
    callback
  ) => this.subscribe("onRefreshOnStartupChanged", callback);
}
