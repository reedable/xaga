import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { Toolbar } from "./Toolbar";

const mocks = vi.hoisted(() => ({
  readOnly: false,
  bookmarkChanged: undefined as
    | undefined
    | ((entry: import("../../types").Entry) => void),
  pins: [] as Array<{ type: "entry"; id: string }>,
  setPin: vi.fn(),
  read: vi.fn(),
  pin: vi.fn(),
  sink: { pushError: vi.fn() },
  platform: {
    openUrl: vi.fn(),
    readArticle: vi.fn(),
    readEntryImages: vi.fn(),
    setEntryPinned: vi.fn(),
    onEntryPinChanged: vi.fn().mockResolvedValue(() => {}),
  },
  events: {
    onBookmarkChanged: (
      handler: (entry: import("../../types").Entry) => void
    ) => {
      mocks.bookmarkChanged = handler;
      return () => {};
    },
    onUnreadChanged: () => () => {},
    onFaviconChanged: () => () => {},
  },
  router: {
    state: { entryId: "article", url: "https://example.org" },
    selectUrl: vi.fn(),
    invokeSearch: vi.fn(),
    clearSearch: vi.fn(),
  },
  bookmarks: {
    loading: false,
    error: null,
    addBookmark: vi.fn(),
    removeBookmark: vi.fn(),
  },
  entries: { markEntryRead: vi.fn(), markEntryUnread: vi.fn() },
  articles: { loadArticleByUrl: vi.fn(), loading: false },
}));
vi.mock("../../platform/Platform", () => ({
  usePlatform: () => mocks.platform,
}));
vi.mock("../../hooks/useRouter", () => ({ default: () => mocks.router }));
vi.mock("../../hooks/useErrorSink", () => ({ useErrorSink: () => mocks.sink }));
vi.mock("../../hooks/useLibraryReadOnly", () => ({
  useLibraryReadOnly: () => mocks.readOnly,
}));
vi.mock("../../hooks/useEmbeddingStartupState", () => ({
  useEmbeddingStartupState: () => "available",
}));
vi.mock("../../hooks/useBookmarks", () => ({
  useBookmarks: () => mocks.bookmarks,
}));
vi.mock("../../hooks/useEntries", () => ({ default: () => mocks.entries }));
vi.mock("../../hooks/useArticles", () => ({
  useArticles: () => mocks.articles,
}));
vi.mock("../../hooks/useEvents", async (original) => ({
  ...(await original<typeof import("../../hooks/useEvents")>()),
  useEvents: () => mocks.events,
}));
vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
let container: HTMLDivElement;
let root: ReturnType<typeof createRoot>;
const article = {
  __kind: "entry",
  id: "article",
  title: "Article",
  createdAt: "2026-10-29",
  url: "https://example.org",
  content: "body",
  annotations: [],
  isPinned: false,
};
beforeEach(() => {
  vi.clearAllMocks();
  mocks.readOnly = false;
  mocks.pins = [];
  mocks.setPin.mockImplementation(async (target, pinned) => {
    mocks.pins = pinned ? [target] : [];
  });
  mocks.platform.readArticle.mockResolvedValue(article);
  mocks.platform.readEntryImages.mockResolvedValue([null]);
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
});
afterEach(async () => {
  await act(async () => root.unmount());
  container.remove();
});
const button = (label: string) =>
  container.querySelector<HTMLButtonElement>(`button[aria-label="${label}"]`)!;
const menuItem = (label: string) =>
  Array.from(document.querySelectorAll<HTMLElement>('[role="menuitem"]')).find(
    (item) => item.textContent === label
  );
async function render() {
  await act(async () => root.render(<Toolbar reload={null} />));
}

it("pins and unpins through the platform using committed state", async () => {
  await render();
  await act(async () => button("Share").click());
  await act(async () => menuItem("Add to Shortcuts")!.click());
  expect(mocks.setPin).toHaveBeenLastCalledWith(
    { type: "entry", id: "article" },
    true
  );
  await act(async () => button("Share").click());
  expect(menuItem("Remove from Shortcuts")).not.toBeUndefined();
  await act(async () => menuItem("Remove from Shortcuts")!.click());
  expect(mocks.setPin).toHaveBeenLastCalledWith(
    { type: "entry", id: "article" },
    false
  );
  await act(async () => button("Share").click());
  expect(menuItem("Add to Shortcuts")).not.toBeUndefined();
});
it("disables pinning in read-only mode", async () => {
  mocks.readOnly = true;
  await render();
  await act(async () => button("Share").click());
  expect(menuItem("Add to Shortcuts")!.getAttribute("aria-disabled")).toBe("true");
  expect(mocks.setPin).not.toHaveBeenCalled();
});

it.each(["../favicons/0123456789ab", "../images/article/0123456789ab"])(
  "displays cached favicon %s through local IPC in read-only mode",
  async (reference) => {
    mocks.readOnly = true;
    mocks.platform.readArticle.mockResolvedValue({ ...article, favicon: reference });
    mocks.platform.readEntryImages.mockResolvedValue(["data:image/png;base64,local"]);
    await render();
    expect(mocks.platform.readEntryImages).toHaveBeenCalledWith(
      expect.any(AbortSignal), "article", [reference]
    );
    expect(container.querySelector("img")?.getAttribute("src")).toBe("data:image/png;base64,local");
    expect(mocks.articles.loadArticleByUrl).not.toHaveBeenCalled();
  }
);

it.each([
  "https://example.org/icon.png",
  "../favicons/0123456789ab",
  "../images/foreign/0123456789ab",
  "../images/article/../../private.png",
])("never renders an unavailable or edited favicon as a direct URL: %s", async (reference) => {
  mocks.platform.readArticle.mockResolvedValue({ ...article, favicon: reference });
  await render();
  expect(container.querySelector("img")).toBeNull();
  expect(mocks.articles.loadArticleByUrl).not.toHaveBeenCalled();
  if (reference.startsWith("https:")) expect(mocks.platform.readEntryImages).not.toHaveBeenCalled();
  expect(mocks.sink.pushError).toHaveBeenCalled();
});
it("reports a refused pin without displaying it as persisted", async () => {
  await render();
  const error = { code: "E_READ_ONLY", message: "Read-only library" };
  mocks.setPin.mockRejectedValueOnce(error);
  await act(async () => button("Share").click());
  await act(async () => menuItem("Add to Shortcuts")!.click());
  expect(mocks.sink.pushError).toHaveBeenCalledWith(error);
  await act(async () => button("Share").click());
  expect(menuItem("Add to Shortcuts")!.getAttribute("aria-disabled")).not.toBe("true");
  expect(menuItem("Remove from Shortcuts")).toBeUndefined();
});

vi.mock("../../providers/PinsContext", () => ({
  usePins: () => ({ pins: mocks.pins, setPin: mocks.setPin }),
}));

it("keeps Refresh visible before Share and focusable while reporting reload progress", async () => {
  const reload = vi.fn();
  await act(async () => root.render(<Toolbar reload={reload} reloading />));
  const refresh = button("Reload in progress...");
  expect(refresh.disabled).toBe(false);
  expect(refresh.title).toBe("Reload in progress...");
  expect(refresh.className).not.toContain("RevealOnHover");
  expect(refresh.parentElement?.nextElementSibling?.getAttribute("role")).toBe(
    "status"
  );
  expect(container.querySelector('[role="status"]')?.textContent).toBe(
    "Reload in progress..."
  );
  await act(() => refresh.click());
  expect(reload).not.toHaveBeenCalled();
  await act(async () => root.render(<Toolbar reload={reload} />));
  expect(container.querySelector('[role="status"]')?.textContent).toBe("");
  await act(() => button("Refresh").click());
  expect(reload).toHaveBeenCalledOnce();
});

const paletteButton = (label: string) =>
  document.querySelector<HTMLButtonElement>(
    `[role="dialog"][aria-label="Bookmark color"] button[aria-label="${label}"]`
  )!;
it("assigns, recolors, and removes a bookmark without losing article context", async () => {
  mocks.bookmarks.addBookmark.mockImplementation(async (entry, color) => ({
    ...entry,
    color,
  }));
  mocks.bookmarks.removeBookmark.mockImplementation(async (entry) => ({
    ...entry,
    color: undefined,
  }));
  await render();
  await act(async () => button("Add bookmark").click());
  expect(
    document.querySelector('[role="dialog"][aria-label="Bookmark color"]')
  ).not.toBeNull();
  expect(document.activeElement).toBe(paletteButton("red"));
  await act(async () => paletteButton("blue").click());
  expect(mocks.bookmarks.addBookmark).toHaveBeenLastCalledWith(
    expect.objectContaining({ id: "article" }),
    "blue"
  );
  expect(button("Change bookmark color")).not.toBeNull();
  await act(async () => button("Change bookmark color").click());
  expect(paletteButton("blue").getAttribute("aria-pressed")).toBe("true");
  await act(async () => paletteButton("violet").click());
  expect(mocks.bookmarks.addBookmark).toHaveBeenLastCalledWith(
    expect.objectContaining({ color: "blue" }),
    "violet"
  );
  await act(async () => button("Change bookmark color").click());
  await act(async () => paletteButton("Remove marking").click());
  const confirmation = Array.from(
    document.querySelectorAll<HTMLButtonElement>("button")
  ).find((item) => item.textContent === "Remove")!;
  await act(async () => confirmation.click());
  expect(mocks.bookmarks.removeBookmark).toHaveBeenCalledOnce();
  expect(button("Add bookmark")).not.toBeNull();
  expect(mocks.router.selectUrl).not.toHaveBeenCalled();
});
it("reflects external color-to-color bookmark events", async () => {
  await render();
  await act(async () =>
    mocks.bookmarkChanged?.({ ...article, __kind: "entry", color: "green" })
  );
  await act(async () => button("Change bookmark color").click());
  expect(paletteButton("green").getAttribute("aria-pressed")).toBe("true");
  await act(async () =>
    mocks.bookmarkChanged?.({ ...article, __kind: "entry", color: "orange" })
  );
  expect(paletteButton("orange").getAttribute("aria-pressed")).toBe("true");
});
it("disables article color changes in read-only mode", async () => {
  mocks.readOnly = true;
  await render();
  expect(button("Add bookmark").disabled).toBe(true);
  expect(mocks.bookmarks.addBookmark).not.toHaveBeenCalled();
});

it("dismisses the swatch popup with Escape and restores toolbar focus", async () => {
  await render();
  button("Add bookmark").focus();
  await act(async () => button("Add bookmark").click());
  await act(async () => {
    document.activeElement?.dispatchEvent(
      new KeyboardEvent("keydown", { key: "Escape", bubbles: true })
    );
    await new Promise((resolve) => setTimeout(resolve, 250));
  });
  expect(
    document.querySelector('[role="dialog"][aria-label="Bookmark color"]')
  ).toBeNull();
  expect(document.activeElement).toBe(button("Add bookmark"));
  expect(mocks.bookmarks.addBookmark).not.toHaveBeenCalled();
});


it("keeps Share visible and opens its ordered browser and QR actions", async () => {
  await render();
  const share = button("Share");
  expect(share.className).not.toContain("RevealOnHover");
  await act(async () => share.click());
  const items = Array.from(document.querySelectorAll<HTMLElement>('[role="menuitem"]'));
  expect(items.map((item) => item.textContent)).toEqual([
    "Open in default browser",
    "Show QR code",
    "Add to Shortcuts",
    "Mark as unread",
  ]);
  expect(items[1].nextElementSibling?.tagName).toBe("HR");
  await act(async () => items[0].click());
  expect(mocks.platform.openUrl).toHaveBeenCalledWith("https://example.org");
  await act(async () => share.click());
  await act(async () => document.querySelectorAll<HTMLElement>('[role="menuitem"]')[1].click());
  expect(document.querySelector('[class*="QrPopover"] svg')).not.toBeNull();
});


it("marks the article unread from Share and closes the menu", async () => {
  await render();
  await act(async () => button("Share").click());
  await act(async () => menuItem("Mark as unread")!.click());
  expect(mocks.entries.markEntryUnread).toHaveBeenCalledWith("article");
  expect(button("Share").getAttribute("aria-expanded")).toBe("false");
});
