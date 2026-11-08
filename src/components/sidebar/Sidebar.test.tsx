import { act, type ReactNode } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { Sidebar } from "./Sidebar";

const mocks = vi.hoisted(() => ({
  readOnly: false,
  selectShortcutTarget: vi.fn(),
  state: {} as { shortcutId?: string; articleScopeId?: string },
  loadingFeedIds: new Set<string>(),
  feeds: [
    { id: "feed-alpha", title: "Alpha Feed" },
    { id: "feed-beta", title: "Beta Feed" },
  ],
  sites: [
    { id: "site-alpha", title: "Alpha Site" },
    { id: "site-beta", title: "Beta Site" },
  ],
  topics: [
    { id: "topic-alpha", title: "Alpha Topic" },
    { id: "topic-beta", title: "Beta Topic" },
  ],
}));

vi.mock("../../hooks/useSidebarSort", () => ({
  useSidebarSort: () => ["unread", vi.fn()],
}));
vi.mock("../../providers/TagsContext", () => ({
  useTags: () => ({ tags: [], hasLoaded: true, unreadCount: 0 }),
}));
vi.mock("./TagList", () => ({
  default: () => <div data-section="tags" data-titles="" />,
}));
vi.mock("../../hooks/useAbortController", () => ({
  default: () => [undefined, { current: new AbortController() }],
}));
vi.mock("../../hooks/usePinnedEntries", () => ({ usePinnedEntries: () => [] }));
vi.mock("../../hooks/useShortcutTargets", () => ({
  useShortcutTargets: () => ({
    targets: [...mocks.feeds, ...mocks.topics],
    missing: [],
  }),
}));
vi.mock("../../hooks/useLibraryReadOnly", () => ({
  useLibraryReadOnly: () => mocks.readOnly,
}));
vi.mock("../../hooks/useSidebar", () => ({
  useSidebar: () => ({ expandedList: null, setExpandedList: vi.fn() }),
}));
vi.mock("../../hooks/useTopics", () => ({
  useTopics: () => ({
    topics: mocks.topics,
    pinnedTopicIds: ["topic-alpha", "topic-beta"],
  }),
}));
vi.mock("../../hooks/useFeeds", () => ({
  useFeeds: () => ({
    loading: false,
    loadingFeedIds: mocks.loadingFeedIds,
    feeds: mocks.feeds,
    pinnedFeedIds: ["feed-alpha", "feed-beta"],
    refreshAllFeeds: vi.fn(),
  }),
}));
vi.mock("../../hooks/useSites", () => ({
  useSites: () => ({ sites: mocks.sites }),
}));

vi.mock("../../hooks/useRouter", () => ({
  default: () => ({
    state: mocks.state,
    selectShortcutTarget: mocks.selectShortcutTarget,
  }),
}));
vi.mock("../../hooks/useAllEntitiesCounts", () => ({
  useAllEntitiesCounts: () => ({ entryCount: 5, unreadCount: 2 }),
}));

function listMock(section: string) {
  return ({
    children,
    ...props
  }: {
    children?: ReactNode;
    [key: string]: unknown;
  }) => {
    const collections =
      (props.shortcuts as typeof mocks.topics | undefined) ??
      (props.topics as typeof mocks.topics | undefined) ??
      (props.feeds as typeof mocks.feeds | undefined) ??
      (props.sites as typeof mocks.sites | undefined) ??
      [];
    return (
      <div
        data-section={section}
        data-filter={
          section === "shortcuts" ? String(props.filter ?? "") : undefined
        }
        data-titles={collections
          .filter(
            ({ title }) =>
              section !== "shortcuts" ||
              title.toLocaleLowerCase().includes(
                String(props.filter ?? "")
                  .trim()
                  .toLocaleLowerCase()
              )
          )
          .map(({ title }) => title)
          .join(",")}
      >
        {children}
      </div>
    );
  };
}

vi.mock("./ShortcutList", () => ({ default: listMock("shortcuts") }));
vi.mock("./TopicList", () => ({ default: listMock("topics") }));
vi.mock("./FeedList", () => ({ default: listMock("feeds") }));
vi.mock("./SiteList", () => ({ default: listMock("sites") }));
vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);

let container: HTMLDivElement;
let root: ReturnType<typeof createRoot>;

beforeEach(() => {
  mocks.state = {};
  mocks.loadingFeedIds = new Set();
  vi.clearAllMocks();
  container = document.createElement("div");
  root = createRoot(container);
});

afterEach(async () => {
  await act(async () => root.unmount());
});

it("applies one trimmed, case-insensitive filter to every collection section", async () => {
  await act(async () => {
    root.render(
      <Sidebar
        filter="  ALPHA  "
        onAddFeed={vi.fn()}
        onAddTopic={vi.fn()}
        onFilterChange={vi.fn()}
      />
    );
  });

  expect(
    container
      .querySelector('[data-section="shortcuts"]')
      ?.getAttribute("data-titles")
  ).toBe("Alpha Feed,Alpha Topic");
  expect(
    container
      .querySelector('[data-section="topics"]')
      ?.getAttribute("data-titles")
  ).toBe("Alpha Topic");
  expect(
    container
      .querySelector('[data-section="feeds"]')
      ?.getAttribute("data-titles")
  ).toBe("Alpha Feed");
  expect(
    container
      .querySelector('[data-section="sites"]')
      ?.getAttribute("data-titles")
  ).toBe("Alpha Site");
});

it("keeps every section and the filter above them visible with zero matches", async () => {
  await act(async () => {
    root.render(
      <Sidebar
        filter="missing"
        onAddFeed={vi.fn()}
        onAddTopic={vi.fn()}
        onFilterChange={vi.fn()}
      />
    );
  });

  expect(container.querySelectorAll("[data-section]")).toHaveLength(5);
  expect(
    [...container.querySelectorAll("[data-section]")].every(
      (section) => section.getAttribute("data-titles") === ""
    )
  ).toBe(true);

  const input = container.querySelector('[aria-label="Filter collections"]');
  expect(input).not.toBeNull();
  const allItems = [
    ...container.querySelectorAll<HTMLElement>('[role="button"]'),
  ].filter((button) => button.textContent === "All items");
  expect(allItems).toHaveLength(1);
  expect(
    allItems[0].compareDocumentPosition(input!) &
      Node.DOCUMENT_POSITION_FOLLOWING
  ).toBeTruthy();
  expect(allItems[0].parentElement!.textContent).toBe("All items2");
  await act(async () => allItems[0].click());
  expect(mocks.selectShortcutTarget).toHaveBeenCalledWith(
    expect.objectContaining({ title: "All items", unreadCount: 2 })
  );
  const firstSection = container.querySelector("[data-section]");
  expect(firstSection).not.toBeNull();
  expect(
    input!.compareDocumentPosition(firstSection!) &
      Node.DOCUMENT_POSITION_FOLLOWING
  ).toBeTruthy();
});

vi.mock("../../hooks/useMissingPins", () => ({ useMissingPins: () => [] }));

it("preserves All items selection and feed-loading feedback", async () => {
  const { ALL_ENTITIES } = await import("../../Shortcuts");
  mocks.state = { shortcutId: ALL_ENTITIES.id };
  mocks.loadingFeedIds = new Set(["feed-alpha"]);
  const render = () =>
    root.render(
      <Sidebar
        filter=""
        onAddFeed={vi.fn()}
        onAddTopic={vi.fn()}
        onFilterChange={vi.fn()}
      />
    );
  await act(async () => render());
  const item = container.querySelector("li")!;
  expect(
    item.querySelector('[role="button"]')!.classList.contains("Mui-selected")
  ).toBe(true);
  expect(item.querySelector('[role="progressbar"]')).not.toBeNull();
  mocks.state = { shortcutId: ALL_ENTITIES.id, articleScopeId: "article" };
  mocks.loadingFeedIds = new Set();
  await act(async () => render());
  expect(
    item.querySelector('[role="button"]')!.classList.contains("Mui-selected")
  ).toBe(false);
  expect(item.querySelector('[role="progressbar"]')).toBeNull();
});

it("keeps the shared sort control available in a read-only library", async () => {
  mocks.readOnly = true;
  await act(async () =>
    root.render(
      <Sidebar
        filter=""
        onFilterChange={vi.fn()}
        onAddFeed={vi.fn()}
        onAddTopic={vi.fn()}
      />
    )
  );
  expect(
    container.querySelector<HTMLButtonElement>(
      '[aria-label="Sort collections"]'
    )?.disabled
  ).toBe(false);
  expect(
    container.querySelector<HTMLButtonElement>('button[aria-label="Refresh"]')
      ?.disabled
  ).toBe(true);
  mocks.readOnly = false;
});
