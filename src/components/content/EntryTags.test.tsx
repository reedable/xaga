import { act } from "react";
import { createRoot } from "react-dom/client";
import { beforeEach, afterEach, expect, it, vi } from "vitest";
import type { EntryTagChange, Tag } from "../../domain";
import { EntryTags } from "./EntryTags";

const mocks = vi.hoisted(() => ({
  readOnly: false, listTags: vi.fn(), readEntryTags: vi.fn(), createTag: vi.fn(),
  setEntryTag: vi.fn(), onTagCreated: vi.fn(), onEntryTagsChanged: vi.fn(), pushError: vi.fn(),
}));
vi.mock("../../platform/Platform", () => ({ usePlatform: () => mocks }));
const sink = { pushError: mocks.pushError };
vi.mock("../../hooks/useErrorSink", () => ({ useErrorSink: () => sink }));
vi.mock("../../hooks/useLibraryReadOnly", () => ({ useLibraryReadOnly: () => mocks.readOnly }));
vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
const research = { id: "research", name: "Research" };
const work = { id: "work", name: "Work" };
let container: HTMLDivElement;
let root: ReturnType<typeof createRoot>;
let changeListener: (change: EntryTagChange) => void;
let createdListener: (tag: Tag) => void;
let cleanup: ReturnType<typeof vi.fn>;
beforeEach(() => {
  vi.clearAllMocks(); mocks.readOnly = false;
  mocks.listTags.mockResolvedValue([research, work]);
  mocks.readEntryTags.mockResolvedValue([research.id]);
  mocks.setEntryTag.mockImplementation(async (entryId, tagId, assigned) => ({ entryId, tagId, assigned }));
  mocks.createTag.mockResolvedValue({ id: "new", name: "New" });
  cleanup = vi.fn();
  mocks.onTagCreated.mockImplementation(async (listener) => { createdListener = listener; return cleanup; });
  mocks.onEntryTagsChanged.mockImplementation(async (listener) => { changeListener = listener; return cleanup; });
  container = document.createElement("div"); document.body.append(container);
  root = createRoot(container);
});
afterEach(async () => { await act(async () => root.unmount()); container.remove(); });
async function render(entryId = "article") {
  await act(async () => root.render(<EntryTags key={entryId} entryId={entryId} />));
}
const chip = (name: string) => [...container.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === name)!;
async function search(value: string) {
  const input = container.querySelector("input")!;
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, value);
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
}
it("adds and removes immediately while failed writes retain selection", async () => {
  await render();
  expect(chip("Research").getAttribute("aria-pressed")).toBe("true");
  expect(chip("Work").getAttribute("aria-pressed")).toBe("false");
  await act(async () => chip("Work").click());
  expect(mocks.setEntryTag).toHaveBeenCalledWith("article", "work", true);
  expect(chip("Work").getAttribute("aria-pressed")).toBe("true");
  await act(async () => chip("Research").click());
  expect(mocks.setEntryTag).toHaveBeenLastCalledWith("article", "research", false);
  expect(chip("Research").getAttribute("aria-pressed")).toBe("false");
  mocks.setEntryTag.mockRejectedValueOnce(new Error("write failed"));
  await act(async () => chip("Work").click());
  expect(chip("Work").getAttribute("aria-pressed")).toBe("true");
  expect(mocks.pushError).toHaveBeenCalledOnce();
});
it("filters and creates a definition then assigns it without a Save action", async () => {
  await render(); await search(" RESEARCH ");
  expect(chip("Research")).toBeTruthy(); expect(chip("Work")).toBeUndefined();
  expect(container.textContent).not.toContain("Create “");
  await search("New");
  await act(async () => chip("Create “New”").click());
  expect(mocks.createTag).toHaveBeenCalledWith("New");
  expect(mocks.setEntryTag).toHaveBeenCalledWith("article", "new", true);
  expect(chip("New").getAttribute("aria-pressed")).toBe("true");
});
it("shows selection read-only and receives changes from another Inspector", async () => {
  mocks.readOnly = true; await render();
  expect(chip("Research").disabled).toBe(true);
  await search("New"); expect(chip("Create “New”").disabled).toBe(true);
  await search("");
  await act(async () => {
    changeListener({ entryId: "another", tagId: "work", assigned: true });
  });
  expect(chip("Work").getAttribute("aria-pressed")).toBe("false");
  await act(async () => {
    changeListener({ entryId: "article", tagId: "work", assigned: true });
    createdListener({ id: "third", name: "Third" });
  });
  expect(chip("Work").getAttribute("aria-pressed")).toBe("true");
  expect(chip("Third")).toBeTruthy();
  expect(mocks.setEntryTag).not.toHaveBeenCalled();
});
it("ignores late results after navigating to another Entry and cleans up listeners", async () => {
  let finish!: (ids: string[]) => void;
  mocks.readEntryTags.mockImplementationOnce(() => new Promise((resolve) => { finish = resolve; }));
  await render("old"); await render("new");
  await act(async () => finish(["work"]));
  expect(chip("Research").getAttribute("aria-pressed")).toBe("true");
  expect(chip("Work").getAttribute("aria-pressed")).toBe("false");
  expect(cleanup).toHaveBeenCalledTimes(2);
});
it("preserves a committed event that arrives during initial loading", async () => {
  let finish!: (ids: string[]) => void;
  mocks.readEntryTags.mockImplementationOnce(() => new Promise((resolve) => { finish = resolve; }));
  await render();
  await act(async () => {
    changeListener({ entryId: "article", tagId: "work", assigned: true });
    finish(["research"]);
  });
  expect(chip("Work").getAttribute("aria-pressed")).toBe("true");
});

it("matches registry lowercase semantics without equating distinct spellings", async () => {
  mocks.listTags.mockResolvedValue([
    { id: "german", name: "straße" }, { id: "greek", name: "ΟΣ" },
  ]);
  mocks.readEntryTags.mockResolvedValue([]);
  await render();
  await search("strasse");
  expect(chip("Create “strasse”")).toBeTruthy();
  await search("οσ");
  expect(chip("ΟΣ")).toBeTruthy();
  expect(container.textContent).not.toContain("Create “");
});
