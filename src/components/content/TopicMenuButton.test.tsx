import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { Topic } from "../../types";
import { TopicMenuButton } from "./TopicMenuButton";

const mocks = vi.hoisted(() => ({
  topics: [] as Topic[],
  createTopic: vi.fn(),
  createTopicWithMember: vi.fn(),
  setTopicPinned: vi.fn(),
  listEntryTopics: vi.fn(async () => [] as string[]),
  addTopicItem: vi.fn(),
  removeTopicItem: vi.fn(),
  rankTopicsRelatedToItem: vi.fn(),
}));

vi.mock("../../hooks/useTopics", () => ({
  useTopics: () => mocks,
}));
vi.mock("../../hooks/useEmbeddingStartupState", () => ({
  useEmbeddingStartupState: () => "unavailable",
}));
vi.mock("../../hooks/useEvents", () => ({
  useEvents: () => ({ onAnnotationRemoved: () => () => undefined }),
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
  container = document.createElement("div");
  document.body.append(container);
  root = createRoot(container);
});

afterEach(async () => {
  await act(() => root.unmount());
  document.body.innerHTML = "";
});

it("offers inline topic creation when no topics exist", async () => {
  await act(() => root.render(<TopicMenuButton item={null} />));

  const menuButton = container.querySelector('[aria-label="Add to Topic"]')!;
  expect(menuButton.hasAttribute("disabled")).toBe(false);
  await click(menuButton);

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
  mocks.createTopic.mockResolvedValueOnce(created);
  await setInput(title, "First topic");
  await click(document.body.querySelector('input[type="checkbox"]')!);
  await click(
    [...document.body.querySelectorAll("button")].find(
      (button) => button.textContent === "Create Topic"
    )!
  );
  expect(mocks.createTopic).toHaveBeenCalledWith({
    title: "First topic",
    description: "",
    threshold: 0.8,
  });
  expect(mocks.setTopicPinned).toHaveBeenCalledWith("new", true);

  mocks.topics = [created];
  await act(() => root.render(<TopicMenuButton item={null} />));
  expect(
    (document.body.querySelector('input[type="checkbox"]') as HTMLInputElement)
      .checked
  ).toBe(true);
});

it("prefills an inline topic title from a trimmed no-match filter", async () => {
  mocks.topics = [topic("existing", "Existing")];
  await act(() => root.render(<TopicMenuButton item={null} />));
  await click(container.querySelector('[aria-label="Add to Topic"]')!);

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
