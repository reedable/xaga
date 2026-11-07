import type { ShortcutNode } from "../../utils/projectPinTree";
import { act, forwardRef, useEffect, useImperativeHandle } from "react";
import { createRoot } from "react-dom/client";
import { MemoryRouter } from "react-router-dom";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import useRouter from "../../hooks/useRouter";
import type { Article, Feed, Topic, ListEntity, NavigationState, Site } from "../../types";
import { ALL_ENTITIES } from "../../Shortcuts";
import { emptySite } from "../../sites";
import { EntityList } from "./EntityList";

const mocks = vi.hoisted(() => {
  const callbacks: Record<string, (value: never) => void> = {};
  const register = (name: string) => (callback: (value: never) => void) => {
    callbacks[name] = callback;
    return () => {
      delete callbacks[name];
    };
  };
  return {
    callbacks,
    events: Object.fromEntries(
      [
        "onFeedsRefreshed",
        "onUnreadChanged",
        "onBookmarkChanged",
        "onAnnotationChanged",
        "onAnnotationReplaced",
        "onAnnotationRemoved",
        "onEmbeddingsChanged",
      ].map((name) => [name, register(name)])
    ),
    tagEntries: vi.fn(),
    tagContext: { tags: [{ id: "tag-a", name: "Research" }, { id: "tag-b", name: "Work" }], hasLoaded: true, unreadCount: 1 },
    read: vi.fn(),
    sink: { pushError: vi.fn() },
    entries: {
      listEntries: vi.fn(),
      listAllEntities: vi.fn(),
      markEntryRead: vi.fn(),
    },
    topics: { topics: [] as Topic[], listTopicEntities: vi.fn() },
    sites: {
      sites: [] as Site[],
      pinnedSites: [] as Site[],
      listSiteEntries: vi.fn(),
    },
    editSite: vi.fn(),
    editFeed: vi.fn(),
    editTopic: vi.fn(),
    feeds: { feeds: [] as Feed[], loading: false, refreshAllFeeds: vi.fn() },
    pins: [] as Article[],
    shortcutHierarchy: [] as ShortcutNode[],
  };
});
vi.mock("../../platform/Platform", () => ({ usePlatform: () => platform }));
const platform = { readArticleScope: mocks.read, listTagEntries: mocks.tagEntries };
vi.mock("../../providers/TagsContext", () => ({ useTags: () => mocks.tagContext }));
vi.mock("../../hooks/useErrorSink", () => ({ useErrorSink: () => mocks.sink }));
vi.mock("../../hooks/useEntries", () => ({ default: () => mocks.entries }));
vi.mock("../../hooks/useTopics", () => ({ useTopics: () => mocks.topics }));
vi.mock("../../hooks/useSites", () => ({ useSites: () => mocks.sites }));
vi.mock("../../hooks/useFeeds", () => ({ useFeeds: () => mocks.feeds }));
vi.mock("../../hooks/usePinnedEntries", () => ({
  usePinnedEntries: () => mocks.pins,
}));
vi.mock("../../hooks/useShortcutTargets", () => ({ useShortcutTargets: () => ({ hierarchy: mocks.shortcutHierarchy, targets: mocks.pins }) }));
vi.mock("../../hooks/useEvents", () => ({ useEvents: () => mocks.events }));
vi.mock("../EntityStatus", () => ({
  default: ({ entity }: { entity: { id: string; isUnread?: boolean } }) => (
    <span data-status-id={entity.id} data-unread={entity.isUnread} />
  ),
}));
vi.mock("react-virtuoso", () => ({
  Virtuoso: forwardRef(function Rows(
    {
      data,
      itemContent,
    }: {
      data: ListEntity[];
      itemContent: (index: number, entity: ListEntity) => React.ReactNode;
    },
    ref
  ) {
    useImperativeHandle(ref, () => ({
      scrollIntoView: ({ done }: { done: () => void }) => done(),
    }));
    return (
      <div>
        {data.map((entity, index) => (
          <div key={entity.id}>{itemContent(index, entity)}</div>
        ))}
      </div>
    );
  }),
}));
vi.mock("../TagActions", () => ({ TagActions: ({ tag }: { tag: { name: string } }) => <button data-tag-action aria-label={`Actions for ${tag.name}`} /> }));
vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
let route: ReturnType<typeof useRouter>;
function Observe() {
  const current = useRouter();
  useEffect(() => {
    route = current;
  }, [current]);
  return null;
}
let container: HTMLDivElement;
let root: ReturnType<typeof createRoot>;
let article: Article;
const rows = () => [
  ...container.querySelectorAll<HTMLElement>(".MuiListItemButton-root"),
];
async function render(
  state: NavigationState = { articleScopeId: "article", entryId: "article" },
  selectable = false
) {
  await act(async () => {
    root.render(
      <MemoryRouter initialEntries={[{ pathname: "/", state }]}>
        <Observe />
        <EntityList
          isSelectableHeader={selectable}
          onEditFeed={mocks.editFeed}
          onEditTopic={mocks.editTopic}
          onEditSite={mocks.editSite}
        />
      </MemoryRouter>
    );
  });
}
beforeEach(() => {
  vi.clearAllMocks();
  article = {
    __kind: "entry",
    id: "article",
    title: "Pinned article",
    url: "https://example.org/a",
    createdAt: "2026-10-29",

    isUnread: true,
    isPinned: true,
    content: "First passage. Last passage.",
    annotations: [
      {
        __kind: "annotation",
        id: "late",
        title: "Last",
        text: ["Last"],
        occurrence: [0],
        reference: "article",
        color: "yellow",
        createdAt: "2026-10-29",
        content: "",
      },
      {
        __kind: "annotation",
        id: "early",
        title: "First",
        text: ["First"],
        occurrence: [0],
        reference: "article",
        color: "yellow",
        createdAt: "2026-10-29",
        content: "",
      },
    ],
  };
  mocks.tagContext.tags = [{ id: "tag-a", name: "Research" }, { id: "tag-b", name: "Work" }];
  mocks.tagEntries.mockResolvedValue([article]);
  mocks.pins = [article];
  mocks.shortcutHierarchy = [];
  mocks.feeds.feeds = [];
  mocks.topics.topics = [];
  mocks.sites.sites = [];
  mocks.sites.pinnedSites = [];
  mocks.entries.listAllEntities.mockResolvedValue([]);
  mocks.read.mockImplementation(async () => structuredClone(article));
  container = document.createElement("div");
  document.body.append(container);
  root = createRoot(container);
});
afterEach(async () => {
  await act(async () => root.unmount());
  container.remove();
});

it("keeps the article scope while selecting annotations and moving with the keyboard", async () => {
  await render();
  expect(rows().map((row) => row.textContent)).toEqual([
    "Pinned article",
    "First",
    "Last",
  ]);
  expect(container.querySelector('[aria-label="Filter"]')).toBeNull();
  expect(container.textContent).not.toContain("Mark all read");
  await act(async () => rows()[1].click());
  expect(route.state).toMatchObject({
    articleScopeId: "article",
    entryId: "article",
    annotationId: "early",
  });
  expect(rows()[1].classList.contains("Mui-selected")).toBe(true);
  expect(rows()[0].classList.contains("Mui-selected")).toBe(false);
  await act(async () =>
    rows()[1].dispatchEvent(
      new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true })
    )
  );
  expect(route.state.annotationId).toBe("late");
  expect(document.activeElement).toBe(rows()[2]);
  expect(mocks.read).toHaveBeenCalledTimes(1);
});

it("preserves the open list and hidden-sidebar header after unpinning or marking read", async () => {
  await render(undefined, true);
  mocks.pins = [];
  await act(async () =>
    mocks.callbacks.onUnreadChanged({
      ...article,
      isUnread: false,
      isPinned: false,
    } as never)
  );
  expect(rows()).toHaveLength(3);
  expect(route.state.articleScopeId).toBe("article");
  expect(container.querySelector('[role="combobox"]')?.textContent).toBe(
    "Pinned article"
  );
  expect(mocks.read).toHaveBeenCalledTimes(1);
});

it("reflects annotation additions, reordered edits, replacements and removals from the authoritative article", async () => {
  await render();
  article.annotations.push({
    ...article.annotations[0],
    id: "new",
    title: "passage",
    text: ["passage"],
    occurrence: [0],
  });
  await act(async () =>
    mocks.callbacks.onAnnotationChanged({
      ...article.annotations[2],
      url: article.url,
    } as never)
  );
  expect(rows().map((row) => row.textContent)).toEqual([
    "Pinned article",
    "First",
    "passage",
    "Last",
  ]);
  article.annotations[1] = {
    ...article.annotations[1],
    title: "Last passage",
    text: ["Last passage"],
  };
  await act(async () =>
    mocks.callbacks.onAnnotationChanged({
      ...article.annotations[1],
      url: article.url,
    } as never)
  );
  expect(rows()[1].textContent).toBe("passage");
  await act(async () => rows()[3].click());
  const oldId = article.annotations[1].id;
  article.annotations[1] = { ...article.annotations[1], id: "replacement" };
  await act(async () =>
    mocks.callbacks.onAnnotationReplaced({
      oldId,
      entity: { ...article.annotations[1], url: article.url },
    } as never)
  );
  expect(route.state.annotationId).toBe("replacement");
  article.annotations = article.annotations.filter(
    (annotation) => annotation.id !== "replacement"
  );
  await act(async () =>
    mocks.callbacks.onAnnotationRemoved("replacement" as never)
  );
  expect(rows()).toHaveLength(3);
});

it("keeps a reading-state event that arrives while an older article snapshot is loading", async () => {
  let resolve!: (value: Article) => void;
  const snapshot = structuredClone(article);
  mocks.read.mockImplementationOnce(
    () =>
      new Promise<Article>((done) => {
        resolve = done;
      })
  );
  await render();
  await act(async () =>
    mocks.callbacks.onUnreadChanged({ ...article, isUnread: false } as never)
  );
  await act(async () => resolve(snapshot));
  expect(rows()).toHaveLength(3);
  expect(
    container
      .querySelector('[data-status-id="article"]')
      ?.getAttribute("data-unread")
  ).toBe("false");
  expect(route.state.articleScopeId).toBe("article");
  expect(mocks.read).toHaveBeenCalledTimes(1);
});

it("filters article-scope rows without changing the open list target", async () => {
  await render();
  const input = container.querySelector<HTMLInputElement>(
    'input[placeholder="Filter items…"]'
  )!;
  const setValue = Object.getOwnPropertyDescriptor(
    HTMLInputElement.prototype,
    "value"
  )!.set!;
  await act(async () => {
    setValue.call(input, "Last");
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
  expect(rows().map((row) => row.textContent)).toEqual(["Last"]);
  expect(route.state.articleScopeId).toBe("article");
  await act(async () => {
    setValue.call(input, "");
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
  expect(rows()).toHaveLength(3);
  expect(mocks.read).toHaveBeenCalledTimes(1);
});

vi.mock("../../hooks/useMissingPins", () => ({ useMissingPins: () => [] }));

it("preserves a Site's open Saved list when article color is removed", async () => {
  mocks.sites.sites = [
    {
      id: "example.org",
      __kind: "site" as const,
      title: "example.org",
      entryCount: 1,
      unreadCount: 0,
    },
  ] as never;
  const marked = { ...article, color: "blue" };
  mocks.sites.listSiteEntries.mockResolvedValue([marked]);
  await render({ siteHostname: "example.org", filter: "saved" });
  expect(mocks.sites.listSiteEntries).toHaveBeenCalledOnce();
  const before = rows().map((row) => row.textContent);
  await act(async () =>
    mocks.callbacks.onBookmarkChanged({ ...marked, color: undefined } as never)
  );
  expect(rows().map((row) => row.textContent)).toEqual(before);
  expect(mocks.sites.listSiteEntries).toHaveBeenCalledOnce();
  expect(route.state.siteHostname).toBe("example.org");
});

it("keeps an empty Site selected when unpinned and opens its Settings", async () => {
  const site = {
    id: "empty.example",
    __kind: "site" as const,
    title: "empty.example",
    entryCount: 0,
    unreadCount: 0,
  };
  mocks.sites.pinnedSites = [site];
  mocks.sites.listSiteEntries.mockResolvedValue([]);
  const state = { siteHostname: site.id };
  await render(state);
  expect(mocks.sites.listSiteEntries).toHaveBeenCalledWith(
    site.id,
    undefined
  );
  const settings =
    container.querySelector<HTMLButtonElement>('button[aria-label="Edit"]') ??
    container.querySelector<HTMLButtonElement>('button[title="Edit"]');
  // MUI Tooltip supplies the accessible name on its child button.
  expect(settings).not.toBeNull();
  await act(async () => settings!.click());
  expect(mocks.editSite).toHaveBeenCalledWith(site);
  mocks.sites.pinnedSites = [];
  await render(state);
  expect(route.state.siteHostname).toBe(site.id);
  expect(container.textContent).toContain(site.id);
  expect(mocks.entries.listAllEntities).not.toHaveBeenCalled();
});

it.each(["feed", "topic", "site"] as const)("selects a %s from the responsive header, loads its members, and opens only its own editor", async (kind) => {
  const feed: Feed = {
    __kind: "feed", id: "feed", title: "Feed", createdAt: "2026-01-01",
    url: "https://example.org/feed", members: [], isPinned: false, entryCount: 1, unreadCount: 0,
  };
  const topic: Topic = { ...feed, __kind: "topic", id: "topic", title: "Topic" };
  const site = emptySite("example.org");
  mocks.feeds.feeds = [feed];
  mocks.topics.topics = [topic];
  mocks.sites.sites = [site];
  mocks.entries.listEntries.mockResolvedValue([article]);
  mocks.topics.listTopicEntities.mockResolvedValue([article]);
  mocks.sites.listSiteEntries.mockResolvedValue([article]);
  const collection = { feed, topic, site }[kind];
  const scope = { feed: { feedId: feed.id }, topic: { topicId: topic.id }, site: { siteHostname: site.id } }[kind];

  await render({ shortcutId: ALL_ENTITIES.id, filter: "saved" }, true);
  const openHeader = async () => {
    const combo = container.querySelector<HTMLElement>('[aria-label="Browse collections and tags"]')!;
    await act(async () => combo.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, button: 0 })));
  };
  const option = (title: string) => [...document.querySelectorAll<HTMLElement>('[role="option"]')].find((entity) => entity.textContent === title)!;
  await openHeader();
  await act(async () => option(collection.title).click());
  expect(route.state).toEqual({ ...scope, filter: "saved" });
  expect(rows().map((row) => row.textContent)).toEqual([article.title]);
  if (kind === "feed") expect(mocks.entries.listEntries).toHaveBeenCalledWith(feed, "saved");
  if (kind === "topic") expect(mocks.topics.listTopicEntities).toHaveBeenCalledWith(topic.id, "saved", "relevance");
  if (kind === "site") expect(mocks.sites.listSiteEntries).toHaveBeenCalledWith(site.id, "saved");

  await render(route.state);
  await act(async () => container.querySelector<HTMLButtonElement>('button[aria-label="Edit"]')!.click());
  for (const [editorKind, editor] of Object.entries({ feed: mocks.editFeed, topic: mocks.editTopic, site: mocks.editSite })) {
    if (editorKind === kind) expect(editor).toHaveBeenCalledExactlyOnceWith(collection);
    else expect(editor).not.toHaveBeenCalled();
  }

  await render(route.state, true);
  await openHeader();
  await act(async () => option("All items").click());
  expect(route.state).toEqual({ shortcutId: ALL_ENTITIES.id, filter: "saved" });
  expect(mocks.entries.listAllEntities).toHaveBeenLastCalledWith("saved");
});

it("browses Entry intersections and retains membership through state and assignment updates", async () => {
  await render({ tagIds: ["tag-a", "tag-b"], filter: "unread" });
  expect(mocks.tagEntries).toHaveBeenCalledWith(expect.any(AbortSignal), ["tag-a", "tag-b"], "all");
  expect(container.textContent).toContain("Research + Work");
  expect(container.querySelector('[aria-label="Unread"]')).toBeNull();
  expect(container.querySelector('[aria-label="All"]')?.getAttribute("aria-pressed")).toBe("true");
  await act(async () => rows()[0].click());
  expect(route.state.tagIds).toEqual(["tag-a", "tag-b"]);
  await act(async () => {
    mocks.callbacks.onUnreadChanged({ ...article, isUnread: false } as never);
    mocks.callbacks.onBookmarkChanged({ ...article, color: undefined } as never);
    mocks.callbacks.onEmbeddingsChanged(undefined as never);
    mocks.callbacks.onFeedsRefreshed(undefined as never);
  });
  expect(rows()).toHaveLength(1);
  expect(mocks.tagEntries).toHaveBeenCalledOnce();
  await act(async () => route.selectFilter("saved"));
  expect(mocks.tagEntries).toHaveBeenCalledTimes(2);
  expect(mocks.tagEntries).toHaveBeenLastCalledWith(expect.any(AbortSignal), ["tag-a", "tag-b"], "saved");
});

it("shares the tag set with the hidden-sidebar header and immediately intersects each choice", async () => {
  await render({ tagIds: [], filter: "all" }, true);
  expect(mocks.tagEntries).toHaveBeenLastCalledWith(expect.any(AbortSignal), [], "all");
  expect(container.querySelector('[aria-label="Unread"]')).not.toBeNull();
  const combo = container.querySelector<HTMLElement>('[role="combobox"]')!;
  expect(combo.getAttribute("aria-label")).toBe("Browse collections and tags");
  await act(async () => combo.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, button: 0 })));
  const option = (name: string) => [...document.querySelectorAll<HTMLElement>('[role="option"]')].find((entity) => entity.textContent === name)!;
  await act(async () => option("Research").click());
  expect(route.state.tagIds).toEqual(["tag-a"]);
  await act(async () => option("Work").click());
  expect(route.state.tagIds).toEqual(["tag-a", "tag-b"]);
  expect(mocks.tagEntries).toHaveBeenLastCalledWith(expect.any(AbortSignal), ["tag-a", "tag-b"], "all");
  expect(option("Research").getAttribute("aria-selected")).toBe("true");
  await act(async () => option("Research").click());
  expect(route.state.tagIds).toEqual(["tag-b"]);
  await act(async () => option("Work").click());
  expect(route.state.tagIds).toEqual([]);
});

it("updates renamed or missing labels without reloading the active list", async () => {
  await render({ tagIds: ["tag-a"], filter: "all" });
  mocks.tagContext.tags = [{ id: "tag-a", name: "Renamed" }];
  await act(async () => route.selectEntity(article));
  expect(container.textContent).toContain("Renamed");
  expect(mocks.tagEntries).toHaveBeenCalledOnce();
  mocks.tagContext.tags = [];
  await act(async () => route.selectFilter("saved"));
  expect(container.textContent).toContain("Unavailable tag");
  expect(route.state.tagIds).toEqual(["tag-a"]);
});

it("ignores a superseded tag query and refreshes membership when text filtering changes", async () => {
  let oldResult!: (value: ListEntity[]) => void;
  mocks.tagEntries.mockImplementationOnce(() => new Promise((resolve) => { oldResult = resolve; }));
  await render({ tagIds: ["tag-a"], filter: "all" });
  await act(async () => route.selectTags(["tag-b"]));
  await act(async () => oldResult([{ ...article, id: "old", title: "Old result" }]));
  expect(rows().map((row) => row.textContent)).toEqual(["Pinned article"]);
  const input = container.querySelector<HTMLInputElement>('input[placeholder="Filter items…"]')!;
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, "Pinned");
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
  expect(mocks.tagEntries).toHaveBeenCalledTimes(3);
  expect(rows().map((row) => row.textContent)).toEqual(["Pinned article"]);
});

it("provides Tab access to tag actions inside the responsive selector without selecting the tag", async () => {
  await render({ tagIds: ["tag-a"], filter: "all" }, true);
  const combo = container.querySelector<HTMLElement>('[role="combobox"]')!;
  await act(async () => combo.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, button: 0 })));
  const option = [...document.querySelectorAll<HTMLElement>('[role="option"]')].find((entity) => entity.textContent === "Research")!;
  await act(async () => {
    option.focus();
    option.dispatchEvent(new KeyboardEvent("keydown", { key: "Tab", bubbles: true, cancelable: true }));
  });
  expect(document.activeElement?.getAttribute("aria-label")).toBe("Actions for Research");
  expect(route.state.tagIds).toEqual(["tag-a"]);
});

it("removes deleted selections without refreshing membership until explicit navigation", async () => {
  await render({ tagIds: ["tag-a", "tag-b"], filter: "unread", entryId: article.id });
  expect(rows()).toHaveLength(1);
  await act(async () => route.replaceTagSelection(route.state.tagIds!.filter((id) => id !== "tag-a")));
  expect(route.state.tagIds).toEqual(["tag-b"]);
  expect(route.state.entryId).toBe(article.id);
  expect(mocks.tagEntries).toHaveBeenCalledOnce();
  await act(async () => route.replaceTagSelection(route.state.tagIds!.filter((id) => id !== "tag-b")));
  expect(route.state.tagIds).toEqual([]);
  expect(rows()).toHaveLength(1);
  expect(mocks.tagEntries).toHaveBeenCalledOnce();
  await act(async () => route.selectFilter("saved"));
  expect(mocks.tagEntries).toHaveBeenCalledTimes(2);
  expect(mocks.tagEntries).toHaveBeenLastCalledWith(expect.any(AbortSignal), [], "saved");
});

it("discards a pending membership result when deletion changes selection", async () => {
  let oldResult!: (value: ListEntity[]) => void;
  mocks.tagEntries.mockImplementationOnce(() => new Promise((resolve) => { oldResult = resolve; }));
  await render({ tagIds: ["tag-a"], filter: "all" });
  await act(async () => route.replaceTagSelection(route.state.tagIds!.filter((id) => id !== "tag-a")));
  await act(async () => oldResult([{ ...article, id: "old", title: "Old result" }]));
  expect(rows()).toHaveLength(0);
  expect(mocks.tagEntries).toHaveBeenCalledOnce();
  await act(async () => route.selectTags([]));
  // Explicitly selecting the already selected empty set must still refresh a retained list.
  expect(mocks.tagEntries).toHaveBeenCalledTimes(2);
});

it("retains the list and reader when merged selections collapse to the survivor", async () => {
  await render({ tagIds: ["tag-a", "tag-b"], filter: "all", entryId: article.id });
  await act(async () => route.replaceTagSelection(["tag-b"]));
  expect(rows()).toHaveLength(1);
  expect(route.state.entryId).toBe(article.id);
  expect(mocks.tagEntries).toHaveBeenCalledOnce();
  await act(async () => route.selectTags(["tag-b"]));
  expect(mocks.tagEntries).toHaveBeenCalledTimes(2);
  expect(mocks.tagEntries).toHaveBeenLastCalledWith(expect.any(AbortSignal), ["tag-b"], "all");
});

it("shows nested shortcut headings in the hidden-sidebar header and opens descendants by typed identity", async () => {
  mocks.shortcutHierarchy = [{ reference: { type: "group", id: "g" }, title: "Research", unreadCount: 0, children: [
    { reference: { type: "group", id: "nested" }, title: "Background", unreadCount: 0, children: [
      { reference: { type: "entry", id: article.id }, title: article.title, unreadCount: 0, target: { ...article, entryCount: 0, unreadCount: 0 } },
    ] },
  ] }];
  await render({ shortcutId: ALL_ENTITIES.id }, true);
  const combo = container.querySelector<HTMLElement>('[role="combobox"]')!;
  await act(async () => combo.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, button: 0 })));
  const list = document.querySelector('[role="listbox"]')!;
  expect(list.textContent).toContain("ResearchBackgroundPinned article");
  const option = [...list.querySelectorAll<HTMLElement>('[role="option"]')].find((item) => item.textContent === article.title)!;
  await act(async () => option.click());
  expect(route.state.articleScopeId).toBe(article.id);
  expect(route.state.entryId).toBe(article.id);
  await act(async () => combo.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, button: 0 })));
  const selected = [...document.querySelectorAll<HTMLElement>('[role="option"]')].find((item) => item.textContent === article.title)!;
  expect(selected.getAttribute("aria-selected")).toBe("true");
});
