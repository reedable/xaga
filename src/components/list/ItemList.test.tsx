import { act, forwardRef, useEffect, useImperativeHandle } from "react";
import { createRoot } from "react-dom/client";
import { MemoryRouter } from "react-router-dom";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import useRouter from "../../hooks/useRouter";
import type { Article, ListItem, NavigationState, Site } from "../../types";
import { ItemList } from "./ItemList";

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
    read: vi.fn(),
    sink: { pushError: vi.fn() },
    entries: {
      listEntries: vi.fn(),
      listAllItems: vi.fn(),
      markEntryRead: vi.fn(),
    },
    topics: { topics: [], listTopicItems: vi.fn() },
    sites: {
      sites: [] as Site[],
      pinnedSites: [] as Site[],
      listSiteEntries: vi.fn(),
    },
    editSite: vi.fn(),
    feeds: { feeds: [], loading: false, refreshAllFeeds: vi.fn() },
    pins: [] as Article[],
  };
});
vi.mock("../../platform/Platform", () => ({ usePlatform: () => platform }));
const platform = { readArticleScope: mocks.read };
vi.mock("../../hooks/useErrorSink", () => ({ useErrorSink: () => mocks.sink }));
vi.mock("../../hooks/useEntries", () => ({ default: () => mocks.entries }));
vi.mock("../../hooks/useTopics", () => ({ useTopics: () => mocks.topics }));
vi.mock("../../hooks/useSites", () => ({ useSites: () => mocks.sites }));
vi.mock("../../hooks/useFeeds", () => ({ useFeeds: () => mocks.feeds }));
vi.mock("../../hooks/usePinnedEntries", () => ({
  usePinnedEntries: () => mocks.pins,
}));
vi.mock("../../hooks/useEvents", () => ({ useEvents: () => mocks.events }));
vi.mock("../ItemStatus", () => ({
  default: ({ item }: { item: { id: string; isUnread?: boolean } }) => (
    <span data-status-id={item.id} data-unread={item.isUnread} />
  ),
}));
vi.mock("react-virtuoso", () => ({
  Virtuoso: forwardRef(function Rows(
    {
      data,
      itemContent,
    }: {
      data: ListItem[];
      itemContent: (index: number, item: ListItem) => React.ReactNode;
    },
    ref
  ) {
    useImperativeHandle(ref, () => ({
      scrollIntoView: ({ done }: { done: () => void }) => done(),
    }));
    return (
      <div>
        {data.map((item, index) => (
          <div key={item.id}>{itemContent(index, item)}</div>
        ))}
      </div>
    );
  }),
}));
vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
let route: ReturnType<typeof useRouter>;
function Observe() {
  const current = useRouter();
  useEffect(() => {
    route = current;
  }, [current]);
  return null;
}
const edit = () => {};
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
        <ItemList
          isSelectableHeader={selectable}
          onEditFeed={edit}
          onEditTopic={edit}
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
  mocks.pins = [article];
  mocks.sites.sites = [];
  mocks.sites.pinnedSites = [];
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
      item: { ...article.annotations[1], url: article.url },
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
      hostname: "example.org",
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
    hostname: "empty.example",
    title: "empty.example",
    entryCount: 0,
    unreadCount: 0,
  };
  mocks.sites.pinnedSites = [site];
  mocks.sites.listSiteEntries.mockResolvedValue([]);
  const state = { siteHostname: site.hostname };
  await render(state);
  expect(mocks.sites.listSiteEntries).toHaveBeenCalledWith(
    site.hostname,
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
  expect(route.state.siteHostname).toBe(site.hostname);
  expect(container.textContent).toContain(site.hostname);
  expect(mocks.entries.listAllItems).not.toHaveBeenCalled();
});
