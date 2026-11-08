import { act, useEffect } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { useSidebarSort } from "./useSidebarSort";
import type { SidebarSortMode } from "../sidebarSort";
import { SidebarSortProvider } from "../providers/SidebarSortProvider";

const mocks = vi.hoisted(() => ({
  getSidebarSort: vi.fn(),
  setSidebarSort: vi.fn(),
  onSidebarSortChanged: vi.fn(),
  pushError: vi.fn(),
}));
vi.mock("../platform/Platform", () => ({ usePlatform: () => mocks }));
vi.mock("./useErrorSink", () => ({ useErrorSink: () => mocks }));
vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
let root: ReturnType<typeof createRoot>;
let changed: (value: SidebarSortMode) => void;
let stop: ReturnType<typeof vi.fn>;
let consumers: ReturnType<typeof useSidebarSort>[];
function Capture({ index }: { index: number }) {
  const value = useSidebarSort();
  useEffect(() => {
    consumers[index] = value;
  }, [value, index]);
  return null;
}
async function render() {
  await act(async () =>
    root.render(
      <SidebarSortProvider>
        <Capture index={0} />
        <Capture index={1} />
      </SidebarSortProvider>
    )
  );
}
beforeEach(() => {
  vi.clearAllMocks();
  consumers = [];
  stop = vi.fn();
  mocks.getSidebarSort.mockResolvedValue("unread");
  mocks.onSidebarSortChanged.mockImplementation(async (callback) => {
    changed = callback;
    return stop;
  });
  mocks.setSidebarSort.mockImplementation(async (value) => changed(value));
  root = createRoot(document.createElement("div"));
});
afterEach(async () => {
  await act(async () => root.unmount());
});

it("starts unread, reads one shared preference, and synchronizes all consumers", async () => {
  await render();
  expect(consumers.map((c) => c[0])).toEqual(["unread", "unread"]);
  expect(mocks.getSidebarSort).toHaveBeenCalledOnce();
  await act(async () => consumers[0][1]("alphabetical"));
  expect(mocks.setSidebarSort).toHaveBeenCalledExactlyOnceWith("alphabetical");
  expect(consumers.map((c) => c[0])).toEqual(["alphabetical", "alphabetical"]);
  await act(async () => changed("unread"));
  expect(consumers.map((c) => c[0])).toEqual(["unread", "unread"]);
  expect(mocks.setSidebarSort).toHaveBeenCalledTimes(1);
});
it("restores the shared choice on a subsequent mount", async () => {
  mocks.getSidebarSort.mockResolvedValue("alphabetical");
  await render();
  expect(consumers.map((c) => c[0])).toEqual(["alphabetical", "alphabetical"]);
  expect(mocks.setSidebarSort).not.toHaveBeenCalled();
});
it("does not let a late startup read overwrite a committed menu or UI change", async () => {
  let finish!: (mode: SidebarSortMode) => void;
  mocks.getSidebarSort.mockImplementation(
    () =>
      new Promise<SidebarSortMode>((resolve) => {
        finish = resolve;
      })
  );
  await render();
  await act(async () => changed("alphabetical"));
  await act(async () => finish("unread"));
  expect(consumers.map((c) => c[0])).toEqual(["alphabetical", "alphabetical"]);
});
it("keeps the previous choice and reports failed persistence", async () => {
  await render();
  const error = new Error("settings write failed");
  mocks.setSidebarSort.mockRejectedValue(error);
  await act(async () => consumers[0][1]("alphabetical"));
  expect(consumers[0][0]).toBe("unread");
  expect(mocks.pushError).toHaveBeenCalledWith(error);
});
it("unsubscribes and ignores late reads after unmount", async () => {
  await render();
  await act(async () => root.render(null));
  expect(stop).toHaveBeenCalledOnce();
});
