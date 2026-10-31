import { act, useState } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { ArticleInspector, type InspectorView } from "./ArticleInspector";

const mocks = vi.hoisted(() => ({ read: vi.fn(), pushError: vi.fn() }));
const platform = { readEntry: mocks.read };
const sink = { pushError: mocks.pushError };
vi.mock("../../platform/Platform", () => ({ usePlatform: () => platform }));
vi.mock("../../hooks/useErrorSink", () => ({ useErrorSink: () => sink }));
vi.mock("./RelatedItemList", () => ({
  RelatedItemList: ({ itemId }: { itemId: string }) => (
    <div>Related to {itemId}</div>
  ),
}));
vi.mock("./TopicMenuButton", () => ({
  ArticleTopicMembership: ({ article }: { article: { id: string } }) => (
    <div>Topics for {article.id}</div>
  ),
}));
vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
let root: ReturnType<typeof createRoot>;
let container: HTMLDivElement;
function Host({ articleId }: { articleId: string }) {
  const [view, setView] = useState<InspectorView>("related");
  return (
    <ArticleInspector
      articleId={articleId}
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
      <ArticleInspector articleId="old" view="topics" onViewChange={() => {}} />
    )
  );
  await act(async () =>
    root.render(
      <ArticleInspector
        articleId="current"
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
      <ArticleInspector
        articleId="article"
        view="topics"
        onViewChange={() => {}}
      />
    )
  );
  expect(mocks.pushError).toHaveBeenCalledWith(failure);
  expect(container.textContent).toContain("Article topics couldn’t be loaded.");
  expect(container.querySelector('[role="status"]')).toBeNull();
});

it("keeps Tags disabled with an accessible explanation", async () => {
  await act(async () => root.render(<Host articleId="article" />));
  expect(tab("Tags").disabled).toBe(true);
  const explanation = document.getElementById(
    tab("Tags").getAttribute("aria-describedby")!
  );
  expect(explanation?.textContent).toBe("Tag assignment isn’t available yet.");
});
