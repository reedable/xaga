import { act, type ReactNode } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { Sidebar } from "./Sidebar";

const mocks = vi.hoisted(() => ({
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

vi.mock("../../hooks/useAbortController", () => ({
  default: () => [undefined, { current: new AbortController() }],
}));
vi.mock("../../hooks/useLibraryReadOnly", () => ({
  useLibraryReadOnly: () => false,
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
    feeds: mocks.feeds,
    pinnedFeedIds: ["feed-alpha", "feed-beta"],
    refreshAllFeeds: vi.fn(),
  }),
}));
vi.mock("../../hooks/useSites", () => ({
  useSites: () => ({ sites: mocks.sites }),
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
        data-titles={collections.map(({ title }) => title).join(",")}
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

it("keeps every section and the bottom filter visible with zero matches", async () => {
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

  expect(container.querySelectorAll("[data-section]")).toHaveLength(4);
  expect(
    [...container.querySelectorAll("[data-section]")].every(
      (section) => section.getAttribute("data-titles") === ""
    )
  ).toBe(true);

  const input = container.querySelector('[aria-label="Filter collections"]');
  expect(input).not.toBeNull();
  expect(container.firstElementChild?.lastElementChild?.contains(input)).toBe(
    true
  );
});
