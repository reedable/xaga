import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { Annotation, Entry, Topic } from "../../types";
import { TopicMembership } from "./TopicMembership";

const mocks = vi.hoisted(() => ({
  readOnly: false,
  hasLoaded: true,
  topics: [] as Topic[],
  createTopicWithMember: vi.fn(),
  setTopicPinned: vi.fn(),
  listEntryTopics: vi.fn(async () => [] as string[]),
  addTopicItem: vi.fn(),
  removeTopicItem: vi.fn(),
  rankTopicsRelatedToItem: vi.fn(),
}));

vi.mock("../../hooks/useLibraryReadOnly", () => ({
  useLibraryReadOnly: () => mocks.readOnly,
}));
vi.mock("../../hooks/useTopics", () => ({
  useTopics: () => mocks,
}));
vi.mock("../../hooks/useEmbeddingStartupState", () => ({
  useEmbeddingStartupState: () => "unavailable",
}));
vi.mock("framer-motion", () => ({
  motion: (Component: React.ElementType) =>
    function MotionComponent(props: Record<string, unknown>) {
      const componentProps = { ...props };
      delete componentProps.layout;
      delete componentProps.transition;
      return <Component {...componentProps} />;
    },
}));

vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);

let root: ReturnType<typeof createRoot>;
let container: HTMLDivElement;

const click = async (element: Element) => {
  await act(async () =>
    element.dispatchEvent(new MouseEvent("click", { bubbles: true }))
  );
};

const setInput = async (input: Element, value: string) => {
  await act(async () => {
    Object.getOwnPropertyDescriptor(
      HTMLInputElement.prototype,
      "value"
    )!.set!.call(input, value);
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
};

const topic = (id: string, title: string): Topic => ({
  __kind: "topic",
  id,
  title,
  createdAt: "2026-01-01T00:00:00Z",
  entryCount: 0,
  unreadCount: 0,
  isPinned: false,
  members: [],
});

beforeEach(() => {
  vi.clearAllMocks();
  mocks.topics = [];
  mocks.readOnly = false;
  mocks.hasLoaded = true;
  container = document.createElement("div");
  document.body.append(container);
  root = createRoot(container);
});

afterEach(async () => {
  await act(async () => root.unmount());
  document.body.innerHTML = "";
});

it("offers inline topic creation when no topics exist", async () => {
  await act(async () => root.render(<TopicMembership item={article} />));

  const createAction = [...document.body.querySelectorAll("button")].find(
    (button) => button.textContent === "Create a New Topic"
  )!;
  await click(createAction);

  const title = document.body.querySelector(
    "input[required]"
  ) as HTMLInputElement;
  expect(title.value).toBe("");
  expect(document.body.textContent).toContain("Description");
  expect(document.body.textContent).toContain("Similarity threshold");
  expect(document.body.textContent).toContain("Add to Shortcuts");

  const created = topic("new", "First topic");
  mocks.createTopicWithMember.mockResolvedValueOnce(created);
  await setInput(title, "First topic");
  await click(document.body.querySelector('input[type="checkbox"]')!);
  await click(
    [...document.body.querySelectorAll("button")].find(
      (button) => button.textContent === "Create Topic"
    )!
  );
  expect(mocks.createTopicWithMember).toHaveBeenCalledWith(
    {
      title: "First topic",
      description: "",
      threshold: 0.8,
    },
    article
  );
  expect(mocks.setTopicPinned).toHaveBeenCalledWith("new", true);

  mocks.listEntryTopics.mockResolvedValueOnce(["new"]);
  mocks.topics = [created];
  await act(async () => root.render(<TopicMembership item={article} />));
  expect(
    (document.body.querySelector('input[type="checkbox"]') as HTMLInputElement)
      .checked
  ).toBe(true);
});

it("prefills an inline topic title from a trimmed no-match filter", async () => {
  mocks.topics = [topic("existing", "Existing")];
  await act(async () => root.render(<TopicMembership item={article} />));

  const filter = document.body.querySelector(
    'input[placeholder="Filter topics…"]'
  )!;
  await setInput(filter, "  New topic  ");

  const createAction = [...document.body.querySelectorAll("button")].find(
    (button) => button.textContent === "Create a New Topic"
  )!;
  await click(createAction);
  expect(
    (document.body.querySelector("input[required]") as HTMLInputElement).value
  ).toBe("New topic");
});

const article = {
  __kind: "entry",
  id: "parent-article",
  title: "Article",
  createdAt: "2026-01-01",
  url: "https://example.org",
} as Entry;

it("edits article memberships inline and preserves checked state after a failed mutation", async () => {
  mocks.topics = [topic("one", "One")];
  mocks.addTopicItem
    .mockResolvedValueOnce(undefined)
    .mockResolvedValueOnce(mocks.topics[0]);
  mocks.removeTopicItem.mockResolvedValue(mocks.topics[0]);
  await act(async () => root.render(<TopicMembership item={article} />));
  const row = () => container.querySelector('[role="checkbox"]')!;
  expect(row().getAttribute("aria-checked")).toBe("false");
  await click(row());
  expect(row().getAttribute("aria-checked")).toBe("false");
  await click(row());
  expect(mocks.addTopicItem).toHaveBeenLastCalledWith("one", article);
  expect(row().getAttribute("aria-checked")).toBe("true");
  await click(row());
  expect(mocks.removeTopicItem).toHaveBeenCalledWith("one", "parent-article");
  expect(row().getAttribute("aria-checked")).toBe("false");
  expect(document.querySelector('[role="dialog"]')).toBeNull();
});

it("allows read-only membership inspection without enabling assignment or creation", async () => {
  mocks.readOnly = true;
  mocks.topics = [topic("one", "One")];
  mocks.listEntryTopics.mockResolvedValueOnce(["one"]);
  await act(async () => root.render(<TopicMembership item={article} />));
  const row = container.querySelector('[role="checkbox"]')!;
  expect(row.getAttribute("aria-checked")).toBe("true");
  expect(row.getAttribute("aria-disabled")).toBe("true");
  await click(row);
  expect(mocks.removeTopicItem).not.toHaveBeenCalled();
  mocks.topics = [];
  await act(async () => root.render(<TopicMembership item={article} />));
  const create = [...container.querySelectorAll("button")].find(
    (button) => button.textContent === "Create a New Topic"
  )!;
  expect(create.disabled).toBe(true);
});

it("does not apply a late membership mutation to a different article", async () => {
  mocks.topics = [topic("one", "One")];
  let finish!: (value: Topic) => void;
  mocks.addTopicItem.mockImplementationOnce(
    () =>
      new Promise((resolve) => {
        finish = resolve;
      })
  );
  await act(async () => root.render(<TopicMembership item={article} />));
  await click(container.querySelector('[role="checkbox"]')!);
  await act(async () =>
    root.render(<TopicMembership item={{ ...article, id: "other" }} />)
  );
  await act(async () => {
    finish(mocks.topics[0]);
  });
  expect(
    container.querySelector('[role="checkbox"]')?.getAttribute("aria-checked")
  ).toBe("false");
});

it("adds and removes the Annotation itself as a typed Topic member", async () => {
  const annotation = {
    __kind: "annotation",
    id: "annotation",
    reference: article.id,
    title: "Passage",
  } as Annotation;
  mocks.topics = [topic("one", "One")];
  mocks.addTopicItem.mockResolvedValue(mocks.topics[0]);
  mocks.removeTopicItem.mockResolvedValue(mocks.topics[0]);
  await act(async () => root.render(<TopicMembership item={annotation} />));
  await click(container.querySelector('[role="checkbox"]')!);
  expect(mocks.addTopicItem).toHaveBeenLastCalledWith("one", annotation);
  await click(container.querySelector('[role="checkbox"]')!);
  expect(mocks.removeTopicItem).toHaveBeenLastCalledWith("one", "annotation");
});

it("shows loading instead of topic creation until the first list arrives", async () => {
  mocks.hasLoaded = false;
  await act(async () => root.render(<TopicMembership item={article} />));
  expect(container.querySelector('[role="status"]')?.textContent).toBe(
    "Loading Topics…"
  );
  expect(container.textContent).not.toContain("Create a New Topic");
  mocks.topics = [topic("one", "One")];
  mocks.hasLoaded = true;
  await act(async () => root.render(<TopicMembership item={article} />));
  expect(container.querySelector('[role="status"]')).toBeNull();
  expect(container.textContent).toContain("One");
  expect(container.textContent).not.toContain("Create a New Topic");
});

it("offers creation only once loading confirms an empty library", async () => {
  mocks.hasLoaded = false;
  await act(async () => root.render(<TopicMembership item={article} />));
  expect(container.textContent).not.toContain("Create a New Topic");
  mocks.hasLoaded = true;
  await act(async () => root.render(<TopicMembership item={article} />));
  expect(container.querySelector('[role="status"]')).toBeNull();
  expect(container.textContent).toContain("Create a New Topic");
});
