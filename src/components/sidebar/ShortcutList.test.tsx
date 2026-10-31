import { act } from "react";
import type { ReactNode } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { Countable } from "../../types";
import ShortcutList from "./ShortcutList";

const mocks = vi.hoisted(() => ({
  sortMode: "alphabetical",
  selectCollection: vi.fn(),
  selectSite: vi.fn(),
  selectArticleScope: vi.fn(),
  state: {} as { articleScopeId?: string; siteHostname?: string },
}));

vi.mock("../../hooks/useAbortController", () => ({
  default: () => [undefined, { current: new AbortController() }],
}));
vi.mock("../../hooks/useFeeds", () => ({
  useFeeds: () => ({ loadingFeedIds: new Set<string>() }),
}));
vi.mock("../../hooks/useRouter", () => ({
  default: () => ({
    selectCollection: mocks.selectCollection,
    selectSite: mocks.selectSite,
    selectArticleScope: mocks.selectArticleScope,
    state: mocks.state,
  }),
}));
vi.mock("../../hooks/useAllItemsCounts", () => ({
  useAllItemsCounts: () => undefined,
}));
vi.mock("../../hooks/useSidebarSort", () => ({
  useSidebarSort: () => [mocks.sortMode, vi.fn()],
}));
vi.mock("./ExpandableList", () => ({
  default: ({ children }: { children: ReactNode }) => <div>{children}</div>,
}));
vi.mock("./SidebarItem", () => ({
  default: ({
    collection,
    onSelectCollection,
    renderSecondaryAction,
    selected,
  }: {
    collection: { title: string };
    onSelectCollection: (value: unknown) => void;
    renderSecondaryAction?: unknown;
    selected?: boolean;
  }) => (
    <button
      data-collection-title={collection.title}
      data-count={!!renderSecondaryAction}
      aria-pressed={!!selected}
      onClick={() => onSelectCollection(collection)}
    />
  ),
}));
vi.mock("./SidebarSortControl", () => ({
  default: () => <div data-sort-control />,
}));
vi.mock("./UnreadCount", () => ({ default: () => null }));
vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);

const shortcuts: Countable[] = [
  {
    id: "zulu",
    title: "Zulu",
    createdAt: "2026-01-01T00:00:00Z",
    entryCount: 1,
    unreadCount: 2,
  },
  {
    id: "alpha",
    title: "Alpha",
    createdAt: "2026-01-01T00:00:00Z",
    entryCount: 1,
    unreadCount: 1,
  },
];

let container: HTMLDivElement;
let root: ReturnType<typeof createRoot>;

beforeEach(() => {
  mocks.sortMode = "alphabetical";
  mocks.state = {};
  vi.clearAllMocks();
  container = document.createElement("div");
  root = createRoot(container);
});

afterEach(async () => {
  await act(async () => root.unmount());
});

it("places the control before All items and sorts only pinned collections", async () => {
  await act(async () => {
    root.render(
      <ShortcutList isExpanded shortcuts={shortcuts} onClickHeader={vi.fn()} />
    );
  });

  expect(
    [...container.firstElementChild!.children].map(
      (element) =>
        element.getAttribute("data-collection-title") ??
        (element.hasAttribute("data-sort-control") ? "sort-control" : null)
    )
  ).toEqual(["sort-control", "All items", "Alpha", "Zulu"]);
});

it("does not render the sort control while collapsed", async () => {
  await act(async () => {
    root.render(<ShortcutList shortcuts={shortcuts} onClickHeader={vi.fn()} />);
  });

  expect(container.querySelector("[data-sort-control]")).toBeNull();
});

it("sorts article shortcuts at zero, displays no count, and selects their article scope", async () => {
  mocks.sortMode = "unread";
  mocks.state = { articleScopeId: "article" };
  const article = {
    __kind: "entry" as const,
    id: "article",
    title: "Article",
    url: "https://example.org",
    createdAt: "2026-10-29",
    isBookmarked: false,
    isUnread: true,
    isPinned: true,
    entryCount: 0,
    unreadCount: 0,
  };
  await act(async () =>
    root.render(
      <ShortcutList
        isExpanded
        shortcuts={[...shortcuts, article]}
        onClickHeader={vi.fn()}
      />
    )
  );
  expect(
    [...container.querySelectorAll("[data-collection-title]")].map((node) =>
      node.getAttribute("data-collection-title")
    )
  ).toEqual(["All items", "Zulu", "Alpha", "Article"]);
  const button = container.querySelector<HTMLButtonElement>(
    '[data-collection-title="Article"]'
  )!;
  expect(button.dataset.count).toBe("false");
  expect(button.getAttribute("aria-pressed")).toBe("true");
  await act(async () => button.click());
  expect(mocks.selectArticleScope).toHaveBeenCalledWith(article);
  expect(mocks.selectCollection).not.toHaveBeenCalled();
});

it("sorts pinned Sites with other shortcuts, keeps All items first, and selects Site scope", async () => {
  const site = {
    id: "example.com",
    hostname: "example.com",
    title: "example.com",
    entryCount: 8,
    unreadCount: 5,
  };
  mocks.sortMode = "unread";
  mocks.state = { siteHostname: site.hostname };
  await act(async () =>
    root.render(
      <ShortcutList
        isExpanded
        shortcuts={[...shortcuts, site]}
        onClickHeader={vi.fn()}
      />
    )
  );
  expect(
    [...container.querySelectorAll("[data-collection-title]")].map((node) =>
      node.getAttribute("data-collection-title")
    )
  ).toEqual(["All items", "example.com", "Zulu", "Alpha"]);
  const button = container.querySelector<HTMLButtonElement>(
    '[data-collection-title="example.com"]'
  )!;
  expect(button.getAttribute("aria-pressed")).toBe("true");
  expect(button.dataset.count).toBe("true");
  await act(async () => button.click());
  expect(mocks.selectSite).toHaveBeenCalledWith(site);
  expect(mocks.selectCollection).not.toHaveBeenCalled();
});
