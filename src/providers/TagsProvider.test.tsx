import { act, useEffect } from "react";
import { createRoot } from "react-dom/client";
import { MemoryRouter } from "../test/NavigationRouter";
import { afterEach, expect, it, vi } from "vitest";
import { TagsProvider } from "./TagsProvider";
import { useTags } from "./TagsContext";
import useRouter from "../hooks/useRouter";

const mocks = vi.hoisted(() => {
  const callbacks: Record<string, (value?: unknown) => void> = {};
  const register = (name: string) =>
    vi.fn(async (callback: () => void) => {
      callbacks[name] = callback;
      return () => {
        delete callbacks[name];
      };
    });
  return {
    callbacks,
    readTagStatus: vi.fn(),
    countTagUnread: vi.fn(),
    onTagCreated: register("created"),
    onTagsChanged: register("changed"),
    onUnreadChanged: register("unread"),
    onEmbeddingsChanged: register("embeddings"),
    pushError: vi.fn(),
  };
});
vi.mock("../platform/Platform", () => ({ usePlatform: () => mocks }));
const sink = { pushError: mocks.pushError };
vi.mock("../hooks/useErrorSink", () => ({ useErrorSink: () => sink }));
vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
let root: ReturnType<typeof createRoot>;
let container: HTMLDivElement;
let router: ReturnType<typeof useRouter>;
function Host() {
  const route = useRouter();
  const tags = useTags();
  useEffect(() => {
    router = route;
  }, [route]);
  return (
    <div>
      {tags.tags.map((tag) => tag.name).join(",")}:
      {tags.unreadCount ?? "loading"}
    </div>
  );
}
afterEach(async () => {
  await act(async () => root.unmount());
  container.remove();
});
it("reads definitions from the registry and discards old intersection counts after navigation", async () => {
  mocks.readTagStatus.mockResolvedValue({
    tags: [{ id: "a", name: "Research" }],
    renaming: [],
    deleting: [],
    merging: [],
  });
  let oldCount!: (count: number) => void;
  mocks.countTagUnread.mockImplementationOnce(
    () =>
      new Promise((resolve) => {
        oldCount = resolve;
      })
  );
  mocks.countTagUnread.mockResolvedValue(2);
  container = document.createElement("div");
  document.body.append(container);
  root = createRoot(container);
  await act(async () =>
    root.render(
      <MemoryRouter
        initialEntries={[{ pathname: "/", state: { tagIds: ["a"] } }]}
      >
        <TagsProvider>
          <Host />
        </TagsProvider>
      </MemoryRouter>
    )
  );
  await act(async () => router.selectTags(["a", "b"]));
  expect(mocks.countTagUnread).toHaveBeenLastCalledWith(
    expect.any(AbortSignal),
    ["a", "b"]
  );
  await act(async () => oldCount(100));
  expect(container.textContent).toBe("Research:2");
  mocks.readTagStatus.mockResolvedValue({
    tags: [{ id: "a", name: "Renamed" }],
    renaming: [],
    deleting: [],
    merging: [],
  });
  // Both registry and count subscriptions receive the same native embedding event.
  await act(async () => mocks.callbacks.created());
  expect(container.textContent).toBe("Renamed:2");
  await act(async () =>
    mocks.callbacks.changed({
      tags: [{ id: "a", name: "Renamed" }],
      renaming: ["a"],
      deleting: [],
      merging: [],
    })
  );
  expect(router.state.tagIds).toEqual(["a", "b"]);
});

it("retains deletion selection updates even when completion arrives before rendering", async () => {
  mocks.readTagStatus.mockResolvedValue({
    tags: [{ id: "a", name: "Research" }],
    renaming: [],
    deleting: [],
    merging: [],
  });
  mocks.countTagUnread.mockResolvedValue(0);
  container = document.createElement("div");
  document.body.append(container);
  root = createRoot(container);
  await act(async () =>
    root.render(
      <MemoryRouter
        initialEntries={[
          { pathname: "/", state: { tagIds: ["a", "b"], entryId: "article" } },
        ]}
      >
        <TagsProvider>
          <Host />
        </TagsProvider>
      </MemoryRouter>
    )
  );
  await act(async () => {
    mocks.callbacks.changed({
      tags: [{ id: "a", name: "Research" }],
      renaming: [],
      deleting: ["a"],
      merging: [],
    });
    mocks.callbacks.changed({
      tags: [],
      renaming: [],
      deleting: [],
      merging: [],
    });
  });
  expect(router.state.tagIds).toEqual(["b"]);
  expect(router.state.entryId).toBe("article");
});

it("preserves the reader and redirects both selected participants through successive merges", async () => {
  mocks.readTagStatus.mockResolvedValue({
    tags: [
      { id: "a", name: "Research" },
      { id: "b", name: "Work" },
    ],
    renaming: [],
    deleting: [],
    merging: [],
  });
  mocks.countTagUnread.mockResolvedValue(0);
  container = document.createElement("div");
  document.body.append(container);
  root = createRoot(container);
  await act(async () =>
    root.render(
      <MemoryRouter
        initialEntries={[
          { pathname: "/", state: { tagIds: ["a", "b"], entryId: "article" } },
        ]}
      >
        <TagsProvider>
          <Host />
        </TagsProvider>
      </MemoryRouter>
    )
  );
  await act(async () => {
    mocks.callbacks.changed({
      tags: [{ id: "a", name: "Work" }],
      renaming: [],
      deleting: [],
      merging: [
        {
          sourceId: "a",
          destinationId: "b",
          survivorId: "a",
          destinationName: "Work",
        },
      ],
    });
    mocks.callbacks.changed({
      tags: [{ id: "a", name: "Work" }],
      renaming: [],
      deleting: [],
      merging: [],
    });
  });
  expect(router.state.tagIds).toEqual(["a"]);
  expect(router.state.entryId).toBe("article");
  expect(router.state.tagSelectionRevision).toBeDefined();
  await act(async () =>
    mocks.callbacks.changed({
      tags: [{ id: "c", name: "Projects" }],
      renaming: [],
      deleting: [],
      merging: [
        {
          sourceId: "a",
          destinationId: "c",
          survivorId: "c",
          destinationName: "Projects",
        },
      ],
    })
  );
  expect(router.state.tagIds).toEqual(["c"]);
  expect(router.state.entryId).toBe("article");
});
