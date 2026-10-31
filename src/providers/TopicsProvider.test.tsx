import { act, useEffect } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, expect, it, vi } from "vitest";
import { TopicsProvider } from "./TopicsProvider";
import { useTopics } from "../hooks/useTopics";
import type { Entry } from "../types";

const mocks = vi.hoisted(() => ({
  platform: {
    listEntryTopics: vi.fn(async () => [] as string[]),
    addTopicItem: vi.fn(),
    removeTopicItem: vi.fn(),
    createTopicWithMember: vi.fn(),
    onUnreadChanged: vi.fn(async () => () => {}),
    onEmbeddingsChanged: vi.fn(async () => () => {}),
  },
  sidebar: { listTopics: vi.fn() },
  pins: { pins: [], setPin: vi.fn() },
  sink: { pushError: vi.fn() },
}));
vi.mock("../platform/Platform", () => ({ usePlatform: () => mocks.platform }));
vi.mock("../hooks/useSidebar", () => ({ useSidebar: () => mocks.sidebar }));
vi.mock("./PinsContext", () => ({ usePins: () => mocks.pins }));
vi.mock("../hooks/useErrorSink", () => ({ useErrorSink: () => mocks.sink }));
vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
let root: ReturnType<typeof createRoot>;
let hook: ReturnType<typeof useTopics>;
const controller = { current: new AbortController() };
function Consumer() {
  const value = useTopics(controller);
  useEffect(() => {
    hook = value;
  }, [value]);
  return null;
}
afterEach(async () => {
  await act(() => root.unmount());
  vi.clearAllMocks();
});
it("retains committed additions and removals over stale index results across picker remounts", async () => {
  const topic = {
    __kind: "topic",
    id: "topic",
    title: "Topic",
    unreadCount: 0,
    members: [],
  };
  mocks.platform.addTopicItem.mockResolvedValue(topic);
  mocks.platform.removeTopicItem.mockResolvedValue(topic);
  const article = { id: "article" } as Entry;
  root = createRoot(document.createElement("div"));
  await act(() =>
    root.render(
      <TopicsProvider>
        <Consumer />
      </TopicsProvider>
    )
  );
  await act(() => hook.addTopicItem("topic", article));
  expect(await hook.listEntryTopics("article")).toEqual(["topic"]);
  await act(() => root.render(<TopicsProvider>{null}</TopicsProvider>));
  await act(() =>
    root.render(
      <TopicsProvider>
        <Consumer />
      </TopicsProvider>
    )
  );
  expect(await hook.listEntryTopics("article")).toEqual(["topic"]);
  mocks.platform.listEntryTopics.mockResolvedValue(["topic"]);
  await act(() => hook.removeTopicItem("topic", "article"));
  expect(await hook.listEntryTopics("article")).toEqual([]);
  // Another article's indexed memberships are untouched.
  expect(await hook.listEntryTopics("other")).toEqual(["topic"]);
});
