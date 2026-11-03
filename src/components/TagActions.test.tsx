import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { TagActions } from "./TagActions";

const mocks = vi.hoisted(() => ({ renameTag: vi.fn(), pushError: vi.fn(), readOnly: false, renaming: [] as string[] }));
vi.mock("../platform/Platform", () => ({ usePlatform: () => mocks }));
vi.mock("../hooks/useLibraryReadOnly", () => ({ useLibraryReadOnly: () => mocks.readOnly }));
vi.mock("../providers/TagsContext", () => ({ useTags: () => ({ renaming: mocks.renaming }) }));
const sink = { pushError: mocks.pushError };
vi.mock("../hooks/useErrorSink", () => ({ useErrorSink: () => sink }));
vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
let root: ReturnType<typeof createRoot>;
let container: HTMLDivElement;
const tag = { id: "tag-a", name: "Research" };
const button = (text: string) => [...document.querySelectorAll<HTMLButtonElement>("button")].find((item) => item.textContent === text)!;
beforeEach(() => {
  vi.clearAllMocks(); mocks.readOnly = false; mocks.renaming = [];
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
  expect(item.textContent).toBe("Renaming…"); expect(item.getAttribute("aria-disabled")).toBe("true");
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
  expect(item.textContent).toBe("Rename");
  expect(item.getAttribute("aria-disabled")).not.toBe("true");
});
