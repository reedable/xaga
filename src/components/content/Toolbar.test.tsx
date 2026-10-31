import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { Toolbar } from "./Toolbar";

const mocks = vi.hoisted(() => ({
  readOnly: false,
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
    onBookmarkChanged: () => () => {},
    onUnreadChanged: () => () => {},
    onFaviconChanged: () => () => {},
  },
  router: {
    state: { entryId: "article", url: "https://example.org" },
    selectUrl: vi.fn(),
    invokeSearch: vi.fn(),
    clearSearch: vi.fn(),
  },
  bookmarks: { addBookmark: vi.fn(), removeBookmark: vi.fn() },
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
vi.mock("./TopicMenuButton", () => ({ TopicMenuButton: () => null }));
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
  isBookmarked: false,
  isPinned: false,
};
beforeEach(() => {
  vi.clearAllMocks();
  mocks.readOnly = false;
  mocks.pins = [];
  mocks.setPin.mockImplementation(async (target, pinned) => { mocks.pins = pinned ? [target] : []; });
  mocks.platform.readArticle.mockResolvedValue(article);
  mocks.platform.readEntryImages.mockResolvedValue([null]);
  container = document.createElement("div");
  root = createRoot(container);
});
afterEach(async () => {
  await act(async () => root.unmount());
});
const button = (label: string) =>
  container.querySelector<HTMLButtonElement>(`button[aria-label="${label}"]`)!;
async function render() {
  await act(async () => root.render(<Toolbar reload={null} />));
}

it("pins and unpins through the platform using committed state", async () => {
  await render();
  await act(async () => button("Add to Shortcuts").click());
  expect(mocks.setPin).toHaveBeenLastCalledWith(
    { type: "entry", id: "article" },
    true
  );
  expect(button("Remove from Shortcuts").getAttribute("aria-pressed")).toBe(
    "true"
  );
  await act(async () => button("Remove from Shortcuts").click());
  expect(mocks.setPin).toHaveBeenLastCalledWith(
    { type: "entry", id: "article" },
    false
  );
  expect(button("Add to Shortcuts").getAttribute("aria-pressed")).toBe("false");
});
it("disables pinning in read-only mode", async () => {
  mocks.readOnly = true;
  await render();
  expect(button("Add to Shortcuts").disabled).toBe(true);
  button("Add to Shortcuts").click();
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
  await act(async () => button("Add to Shortcuts").click());
  expect(mocks.sink.pushError).toHaveBeenCalledWith(error);
  expect(button("Add to Shortcuts").disabled).toBe(false);
  expect(button("Remove from Shortcuts")).toBeNull();
});

vi.mock("../../providers/PinsContext", () => ({ usePins: () => ({ pins: mocks.pins, setPin: mocks.setPin }) }));

it("keeps Refresh visible beside Bookmark and focusable while reporting reload progress", async () => {
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
