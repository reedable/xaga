import { act, useEffect } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import ArticleReader from "./ArticleReader";

// This test follows summary data through React; stylesheet bundling is checked
// by the application build and rendering by the native WebKit probe.
vi.mock("../../../../lliwio-css/src/index.css?inline", () => ({ default: "" }));

const mocks = vi.hoisted(() => ({
  appearance: {
    htmlReaderView: "summary", themeMode: "light", htmlReaderFontFamily: "system",
    htmlReaderFontSize: 16, htmlReaderLineHeight: 1.75,
  },
  article: { id: "entry", title: "Comic", description: '<img src="https://example.org/comic.png" alt="Comic">', content: "Full body" } as { id: string; title: string; description?: string; content: string },
  html: "",
  sink: { pushError: vi.fn() },
  platform: {},
  noop: () => {},
  events: { setFindEnabled: vi.fn(), onUnreadChanged: () => () => {}, onBookmarkChanged: () => () => {}, onLoaderClosed: vi.fn((_callback: (event: typeof closedArticle) => Promise<void>) => () => {}) },
  router: { state: { url: "https://example.org/comic", entryId: "entry" }, selectUrl: () => {}, replaceUrl: () => {} },
  find: vi.fn(),
  extract: vi.fn(),
  save: vi.fn(),
  read: vi.fn(),
}));
vi.mock("../../hooks/useAppearance", () => ({ useAppearance: () => mocks.appearance }));
vi.mock("../../hooks/useRouter", () => ({ default: () => mocks.router }));
vi.mock("../../platform/Platform", () => ({ usePlatform: () => mocks.platform }));
vi.mock("../../hooks/useErrorSink", () => ({ useErrorSink: () => mocks.sink }));
vi.mock("../../hooks/useEvents", () => ({ useEvents: () => mocks.events }));
vi.mock("../../hooks/useArticles", async (importOriginal) => ({ ...await importOriginal<typeof import("../../hooks/useArticles")>(), useArticles: () => ({ findArticleByUrl: mocks.find, extractArticleFromHtml: mocks.extract, saveExtractedArticle: mocks.save, readEntry: mocks.read }) }));
vi.mock("../../hooks/useAnnotations", () => ({ useAnnotations: () => ({ annotations: [], setAnnotationsFromArticle: mocks.noop }) }));
vi.mock("../../utils/ArticleLoadAttempt", () => ({ default: { create: async () => ({
  runStep: async (_: string, fn: () => unknown) => fn(), complete: async () => {},
}) } }));
vi.mock("./RelatedItemList", () => ({ RelatedItemList: () => null }));
vi.mock("./HtmlReader", () => ({ HtmlReader: ({ html }: { html: string }) => {
  useEffect(() => { mocks.html = html; }, [html]);
  return null;
} }));
vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
let root: ReturnType<typeof createRoot>;
beforeEach(() => {
  vi.clearAllMocks();
  mocks.html = "";
  mocks.appearance.htmlReaderView = "summary";
  mocks.article.description = '<img src="https://example.org/comic.png" alt="Comic">';
  mocks.find.mockImplementation(async () => mocks.article);
  root = createRoot(document.createElement("div"));
});
afterEach(async () => { await act(() => root.unmount()); });
it("passes the existing image summary through to the iframe and preserves it across view switches", async () => {
  await act(() => root.render(<ArticleReader />));
  expect(mocks.html).toContain('<article><img src="https://example.org/comic.png" alt="Comic"></article>');
  mocks.appearance.htmlReaderView = "reader";
  await act(() => root.render(<ArticleReader />));
  expect(mocks.html).toContain("<article><p>Full body</p></article>");
  mocks.appearance.htmlReaderView = "summary";
  await act(() => root.render(<ArticleReader />));
  expect(mocks.html).toContain('<article><img src="https://example.org/comic.png" alt="Comic"></article>');
  expect(mocks.sink.pushError).not.toHaveBeenCalled();
});
it("reproduces the empty iframe document when URL ingestion has removed the saved summary", async () => {
  mocks.article.description = undefined;
  await act(() => root.render(<ArticleReader />));
  expect(mocks.find).toHaveBeenCalled();
  expect(mocks.html).toBe("");
  expect(mocks.sink.pushError).not.toHaveBeenCalled();
});

const closedArticle = {
  loaderUrl: "https://example.org/comic", samePage: true,
  html: "<html>Updated article</html>", mimeType: "text/html",
  favicon: null, faviconData: null,
};
async function closeLoader() {
  const callback = mocks.events.onLoaderClosed.mock.calls.at(-1)![0];
  await act(() => callback(closedArticle));
}
function clickDialogButton(label: string) {
  const button = [...document.querySelectorAll("[role=dialog] button")]
    .find((button) => button.textContent === label) as HTMLButtonElement;
  expect(button).toBeDefined();
  button.click();
}
it("offers an update on loader close and saves only after Update", async () => {
  const extracted = { title: "Updated", markdown: "Updated body", favicon: null };
  mocks.extract.mockResolvedValue(extracted);
  mocks.read.mockResolvedValue(mocks.article);
  mocks.save.mockResolvedValue({ ...mocks.article, content: "Updated body" });
  await act(() => root.render(<ArticleReader />));
  await closeLoader();
  expect(document.querySelector("[role=dialog]")?.textContent).toContain("Update article?");
  expect(mocks.save).not.toHaveBeenCalled();
  await act(() => clickDialogButton("Update"));
  expect(mocks.save).toHaveBeenCalledExactlyOnceWith(mocks.article, extracted);
});
it("keeps the saved article when Keep old is selected", async () => {
  mocks.extract.mockResolvedValue({ title: "Updated", markdown: "Updated body", favicon: null });
  await act(() => root.render(<ArticleReader />));
  await closeLoader();
  await act(() => clickDialogButton("Keep old"));
  expect(mocks.save).not.toHaveBeenCalled();
});
