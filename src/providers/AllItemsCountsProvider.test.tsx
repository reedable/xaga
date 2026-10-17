import { act, useEffect } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import {
  AllItemsCountsProvider,
  useAllItemsCounts,
} from "./AllItemsCountsProvider";
import type { AllItemsCounts } from "../types";

const platform = vi.hoisted(() => ({
  getAllItemsCounts: vi.fn(),
  onUnreadChanged: vi.fn(),
  onEmbeddingsChanged: vi.fn(),
}));
vi.mock("../platform/Platform", () => ({ usePlatform: () => platform }));
vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);

let counts: AllItemsCounts | null = null;
function Capture() {
  const value = useAllItemsCounts();
  useEffect(() => {
    counts = value;
  }, [value]);
  return null;
}

let root: ReturnType<typeof createRoot>;
beforeEach(async () => {
  vi.useFakeTimers();
  vi.clearAllMocks();
  counts = null;
  platform.getAllItemsCounts.mockResolvedValue({
    entryCount: 10,
    unreadCount: 3,
  });
  platform.onUnreadChanged.mockResolvedValue(() => {});
  platform.onEmbeddingsChanged.mockResolvedValue(() => {});
  root = createRoot(document.createElement("div"));
  await act(async () => {
    root.render(
      <AllItemsCountsProvider>
        <Capture />
      </AllItemsCountsProvider>
    );
  });
});

afterEach(async () => {
  await act(async () => root.unmount());
  vi.useRealTimers();
});

it("loads counts when mounted", () => {
  expect(counts).toEqual({ entryCount: 10, unreadCount: 3 });
  expect(platform.getAllItemsCounts).toHaveBeenCalledOnce();
});

it("coalesces unread and embedding changes into one refresh", async () => {
  const unreadChanged = platform.onUnreadChanged.mock.calls[0][0];
  const embeddingsChanged = platform.onEmbeddingsChanged.mock.calls[0][0];

  await act(async () => {
    unreadChanged({});
    embeddingsChanged();
    await vi.advanceTimersByTimeAsync(400);
  });

  expect(platform.getAllItemsCounts).toHaveBeenCalledTimes(2);
});
