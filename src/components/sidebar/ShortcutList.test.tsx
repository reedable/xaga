import { act } from "react";
import type { ReactNode } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { Countable } from "../../types";
import ShortcutList from "./ShortcutList";

const mocks = vi.hoisted(() => ({ sortMode: "alphabetical" }));

vi.mock("../../hooks/useAbortController", () => ({
  default: () => [undefined, { current: new AbortController() }],
}));
vi.mock("../../hooks/useFeeds", () => ({
  useFeeds: () => ({ loadingFeedIds: new Set<string>() }),
}));
vi.mock("../../hooks/useRouter", () => ({
  default: () => ({
    selectCollection: vi.fn(),
    state: {},
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
  default: ({ collection }: { collection: { title: string } }) => (
    <div data-collection-title={collection.title} />
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
