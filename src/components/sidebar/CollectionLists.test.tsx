import { act, useEffect } from "react";
import { createRoot } from "react-dom/client";
import { MemoryRouter } from "../../test/NavigationRouter";
import { afterEach, expect, it, vi } from "vitest";
import type { Feed, Topic } from "../../domain";
import useRouter from "../../hooks/useRouter";
import { emptySite } from "../../sites";
import FeedList from "./FeedList";
import SiteList from "./SiteList";
import TopicList from "./TopicList";
vi.mock("react-virtuoso", () => import("../../test/mockVirtuoso"));

vi.mock("../../hooks/useFeeds", () => ({
  useFeeds: () => ({ loadingFeedIds: new Set<string>() }),
}));
vi.mock("../../hooks/useSidebarSort", () => ({
  useSidebarSort: () => ["alphabetical", vi.fn()],
}));
vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);

const feed: Feed = {
  __kind: "feed",
  id: "feed",
  title: "Feed",
  createdAt: "2026-01-01",
  url: "https://example.org/feed",
  members: [],
  isPinned: false,
  entryCount: 5,
  unreadCount: 3,
};
const topic: Topic = { ...feed, __kind: "topic", id: "topic", title: "Topic" };
const site = { ...emptySite("example.org"), entryCount: 5, unreadCount: 3 };
let root: ReturnType<typeof createRoot>;
let container: HTMLDivElement;
let router: ReturnType<typeof useRouter>;
function Observe() {
  const value = useRouter();
  useEffect(() => {
    router = value;
  }, [value]);
  return null;
}
afterEach(async () => {
  await act(async () => root.unmount());
  container.remove();
});

it.each(["feed", "topic", "site"] as const)(
  "selects a %s from the sidebar with its counts",
  async (kind) => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    await act(async () =>
      root.render(
        <MemoryRouter initialEntries={[{ pathname: "/", state: {} }]}>
          <Observe />
          {kind === "feed" && (
            <FeedList
              isExpanded
              feeds={[feed]}
              onAddFeed={vi.fn()}
              onClickHeader={vi.fn()}
            />
          )}
          {kind === "topic" && (
            <TopicList
              isExpanded
              topics={[topic]}
              onAddTopic={vi.fn()}
              onClickHeader={vi.fn()}
            />
          )}
          {kind === "site" && (
            <SiteList isExpanded sites={[site]} onClickHeader={vi.fn()} />
          )}
        </MemoryRouter>
      )
    );
    const collection = { feed, topic, site }[kind];
    const row = [
      ...container.querySelectorAll<HTMLElement>(".MuiListItemButton-root"),
    ].find((item) => item.textContent?.includes(collection.title))!;
    expect(row).toBeDefined();
    expect(row.closest('[role="listitem"]')?.textContent).toContain("3");
    await act(async () => row.click());
    expect(router.state).toEqual({
      ...{
        feed: { feedId: feed.id },
        topic: { topicId: topic.id },
        site: { siteHostname: site.id },
      }[kind],
    });
    expect(row.classList.contains("Mui-selected")).toBe(true);
  }
);
