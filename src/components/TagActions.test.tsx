import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { TagActions } from "./TagActions";

const mocks = vi.hoisted(() => ({ readTagStatus: vi.fn(), mergeTags: vi.fn(), renameTag: vi.fn(), deleteTag: vi.fn(), deleting: [] as string[], pushError: vi.fn(), readOnly: false, renaming: [] as string[] }));
vi.mock("../platform/Platform", () => ({ usePlatform: () => mocks }));
vi.mock("../hooks/useLibraryReadOnly", () => ({ useLibraryReadOnly: () => mocks.readOnly }));
vi.mock("../providers/TagsContext", () => ({ useTags: () => ({ renaming: mocks.renaming, deleting: mocks.deleting }) }));
const sink = { pushError: mocks.pushError };
vi.mock("../hooks/useErrorSink", () => ({ useErrorSink: () => sink }));
vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
let root: ReturnType<typeof createRoot>;
let container: HTMLDivElement;
const tag = { id: "tag-a", name: "Research" };
const button = (text: string) => [...document.querySelectorAll<HTMLButtonElement>("button")].find((item) => item.textContent === text)!;
beforeEach(() => {
  vi.clearAllMocks(); mocks.readOnly = false; mocks.renaming = []; mocks.deleting = [];
  mocks.readTagStatus.mockResolvedValue({ tags: [tag], renaming: [], deleting: [], merging: [] });
  mocks.mergeTags.mockResolvedValue({ tags: [], renaming: [], deleting: [], merging: [] });
  mocks.deleteTag.mockResolvedValue({ tags: [tag], renaming: [], deleting: [tag.id] });
  mocks.renameTag.mockResolvedValue({ tags: [{ ...tag, name: "Changed" }], renaming: [tag.id] });
  container = document.createElement("div"); document.body.append(container); root = createRoot(container);
});
afterEach(async () => { await act(async () => root.unmount()); container.remove(); });
async function menu() {
  await act(async () => root.render(<TagActions tag={tag} />));
  await act(async () => container.querySelector<HTMLButtonElement>("button")!.click());
}
async function dialog() {
  await menu();
  await act(async () => (document.querySelector('[role="menuitem"]') as HTMLElement).click());
}
async function name(value: string) {
  await act(async () => {
    const input = document.querySelector<HTMLInputElement>('input')!;
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, value);
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
}
it("prefills the dialog, submits its form, and keeps a collision inline", async () => {
  await dialog(); expect(document.querySelector<HTMLInputElement>("input")!.value).toBe("Research");
  await name(" Other ");
  mocks.renameTag.mockRejectedValueOnce({ code: "tags:collision", message: "A tag already uses this name.", detail: "" });
  await act(async () => document.querySelector("form")!.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true })));
  expect(mocks.renameTag).toHaveBeenCalledWith(tag.id, "Other");
  expect(document.body.textContent).toContain("A tag already uses this name.");
  expect(document.querySelector('[role="dialog"]')).not.toBeNull();
  await name("Changed");
  await act(async () => button("Rename").click());
  expect(mocks.renameTag).toHaveBeenLastCalledWith(tag.id, "Changed");
  expect(mocks.pushError).not.toHaveBeenCalled();
});
it("cancels without mutation and returns focus to the actions control", async () => {
  await dialog();
  await act(async () => button("Cancel").click());
  expect(mocks.renameTag).not.toHaveBeenCalled();
  await act(async () => { await new Promise((resolve) => setTimeout(resolve, 20)); });
  expect(document.activeElement).toBe(container.querySelector("button"));
});
it("refuses another rename while transitioning and in read-only mode", async () => {
  mocks.renaming = [tag.id]; await menu();
  const item = document.querySelector('[role="menuitem"]')!;
  expect(item.textContent).toBe("Renaming “Research”…"); expect(item.getAttribute("aria-disabled")).toBe("true");
  await act(async () => container.querySelector<HTMLButtonElement>("button")!.click());
  mocks.renaming = []; mocks.readOnly = true;
  await act(async () => root.render(<TagActions tag={tag} />));
  expect(document.querySelector('[role="menuitem"]')?.getAttribute("aria-disabled")).toBe("true");
});

it("enables another rename as soon as indexed work completes", async () => {
  mocks.renaming = [tag.id]; await menu();
  expect(document.querySelector('[role="menuitem"]')?.getAttribute("aria-disabled")).toBe("true");
  mocks.renaming = [];
  await act(async () => root.render(<TagActions tag={tag} />));
  const item = document.querySelector('[role="menuitem"]')!;
  expect(item.textContent).toBe("Rename “Research”");
  expect(item.getAttribute("aria-disabled")).not.toBe("true");
});

it("confirms deletion once, preserves the tag on cancellation, and restores focus", async () => {
  await menu();
  const deleteItem = () => [...document.querySelectorAll<HTMLElement>('[role="menuitem"]')].find((item) => item.textContent === "Delete “Research”")!;
  await act(async () => deleteItem().click());
  expect(document.body.textContent).toContain("Articles will remain intact.");
  await act(async () => button("Cancel").click());
  expect(mocks.deleteTag).not.toHaveBeenCalled();
  await act(async () => { await new Promise((resolve) => setTimeout(resolve, 20)); });
  expect(document.activeElement).toBe(container.querySelector("button"));
  await act(async () => container.querySelector<HTMLButtonElement>("button")!.click());
  await act(async () => deleteItem().click());
  await act(async () => button("Delete").click());
  expect(mocks.deleteTag).toHaveBeenCalledExactlyOnceWith(tag.id);
});

it("disables conflicting actions while deleting and refuses read-only deletion", async () => {
  mocks.deleting = [tag.id]; await menu();
  let items = [...document.querySelectorAll<HTMLElement>('[role="menuitem"]')];
  expect(items.map((item) => item.getAttribute("aria-disabled"))).toEqual(["true", "true"]);
  expect(items[1].textContent).toBe("Deleting “Research”…");
  mocks.deleting = []; mocks.readOnly = true;
  await act(async () => root.render(<TagActions tag={tag} />));
  items = [...document.querySelectorAll<HTMLElement>('[role="menuitem"]')];
  await act(async () => items[1].click());
  expect(document.querySelector('[role="dialog"]')).toBeNull();
  expect(mocks.deleteTag).not.toHaveBeenCalled();
});

async function collision() {
  mocks.readTagStatus.mockResolvedValue({ tags: [tag, { id: "tag-b", name: "Work" }], renaming: [], deleting: [], merging: [] });
  mocks.renameTag.mockRejectedValueOnce({ code: "tags:collision", message: "A tag already uses this name.", detail: "" });
  await dialog(); await name("Work");
  await act(async () => button("Rename").click());
}
it("requires explicit merge confirmation after a collision", async () => {
  await collision();
  expect(document.body.textContent).toContain("Combine “Research” and “Work” into a single tag named “Work”.");
  expect(document.querySelector('[role="dialog"] strong')?.textContent).toBe("“Work”");
  expect(document.querySelector('[role="dialog"]')?.textContent).toContain("This cannot be undone.");
  expect(mocks.mergeTags).not.toHaveBeenCalled();
  await act(async () => button("Merge").click());
  expect(mocks.mergeTags).toHaveBeenCalledExactlyOnceWith("tag-a", "tag-b");
});
it("cancels a proposed merge without mutation", async () => {
  await collision();
  await act(async () => button("Cancel").click());
  expect(mocks.mergeTags).not.toHaveBeenCalled();
});
it("waits for a destination transition and keeps a racing busy response inline", async () => {
  await collision();
  mocks.renaming = ["tag-b"];
  await act(async () => root.render(<TagActions tag={tag} />));
  expect(button("Merge").disabled).toBe(true);
  mocks.renaming = [];
  await act(async () => root.render(<TagActions tag={tag} />));
  mocks.mergeTags.mockRejectedValueOnce({ code: "tags:busy", message: "Tag changes are still running.", detail: "" });
  await act(async () => button("Merge").click());
  expect(document.querySelector('[role="alert"]')?.textContent).toContain("Tag changes are still running.");
  expect(mocks.pushError).not.toHaveBeenCalled();
});
