import { act, useState } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { Inspector, type InspectorView } from "./Inspector";

const mocks = vi.hoisted(() => ({
  read: vi.fn(),
  readArticle: vi.fn(),
  pushError: vi.fn(),
}));
const platform = { readEntry: mocks.read, readArticleScope: mocks.readArticle };
const sink = { pushError: mocks.pushError };
vi.mock("../../platform/Platform", () => ({ usePlatform: () => platform }));
vi.mock("../../hooks/useErrorSink", () => ({ useErrorSink: () => sink }));
vi.mock("./RelatedItemList", () => ({
  RelatedItemList: ({ itemId }: { itemId: string }) => (
    <div>Related to {itemId}</div>
  ),
}));
vi.mock("./TopicMembership", () => ({
  TopicMembership: ({ item }: { item: { id: string } }) => (
    <div>Topics for {item.id}</div>
  ),
}));
vi.mock("./EntryTags", () => ({ EntryTags: ({ entryId }: { entryId: string }) => <div>Tags for {entryId}</div> }));
vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
let root: ReturnType<typeof createRoot>;
let container: HTMLDivElement;
function Host({ articleId }: { articleId: string }) {
  const [view, setView] = useState<InspectorView>("related");
  return (
    <Inspector
      target={{ entryId: articleId }}
      view={view}
      onViewChange={setView}
    />
  );
}
beforeEach(() => {
  vi.clearAllMocks();
  mocks.read.mockImplementation(async (_signal, id) => ({ id }));
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
});
afterEach(async () => {
  await act(async () => root.unmount());
  container.remove();
});
const tab = (name: string) =>
  [...container.querySelectorAll<HTMLButtonElement>('[role="tab"]')].find(
    (button) => button.textContent === name
  )!;

it("switches views by keyboard and retains the selection across article navigation", async () => {
  await act(async () => root.render(<Host articleId="parent-article" />));
  expect(container.textContent).toContain("Related to parent-article");
  await act(async () => {
    tab("Related").focus();
    tab("Related").dispatchEvent(
      new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true })
    );
  });
  expect(tab("Topics").getAttribute("aria-selected")).toBe("true");
  const panel = document.getElementById(
    tab("Topics").getAttribute("aria-controls")!
  );
  expect(panel?.getAttribute("aria-labelledby")).toBe(tab("Topics").id);
  expect(panel?.hidden).toBe(false);
  expect(container.textContent).toContain("Topics for parent-article");
  await act(async () => root.render(<Host articleId="another-article" />));
  expect(tab("Topics").getAttribute("aria-selected")).toBe("true");
  expect(container.textContent).toContain("Topics for another-article");
});

it("discards a late article lookup after navigation", async () => {
  let finish!: (article: { id: string }) => void;
  mocks.read.mockImplementationOnce(
    () =>
      new Promise((resolve) => {
        finish = resolve;
      })
  );
  await act(async () =>
    root.render(
      <Inspector
        target={{ entryId: "old" }}
        view="topics"
        onViewChange={() => {}}
      />
    )
  );
  await act(async () =>
    root.render(
      <Inspector
        target={{ entryId: "current" }}
        view="topics"
        onViewChange={() => {}}
      />
    )
  );
  await act(async () => {
    finish({ id: "old" });
  });
  expect(container.textContent).toContain("Topics for current");
  expect(container.textContent).not.toContain("Topics for old");
});

it("reports a failed article lookup without leaving a loading status indefinitely", async () => {
  const failure = new Error("unavailable");
  mocks.read.mockRejectedValueOnce(failure);
  await act(async () =>
    root.render(
      <Inspector
        target={{ entryId: "article" }}
        view="topics"
        onViewChange={() => {}}
      />
    )
  );
  expect(mocks.pushError).toHaveBeenCalledWith(failure);
  expect(container.textContent).toContain("Article topics couldn’t be loaded.");
  expect(container.querySelector('[role="status"]')).toBeNull();
});

it("exposes the Article Tags view", async () => {
  await act(async () => root.render(<Host articleId="article" />));
  expect(tab("Tags")).toBeTruthy();
});

it("reads Annotation Topics from its parent file without substituting Article memberships", async () => {
  mocks.readArticle.mockResolvedValue({
    annotations: [{ id: "annotation", reference: "parent" }],
  });
  const target = { entryId: "parent", annotationId: "annotation" };
  await act(async () =>
    root.render(
      <Inspector target={target} view="related" onViewChange={() => {}} />
    )
  );
  expect(container.textContent).toContain("Related to annotation");
  expect(tab("Tags")).toBeUndefined();
  await act(async () =>
    root.render(
      <Inspector target={target} view="topics" onViewChange={() => {}} />
    )
  );
  expect(mocks.readArticle).toHaveBeenCalledWith(
    expect.any(AbortSignal),
    "parent"
  );
  expect(mocks.read).not.toHaveBeenCalled();
  expect(container.textContent).toContain("Topics for annotation");
  expect(container.textContent).not.toContain("Topics for parent");
});

it("does not fall back to Article memberships when an Annotation is missing", async () => {
  mocks.readArticle.mockResolvedValue({ annotations: [] });
  await act(async () =>
    root.render(
      <Inspector
        target={{ entryId: "parent", annotationId: "missing" }}
        view="topics"
        onViewChange={() => {}}
      />
    )
  );
  expect(container.textContent).toContain(
    "Annotation topics couldn’t be loaded."
  );
  expect(mocks.read).not.toHaveBeenCalled();
});

it("preserves both tab contents and avoids repeating the target lookup on tab switches", async () => {
  await act(async () => root.render(<Host articleId="article" />));
  const panels = [
    ...container.querySelectorAll<HTMLElement>('[role="tabpanel"]'),
  ];
  const relatedContent = panels[0].firstElementChild;
  const topicContent = panels[1].firstElementChild;
  expect(relatedContent).not.toBeNull();
  expect(topicContent).not.toBeNull();
  expect(mocks.read).toHaveBeenCalledTimes(1);
  await act(async () => tab("Topics").click());
  await act(async () => tab("Related").click());
  await act(async () => tab("Topics").click());
  expect(panels[0].firstElementChild).toBe(relatedContent);
  expect(panels[1].firstElementChild).toBe(topicContent);
  expect(mocks.read).toHaveBeenCalledTimes(1);
});
