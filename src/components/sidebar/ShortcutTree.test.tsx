import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import ShortcutTree from "./ShortcutTree";
import type { PinTarget } from "../../pins";
import type { ShortcutNode } from "../../utils/projectPinTree";

const mocks = vi.hoisted(() => ({
  organizePins: vi.fn(),
  setPin: vi.fn(),
  readOnly: false,
  pushError: vi.fn(),
  getLibrary: vi.fn(),
  getViewState: vi.fn(),
  setViewState: vi.fn(),
}));
vi.mock("../../providers/PinsContext", () => ({ usePins: () => mocks }));
vi.mock("../../platform/Platform", () => ({ usePlatform: () => mocks }));
vi.mock("../../hooks/useLibraryReadOnly", () => ({
  useLibraryReadOnly: () => mocks.readOnly,
}));
vi.mock("../../hooks/useErrorSink", () => ({ useErrorSink: () => mocks }));
vi.mock("../../storage/viewState", () => mocks);
vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
const leaf: ShortcutNode = {
  reference: { type: "entry", id: "article" },
  title: "Article",
  unreadCount: 0,
};
const nodes: ShortcutNode[] = [
  {
    reference: { type: "group", id: "outer" },
    title: "Research",
    unreadCount: 3,
    children: [
      {
        reference: { type: "group", id: "inner" },
        title: "Background",
        unreadCount: 3,
        children: [leaf],
      },
    ],
  },
];
let root: ReturnType<typeof createRoot>;
let container: HTMLDivElement;
let filter: string;
let createRequest: number;
const render = (items: ShortcutNode[] = nodes) =>
  act(async () =>
    root.render(
      <ShortcutTree
        nodes={items}
        filter={filter}
        createRequest={createRequest}
        renderTarget={(node) => <button>{node.title}</button>}
      />
    )
  );
const button = (label: string) =>
  document.querySelector<HTMLButtonElement>(`[aria-label="${label}"]`)!;
async function click(element: HTMLElement) {
  await act(async () => element.click());
}
async function groupAction(title: string, action: string) {
  await click(button(`Actions for group ${title}`));
  const item = [
    ...document.querySelectorAll<HTMLElement>('[role="menuitem"]'),
  ].find((item) => item.textContent === action)!;
  await click(item);
}
beforeEach(() => {
  vi.clearAllMocks();
  mocks.readOnly = false;
  mocks.getLibrary.mockResolvedValue("/library");
  mocks.getViewState.mockResolvedValue({});
  mocks.setViewState.mockResolvedValue(undefined);
  mocks.organizePins.mockResolvedValue(undefined);
  filter = "";
  createRequest = 0;
  container = document.createElement("div");
  document.body.append(container);
  root = createRoot(container);
});
afterEach(async () => {
  await act(async () => root.unmount());
  document.body.innerHTML = "";
});
it("remembers collapse per library and temporarily reveals filtered descendants", async () => {
  await render();
  const group = container.querySelector<HTMLElement>("[aria-expanded]")!;
  await click(group);
  expect(group.getAttribute("aria-expanded")).toBe("false");
  expect(mocks.setViewState).toHaveBeenLastCalledWith("pins.groups./library", {
    outer: false,
  });
  filter = "Article";
  await render();
  expect(
    container.querySelector("[aria-expanded]")?.getAttribute("aria-expanded")
  ).toBe("true");
  expect(container.textContent).toContain("Article");
  await click(container.querySelector<HTMLElement>("[aria-expanded]")!);
  expect(mocks.setViewState).toHaveBeenLastCalledWith("pins.groups./library", {
    outer: false,
  });
  filter = "";
  await render();
  expect(
    container.querySelector("[aria-expanded]")?.getAttribute("aria-expanded")
  ).toBe("false");
});
it("offers root but excludes the moved group and descendants from keyboard destinations", async () => {
  await render();
  await groupAction("Research", "Move to…");
  const select = document.querySelector<HTMLElement>('[role="combobox"]')!;
  expect(select.textContent).toBe("Top level of Shortcuts");
  await act(async () =>
    select.dispatchEvent(
      new MouseEvent("mousedown", { bubbles: true, button: 0 })
    )
  );
  expect(document.querySelector('[role="listbox"]')?.textContent).toBe(
    "Top level of Shortcuts"
  );
  const option = document.querySelector<HTMLElement>('[role="option"]')!;
  await click(option);
  expect(select.textContent).toBe("Top level of Shortcuts");
  const move = [...document.querySelectorAll<HTMLButtonElement>("button")].find(
    (button) => button.textContent === "Move"
  )!;
  await click(move);
  expect(mocks.organizePins).toHaveBeenCalledWith({
    action: "move",
    node: { type: "group", id: "outer" },
    parent: null,
  });
});
it("keeps failed mutations visible and disables organization in read-only mode", async () => {
  await render();
  const error = new Error("failed save");
  mocks.organizePins.mockRejectedValueOnce(error);
  await groupAction("Research", "Remove group");
  expect(mocks.pushError).toHaveBeenCalledWith(error);
  expect(container.textContent).toContain("Research");
  mocks.readOnly = true;
  await render();
  expect(button("Actions for group Research").disabled).toBe(true);
  await click(container.querySelector<HTMLElement>("[aria-expanded]")!);
  expect(
    container.querySelector("[aria-expanded]")?.getAttribute("aria-expanded")
  ).toBe("false");
});
it("opens root creation and saves a trimmed title with stable identity", async () => {
  await render();
  createRequest = 1;
  await render();
  const input = document.querySelector<HTMLInputElement>("input")!;
  await act(async () => {
    Object.getOwnPropertyDescriptor(
      HTMLInputElement.prototype,
      "value"
    )!.set!.call(input, " Research ");
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
  const form = document.querySelector("form")!;
  await act(async () =>
    form.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }))
  );
  expect(mocks.organizePins).toHaveBeenCalledWith({
    action: "createGroup",
    id: expect.any(String),
    title: "Research",
    parent: null,
  });
});

it("accepts a drop into a collapsed group, expands it after saving, and keeps root moves available", async () => {
  await render();
  await click(container.querySelector<HTMLElement>("[aria-expanded]")!);
  const groupRow =
    container.querySelector<HTMLElement>("[aria-expanded]")!.parentElement!
      .parentElement!;
  const transfer = {
    types: ["application/x-xaga-pin"],
    getData: () => JSON.stringify({ type: "entry", id: "article" }),
    dropEffect: "none",
  };
  const event = new Event("drop", { bubbles: true, cancelable: true });
  Object.defineProperty(event, "dataTransfer", { value: transfer });
  await act(async () => groupRow.dispatchEvent(event));
  expect(mocks.organizePins).toHaveBeenCalledWith({
    action: "move",
    node: { type: "entry", id: "article" },
    parent: "outer",
  });
  expect(
    container.querySelector("[aria-expanded]")?.getAttribute("aria-expanded")
  ).toBe("true");
});

it("opens a single group action menu with visible labels and keyboard dismissal", async () => {
  await render();
  const trigger = button("Actions for group Research");
  trigger.focus();
  await act(async () =>
    trigger.dispatchEvent(
      new KeyboardEvent("keydown", { key: "Enter", bubbles: true })
    )
  );
  // Native button activation produces a click after Enter in the browser.
  await click(trigger);
  expect(trigger.getAttribute("aria-expanded")).toBe("true");
  expect(
    [...document.querySelectorAll('[role="menuitem"]')].map(
      (item) => item.textContent
    )
  ).toEqual(["Add group", "Rename group", "Move to…", "Remove group"]);
  const menu = document.querySelector<HTMLElement>('[role="menu"]')!;
  await act(async () =>
    menu.dispatchEvent(
      new KeyboardEvent("keydown", { key: "Escape", bubbles: true })
    )
  );
  expect(trigger.getAttribute("aria-expanded")).toBe("false");
  expect(document.activeElement).toBe(trigger);
  expect(mocks.organizePins).not.toHaveBeenCalled();
});
it("opens the group's rename dialog from its labeled action", async () => {
  await render();
  await groupAction("Research", "Rename group");
  expect(document.querySelector('[role="dialog"]')?.textContent).toContain(
    "Rename group"
  );
  expect(document.querySelector<HTMLInputElement>("input")?.value).toBe(
    "Research"
  );
});

it.each<PinTarget>([
  { type: "feed", id: "feed" },
  { type: "topic", id: "topic" },
  { type: "entry", id: "article" },
  { type: "site", hostname: "example.com" },
])(
  "offers labeled actions and unpins the typed $type target",
  async (reference) => {
    mocks.setPin.mockResolvedValue(undefined);
    await render([{ reference, title: "Pinned target", unreadCount: 3 }]);
    const trigger = button("Actions for shortcut Pinned target");
    trigger.focus();
    await click(trigger);
    expect(trigger.getAttribute("aria-expanded")).toBe("true");
    const items = [
      ...document.querySelectorAll<HTMLElement>('[role="menuitem"]'),
    ];
    expect(items.map((item) => item.textContent)).toEqual([
      "Move to…",
      "Remove shortcut",
    ]);
    await click(items[1]);
    expect(mocks.setPin).toHaveBeenCalledWith(reference, false);
    expect(mocks.organizePins).not.toHaveBeenCalled();
  }
);
it("moves a shortcut through its menu and disables that menu when read-only", async () => {
  await render();
  await click(button("Actions for shortcut Article"));
  await click(
    [...document.querySelectorAll<HTMLElement>('[role="menuitem"]')].find(
      (item) => item.textContent === "Move to…"
    )!
  );
  const move = [...document.querySelectorAll<HTMLButtonElement>("button")].find(
    (item) => item.textContent === "Move"
  )!;
  await click(move);
  expect(mocks.organizePins).toHaveBeenCalledWith({
    action: "move",
    node: { type: "entry", id: "article" },
    parent: null,
  });
  mocks.readOnly = true;
  await render();
  expect(button("Actions for shortcut Article").disabled).toBe(true);
});

it("keeps descendant elements mounted when their group collapses and reopens", async () => {
  await render();
  const article = [...container.querySelectorAll("button")].find(
    (item) => item.textContent === "Article"
  )!;
  const group = container.querySelector<HTMLElement>("[aria-expanded]")!;
  await click(group);
  expect(group.getAttribute("aria-expanded")).toBe("false");
  expect(container.contains(article)).toBe(true);
  await click(group);
  expect(group.getAttribute("aria-expanded")).toBe("true");
  expect(
    [...container.querySelectorAll("button")].find(
      (item) => item.textContent === "Article"
    )
  ).toBe(article);
});
