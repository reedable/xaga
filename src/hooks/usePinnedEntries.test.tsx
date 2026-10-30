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
      onEntryPinChanged: register("pin"),
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
  isBookmarked: false,
};
beforeEach(() => {
  vi.clearAllMocks();
  mocks.platform.listPinnedEntries.mockResolvedValue([article]);
  root = createRoot(document.createElement("div"));
});
afterEach(async () => {
  await act(async () => root.unmount());
});

it("keeps committed pin intent ahead of delayed cache and reading-state events", async () => {
  await act(async () => root.render(<Probe />));
  expect(entries).toEqual([article]);
  await act(async () => mocks.callbacks.pin({ ...article, isPinned: false }));
  expect(entries).toEqual([]);
  await act(async () => mocks.callbacks.cache());
  expect(entries).toEqual([]);
  mocks.platform.listPinnedEntries.mockResolvedValue([]);
  await act(async () => mocks.callbacks.pin(article));
  await act(async () =>
    mocks.callbacks.unread({ ...article, isPinned: false, isUnread: false })
  );
  await act(async () => mocks.callbacks.cache());
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
  await act(async () => mocks.callbacks.pin(article));
  mocks.platform.listPinnedEntries.mockResolvedValue([
    { ...article, title: "Updated title" },
  ]);
  await act(async () => mocks.callbacks.cache());
  expect(entries[0].title).toBe("Updated title");
  expect(entries[0].isPinned).toBe(true);
});
