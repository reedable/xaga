import { act, useEffect } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { Entry } from "../types";
import { usePinnedEntries } from "./usePinnedEntries";

const mocks = vi.hoisted(() => {
  const callbacks: Record<string, (entry?: unknown) => void> = {};
  const register = (name: string) =>
    vi.fn(async (callback: (entry?: unknown) => void) => {
      callbacks[name] = callback;
      return () => {
        delete callbacks[name];
      };
    });
  return {
    callbacks,
    sink: { pushError: vi.fn() },
    platform: {
      listPinnedEntries: vi.fn(),
      onPinChanged: register("pin"),
      onFaviconChanged: register("favicon"),
      onUnreadChanged: register("unread"),
      onBookmarkChanged: register("bookmark"),
      onEmbeddingsChanged: register("cache"),
    },
  };
});
vi.mock("../platform/Platform", () => ({ usePlatform: () => mocks.platform }));
vi.mock("./useErrorSink", () => ({ useErrorSink: () => mocks.sink }));
vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
let entries: Entry[];
function Probe() {
  const current = usePinnedEntries();
  useEffect(() => {
    entries = current;
  }, [current]);
  return null;
}
let root: ReturnType<typeof createRoot>;
const article: Entry = {
  __kind: "entry",
  id: "article",
  title: "Article",
  createdAt: "2026-10-29",
  url: "https://example.org",
  isPinned: true,
  isUnread: true,
};
beforeEach(() => {
  vi.clearAllMocks();
  mocks.platform.listPinnedEntries.mockResolvedValue([article]);
  root = createRoot(document.createElement("div"));
});
afterEach(async () => {
  await act(async () => root.unmount());
});

it("reads committed membership from files independently of cache events", async () => {
  await act(async () => root.render(<Probe />));
  expect(entries).toEqual([article]);
  mocks.platform.listPinnedEntries.mockResolvedValue([]);
  await act(async () =>
    mocks.callbacks.unread({
      target: { type: "entry", id: article.id },
      pinned: false,
    })
  );
  expect(entries).toEqual([]);
  await act(async () => mocks.callbacks.cache());
  expect(entries).toEqual([]);
  mocks.platform.listPinnedEntries.mockResolvedValue([
    { ...article, isUnread: false },
  ]);
  await act(async () =>
    mocks.callbacks.unread({
      target: { type: "entry", id: article.id },
      pinned: true,
    })
  );
  expect(entries).toEqual([{ ...article, isUnread: false }]);
});
it("does not allow an earlier cache request to overwrite a later refresh", async () => {
  let resolveInitial!: (entries: Entry[]) => void;
  mocks.platform.listPinnedEntries.mockImplementationOnce(
    () =>
      new Promise<Entry[]>((resolve) => {
        resolveInitial = resolve;
      })
  );
  await act(async () => root.render(<Probe />));
  mocks.platform.listPinnedEntries.mockResolvedValue([]);
  await act(async () => mocks.callbacks.cache());
  await act(async () => resolveInitial([article]));
  expect(entries).toEqual([]);
});

it("allows refreshed article metadata to replace the pin command's snapshot", async () => {
  await act(async () => root.render(<Probe />));
  await act(async () => mocks.callbacks.unread(article));
  mocks.platform.listPinnedEntries.mockResolvedValue([
    { ...article, title: "Updated title" },
  ]);
  await act(async () => mocks.callbacks.cache());
  expect(entries[0].title).toBe("Updated title");
  expect(entries[0].isPinned).toBe(true);
});

vi.mock("../providers/PinsContext", () => {
  const pins = [{ type: "entry", id: "article" }];
  return { usePins: () => ({ pins }) };
});

it("refreshes pinned article colors for color-to-color and removal events", async () => {
  await act(async () => root.render(<Probe />));
  for (const color of ["blue", "green", undefined]) {
    mocks.platform.listPinnedEntries.mockResolvedValue([{ ...article, color }]);
    await act(async () => mocks.callbacks.bookmark({ ...article, color }));
    expect(entries[0].color).toBe(color);
    expect(entries[0].isPinned).toBe(true);
  }
});
