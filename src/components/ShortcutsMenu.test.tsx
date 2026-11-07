import { act, useEffect } from "react";
import { createRoot } from "react-dom/client";
import { MemoryRouter } from "react-router-dom";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import ShortcutsMenu from "./ShortcutsMenu";
import useRouter from "../hooks/useRouter";
import { useSidebarSort } from "../hooks/useSidebarSort";
import { PinsContext } from "../providers/PinsContext";
import FeedsContext from "../providers/FeedsContext";
import TopicsContext from "../providers/TopicsContext";
import SitesContext from "../providers/SitesContext";
import { emptySite } from "../sites";
import type { Entry, Feed, Topic } from "../types";
import type { PinTarget, PinNode, ShortcutMenuItem } from "../pins";
import { ALL_ENTITIES } from "../Shortcuts";

const mocks = vi.hoisted(() => ({
  entries: [] as Entry[],
  platform: {
    setShortcutsMenu: vi.fn(),
    onShortcutMenuSelected: vi.fn(),
    readTopic: vi.fn(),
    listTopics: vi.fn(),
  },
  sink: { pushError: vi.fn() },
  readOnly: false,
  setPin: vi.fn(),
  storage: { getViewState: vi.fn(), setViewState: vi.fn() },
}));
vi.mock("../platform/Platform", () => ({ usePlatform: () => mocks.platform }));
vi.mock("../hooks/usePinnedEntries", () => ({
  usePinnedEntries: () => mocks.entries,
}));
vi.mock("../hooks/useLibraryReadOnly", () => ({
  useLibraryReadOnly: () => mocks.readOnly,
}));
vi.mock("../hooks/useErrorSink", () => ({ useErrorSink: () => mocks.sink }));
vi.mock("../storage/viewState", () => mocks.storage);
vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
const feed: Feed = {
  __kind: "feed",
  id: "feed",
  title: "Zulu Feed",
  createdAt: "now",
  members: [],
  url: "https://example.com/feed",
  isPinned: true,
  entryCount: 10,
  unreadCount: 5,
};
const topic: Topic = {
  ...feed,
  __kind: "topic",
  id: "topic",
  title: "Alpha Topic",
  unreadCount: 2,
};
const article: Entry = {
  __kind: "entry",
  id: "article",
  title: "Article",
  createdAt: "now",
  url: "https://example.com/article",
  isPinned: true,
};
let root: ReturnType<typeof createRoot>;
let router: ReturnType<typeof useRouter>;
let setSort: ReturnType<typeof useSidebarSort>[1];
let selected: (key: string) => void;
let unlisten: ReturnType<typeof vi.fn>;
let pins: PinTarget[];
let tree: PinNode[] | undefined;
let feeds: Feed[];
let topics: Topic[];
let loaded: boolean;
function Host() {
  const value = useRouter();
  const [, setMode] = useSidebarSort("shortcuts");
  useEffect(() => {
    router = value;
    setSort = setMode;
  }, [value, setMode]);
  return <ShortcutsMenu />;
}
async function render() {
  await act(async () =>
    root.render(
      <MemoryRouter
        initialEntries={[{ pathname: "/", state: { filter: "saved" } }]}
      >
        <PinsContext.Provider
          value={{
            pins,
            tree: tree ?? pins,
            organizePins: vi.fn(),
            setPin: mocks.setPin,
          }}
        >
          <FeedsContext.Provider
            value={{
              feeds,
              hasLoaded: loaded,
              pinnedFeedIds: feeds.map((f) => f.id),
              error: null,
              loading: false,
              loadingFeedIds: new Set(),
              setError: vi.fn(),
              setLoading: vi.fn(),
              setLoadingSet: vi.fn(),
              setFeeds: vi.fn(),
              setFeedPinned: vi.fn(),
            }}
          >
            <TopicsContext.Provider
              value={{
                topics,
                hasLoaded: loaded,
                pinnedTopicIds: topics.map((t) => t.id),
                setTopics: vi.fn(),
                setTopicPinned: vi.fn(),
                setMembership: vi.fn(),
                getMemberships: (_id, ids) => ids,
              }}
            >
              <SitesContext.Provider
                value={{
                  sites: [],
                  pinnedSites: pins.some((p) => p.type === "site")
                    ? [emptySite("example.com")]
                    : [],
                  hasLoaded: loaded,
                  refreshSites: vi.fn(),
                }}
              >
                <Host />
              </SitesContext.Provider>
            </TopicsContext.Provider>
          </FeedsContext.Provider>
        </PinsContext.Provider>
      </MemoryRouter>
    )
  );
}
function items(): ShortcutMenuItem[] {
  return mocks.platform.setShortcutsMenu.mock.calls.at(-1)![0];
}
async function click(title: string) {
  await act(async () =>
    selected(items().find((item) => item.title === title)!.key)
  );
}
function button(text: string) {
  return [...document.querySelectorAll<HTMLButtonElement>("button")].find(
    (b) => b.textContent === text
  )!;
}
beforeEach(() => {
  vi.clearAllMocks();
  mocks.platform.setShortcutsMenu.mockResolvedValue(undefined);
  unlisten = vi.fn();
  mocks.platform.onShortcutMenuSelected.mockImplementation(async (callback) => {
    selected = callback;
    return unlisten;
  });
  mocks.storage.getViewState.mockResolvedValue(undefined);
  mocks.storage.setViewState.mockResolvedValue(undefined);
  mocks.setPin.mockResolvedValue(undefined);
  mocks.entries = [article];
  mocks.readOnly = false;
  tree = undefined;
  pins = [
    { type: "entry", id: "article" },
    { type: "feed", id: "feed" },
    { type: "topic", id: "topic" },
    { type: "site", hostname: "example.com" },
  ];
  feeds = [feed];
  topics = [topic];
  loaded = true;
  const container = document.createElement("div");
  document.body.append(container);
  root = createRoot(container);
});
afterEach(async () => {
  await act(async () => root.unmount());
  document.body.innerHTML = "";
});

it("shares sidebar sorting and updates titles, counts and membership", async () => {
  await render();
  expect(items().map((i) => i.title)).toEqual([
    "Zulu Feed",
    "Alpha Topic",
    "Article",
    "example.com",
  ]);
  await act(async () => setSort("alphabetical"));
  expect(items().map((i) => i.title)).toEqual([
    "Alpha Topic",
    "Article",
    "example.com",
    "Zulu Feed",
  ]);
  await act(async () => setSort("unread"));
  expect(items().map((i) => i.title)).toEqual([
    "Zulu Feed",
    "Alpha Topic",
    "Article",
    "example.com",
  ]);
  topics = [{ ...topic, unreadCount: 20, title: "Renamed Topic" }];
  await render();
  expect(items().map((i) => i.title)).toEqual([
    "Renamed Topic",
    "Zulu Feed",
    "Article",
    "example.com",
  ]);
  feeds = [];
  pins = pins.filter((p) => p.type !== "feed");
  await render();
  expect(items().some((i) => i.title === feed.title)).toBe(false);
  expect(mocks.setPin).not.toHaveBeenCalled();
});

it("routes every target through sidebar navigation, including article scope and empty Sites", async () => {
  await render();
  await click("Zulu Feed");
  expect(router.state).toEqual({ feedId: "feed", filter: "saved" });
  await click("Alpha Topic");
  expect(router.state).toEqual({ topicId: "topic", filter: "saved" });
  await click("example.com");
  expect(router.state).toEqual({
    siteHostname: "example.com",
    filter: "saved",
  });
  await click("Article");
  expect(router.state).toEqual({
    articleScopeId: "article",
    entryId: "article",
    url: article.url,
  });
  await act(async () => selected("all"));
  expect(router.state.shortcutId).toBe(ALL_ENTITIES.id);
});

it("keeps All items navigation with no pins and ignores delayed clicks after removal", async () => {
  await render();
  const stale = items()[0].key;
  feeds = [];
  topics = [];
  mocks.entries = [];
  pins = [];
  await render();
  expect(items()).toEqual([]);
  const previous = router.state;
  await act(async () => selected(stale));
  expect(router.state).toBe(previous);
  await act(async () => selected("all"));
  expect(router.state.shortcutId).toBe(ALL_ENTITIES.id);
});

it("projects loading separately and opens unavailable-target actions without a sidebar in read-only mode", async () => {
  mocks.readOnly = true;
  pins = [{ type: "topic", id: "missing" }];
  feeds = [];
  topics = [];
  mocks.entries = [];
  loaded = false;
  await render();
  expect(items()).toEqual([
    expect.objectContaining({ title: "Loading shortcut…", enabled: false }),
  ]);
  loaded = true;
  await render();
  await click("Shortcut unavailable");
  expect(document.querySelector('[role="dialog"]')?.textContent).toContain(
    "We can’t find this topic."
  );
  expect(button("Remove shortcut").disabled).toBe(true);
  mocks.platform.readTopic.mockRejectedValue({
    code: "topic:not-found",
    message: "missing",
  });
  await act(async () => button("Retry").click());
  expect(mocks.platform.readTopic).toHaveBeenCalled();
  expect(document.querySelector('[role="status"]')?.textContent).toContain(
    "We can’t find this topic."
  );
  await act(async () => button("Close").click());
  await act(async () => selected("all"));
  expect(router.state.shortcutId).toBe(ALL_ENTITIES.id);
  expect(mocks.setPin).not.toHaveBeenCalled();
});

it("removes only an unavailable shortcut without changing navigation", async () => {
  pins = [{ type: "topic", id: "missing" }];
  feeds = [];
  topics = [];
  mocks.entries = [];
  await render();
  await click("Shortcut unavailable");
  const previous = router.state;
  await act(async () => button("Remove shortcut").click());
  expect(mocks.setPin).toHaveBeenCalledWith(pins[0], false);
  expect(router.state).toBe(previous);
});

it("serializes native updates and applies the latest snapshot after an in-flight update", async () => {
  let finish!: () => void;
  mocks.platform.setShortcutsMenu.mockImplementationOnce(
    () =>
      new Promise<void>((resolve) => {
        finish = resolve;
      })
  );
  await render();
  topics = [{ ...topic, title: "Latest" }];
  await render();
  expect(mocks.platform.setShortcutsMenu).toHaveBeenCalledTimes(1);
  await act(async () => finish());
  expect(items().some((i) => i.title === "Latest")).toBe(true);
  await act(async () => root.unmount());
  expect(unlisten).toHaveBeenCalledTimes(1);
  root = createRoot(document.createElement("div"));
});

it("projects nested sorted submenus, empty groups, and current descendant navigation", async () => {
  tree = [
    {
      type: "group",
      id: "research",
      title: "Research",
      children: [
        { type: "topic", id: "topic" },
        {
          type: "group",
          id: "articles",
          title: "Articles",
          children: [{ type: "entry", id: "article" }],
        },
        { type: "feed", id: "feed" },
      ],
    },
    { type: "group", id: "empty", title: "Empty group", children: [] },
    { type: "site", hostname: "example.com" },
  ];
  await render();
  expect(items().map((item) => item.title)).toEqual([
    "Research",
    "Empty group",
    "example.com",
  ]);
  expect(items()[0].children?.map((item) => item.title)).toEqual([
    "Zulu Feed",
    "Alpha Topic",
    "Articles",
  ]);
  expect(items()[1].children).toEqual([]);
  const articleKey = items()[0].children?.[2].children?.[0].key;
  await act(async () => selected(articleKey!));
  expect(router.state.articleScopeId).toBe("article");
  await act(async () => setSort("alphabetical"));
  expect(items().map((item) => item.title)).toEqual([
    "Empty group",
    "example.com",
    "Research",
  ]);
  expect(items()[2].children?.map((item) => item.title)).toEqual([
    "Alpha Topic",
    "Articles",
    "Zulu Feed",
  ]);
  const staleKey = items()[2].children?.[1].children?.[0].key;
  tree = [
    { type: "group", id: "empty", title: "Renamed empty group", children: [] },
  ];
  await render();
  await act(async () => selected(staleKey!));
  expect(items().map((item) => item.title)).toEqual(["Renamed empty group"]);
  expect(router.state.articleScopeId).toBe("article");
});
