import { act, useEffect, useRef } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import FeedsContext, { type FeedsContextType } from "../providers/FeedsContext";
import type { Feed } from "../types";
import { useFeeds } from "./useFeeds";

const mocks = vi.hoisted(() => ({
  platform: { listFeeds: vi.fn(), refreshAllFeeds: vi.fn() },
  pushError: vi.fn(),
}));
vi.mock("../platform/Platform", () => ({ usePlatform: () => mocks.platform }));
vi.mock("./useErrorSink", () => ({
  useErrorSink: () => ({ pushError: mocks.pushError }),
}));
vi.mock("./useSidebar", () => ({ useSidebar: () => ({ listFeeds: vi.fn() }) }));
vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
let root: ReturnType<typeof createRoot>;
let hook: ReturnType<typeof useFeeds>;
let controller: AbortController;
let context: FeedsContextType;
const feed: Feed = {
  __kind: "feed",
  id: "notes",
  title: "Field notes",
  url: "https://example.org/feed",
  createdAt: "2026-01-01T00:00:00Z",
  isPinned: false,
  entryCount: 0,
  unreadCount: 0,
  members: [],
};
const failure = {
  id: "workshop",
  title: "Workshop",
  code: "feed:http",
  message: "Couldn't reach this feed.",
};
beforeEach(async () => {
  vi.resetAllMocks();
  controller = new AbortController();
  mocks.platform.listFeeds.mockResolvedValue([feed]);
  context = {
    error: null,
    loading: false,
    loadingFeedIds: new Set(),
    feeds: [],
    pinnedFeedIds: [],
    setError: vi.fn(),
    setLoading: vi.fn(),
    setLoadingSet: vi.fn(),
    setFeeds: vi.fn(),
    setFeedPinned: vi.fn(),
  };
  function Host() {
    const feeds = useFeeds(useRef(controller));
    useEffect(() => {
      hook = feeds;
    }, [feeds]);
    return null;
  }
  root = createRoot(document.createElement("div"));
  await act(() =>
    root.render(
      <FeedsContext.Provider value={context}>
        <Host />
      </FeedsContext.Provider>
    )
  );
});
afterEach(async () => {
  await act(() => root.unmount());
});
it("retains successful feeds and consolidates safe failures with retry guidance", async () => {
  mocks.platform.refreshAllFeeds.mockResolvedValue({
    feeds: [feed],
    failures: [failure, { ...failure, id: "other", title: "Other feed" }],
  });
  await act(() => hook.refreshAllFeeds());
  expect(mocks.pushError).toHaveBeenCalledTimes(1);
  expect(mocks.pushError.mock.calls[0][0].message).toMatch(
    /Workshop: Couldn't reach this feed.*Other feed:.*Use Refresh to retry/
  );
  const merge = vi.mocked(context.setFeeds).mock.calls[0][0] as (
    current: (typeof feed)[]
  ) => (typeof feed)[];
  expect(
    merge([
      { ...feed, title: "Old" },
      { ...feed, id: "unaffected", title: "Unaffected" },
    ])
  ).toEqual([feed, { ...feed, id: "unaffected", title: "Unaffected" }]);
  expect(context.setLoadingSet).toHaveBeenLastCalledWith(new Set());
});
it.each([{ failures: [] }, { failures: [failure] }])(
  "clears loading for no-new success or all failures",
  async ({ failures }) => {
    mocks.platform.refreshAllFeeds.mockResolvedValue({ feeds: [], failures });
    await act(() => hook.refreshAllFeeds());
    expect(mocks.pushError).toHaveBeenCalledTimes(failures.length ? 1 : 0);
    expect(context.setLoadingSet).toHaveBeenLastCalledWith(new Set());
  }
);
it("reports whole-request refusal without exposing diagnostics", async () => {
  mocks.platform.refreshAllFeeds.mockRejectedValue({
    code: "library:read-only",
    message: "This library is read-only.",
    detail: "private diagnostic",
  });
  await act(() => hook.refreshAllFeeds());
  expect(mocks.pushError.mock.calls[0][0].message).toBe(
    "This library is read-only. Use Refresh to retry."
  );
  expect(context.setLoadingSet).toHaveBeenLastCalledWith(new Set());
});
it("silences cancellation and clears loading", async () => {
  mocks.platform.refreshAllFeeds.mockImplementation(async () => {
    controller.abort();
    throw new Error("cancelled");
  });
  await act(() => hook.refreshAllFeeds());
  expect(mocks.pushError).not.toHaveBeenCalled();
  expect(context.setLoadingSet).toHaveBeenLastCalledWith(new Set());
});
