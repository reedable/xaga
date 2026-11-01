import { act, useEffect } from "react";
import { createRoot } from "react-dom/client";
import { MemoryRouter } from "react-router-dom";
import { afterEach, expect, it, vi } from "vitest";
import { TagsProvider } from "./TagsProvider";
import { useTags } from "./TagsContext";
import useRouter from "../hooks/useRouter";

const mocks = vi.hoisted(() => {
  const callbacks: Record<string, () => void> = {};
  const register = (name: string) => vi.fn(async (callback: () => void) => {
    callbacks[name] = callback; return () => { delete callbacks[name]; };
  });
  return { callbacks, listTags: vi.fn(), countTagUnread: vi.fn(),
    onTagCreated: register("created"), onUnreadChanged: register("unread"),
    onEmbeddingsChanged: register("embeddings"), pushError: vi.fn() };
});
vi.mock("../platform/Platform", () => ({ usePlatform: () => mocks }));
const sink = { pushError: mocks.pushError };
vi.mock("../hooks/useErrorSink", () => ({ useErrorSink: () => sink }));
vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
let root: ReturnType<typeof createRoot>;
let container: HTMLDivElement;
let router: ReturnType<typeof useRouter>;
function Host() {
  const route = useRouter(); const tags = useTags();
  useEffect(() => { router = route; }, [route]);
  return <div>{tags.tags.map((tag) => tag.name).join(",")}:{tags.unreadCount ?? "loading"}</div>;
}
afterEach(async () => { await act(async () => root.unmount()); container.remove(); });
it("reads definitions from the registry and discards old intersection counts after navigation", async () => {
  mocks.listTags.mockResolvedValue([{ id: "a", name: "Research" }]);
  let oldCount!: (count: number) => void;
  mocks.countTagUnread.mockImplementationOnce(() => new Promise((resolve) => { oldCount = resolve; }));
  mocks.countTagUnread.mockResolvedValue(2);
  container = document.createElement("div"); document.body.append(container); root = createRoot(container);
  await act(async () => root.render(<MemoryRouter initialEntries={[{ pathname: "/", state: { tagIds: ["a"] } }]}>
    <TagsProvider><Host /></TagsProvider>
  </MemoryRouter>));
  await act(async () => router.selectTags(["a", "b"]));
  expect(mocks.countTagUnread).toHaveBeenLastCalledWith(expect.any(AbortSignal), ["a", "b"]);
  await act(async () => oldCount(100));
  expect(container.textContent).toBe("Research:2");
  mocks.listTags.mockResolvedValue([{ id: "a", name: "Renamed" }]);
  // Both registry and count subscriptions receive the same native embedding event.
  await act(async () => mocks.callbacks.created());
  expect(container.textContent).toBe("Renamed:2");
  expect(router.state.tagIds).toEqual(["a", "b"]);
});
