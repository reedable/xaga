import { act, useEffect } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { useContentLoader } from "./useContentLoader";

const mocks = vi.hoisted(() => ({
  platform: { readStoredPdf: vi.fn(), openStoredPdf: vi.fn() },
  articles: { findArticleByUrl: vi.fn(), loadArticleByUrl: vi.fn() },
  router: { replaceEntry: vi.fn(), replaceMimeType: vi.fn() },
  sink: { pushError: vi.fn() },
  settings: { inlinePdfReader: true },
}));
vi.mock("../platform/Platform", () => ({ usePlatform: () => mocks.platform }));
vi.mock("./useArticles", () => ({
  useArticles: () => mocks.articles,
  base64ToBytes: () => new Uint8Array([37, 80, 68, 70, 45]),
}));
vi.mock("./useRouter", () => ({ default: () => mocks.router }));
vi.mock("./useErrorSink", () => ({ useErrorSink: () => mocks.sink }));
vi.mock("./useSettings", () => ({ useSettings: () => mocks.settings }));
vi.mock("../utils/ArticleLoadAttempt", () => ({
  default: {
    create: async () => ({
      runStep: async (_: string, fn: () => unknown) => fn(),
      complete: async () => {},
    }),
  },
}));
vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
let root: ReturnType<typeof createRoot>;
let hook: ReturnType<typeof useContentLoader>;
let controller: { current: AbortController };
function Host() {
  const value = useContentLoader(controller);
  useEffect(() => {
    hook = value;
  }, [value]);
  return null;
}
beforeEach(async () => {
  vi.clearAllMocks();
  mocks.settings.inlinePdfReader = true;
  mocks.platform.readStoredPdf.mockResolvedValue(null);
  mocks.platform.openStoredPdf.mockResolvedValue(undefined);
  mocks.articles.findArticleByUrl.mockResolvedValue(null);
  mocks.articles.loadArticleByUrl.mockResolvedValue({
    mimeType: "application/pdf",
    pdfBytes: new Uint8Array([37, 80, 68, 70, 45]),
    article: null,
  });
  URL.createObjectURL = vi.fn(() => "blob:pdf-test");
  URL.revokeObjectURL = vi.fn();
  controller = { current: new AbortController() };
  root = createRoot(document.createElement("div"));
  await act(() => root.render(<Host />));
});
afterEach(async () => {
  await act(() => root.unmount());
});
it("reuses the stored PDF without article lookup or network loading", async () => {
  mocks.platform.readStoredPdf.mockResolvedValue({ pdfBase64: "JVBERi0=" });
  await act(() => hook.resolve("https://example.org/paper"));
  expect(mocks.articles.findArticleByUrl).not.toHaveBeenCalled();
  expect(mocks.articles.loadArticleByUrl).not.toHaveBeenCalled();
  expect(hook.pdfBlobUrl).toBe("blob:pdf-test");
  expect(mocks.router.replaceMimeType).toHaveBeenCalledWith("application/pdf");
});
it("opens the persisted PDF in the default app after ingestion", async () => {
  mocks.settings.inlinePdfReader = false;
  // Settings are read during render.
  await act(() => root.render(<Host />));
  await act(() => hook.resolve("https://example.org/paper"));
  expect(mocks.platform.openStoredPdf).toHaveBeenCalledWith(
    "https://example.org/paper"
  );
  expect(URL.createObjectURL).not.toHaveBeenCalled();
});
it("refresh bypasses local PDF and article lookups", async () => {
  await act(() => hook.resolve("https://example.org/paper", true));
  expect(mocks.platform.readStoredPdf).not.toHaveBeenCalled();
  expect(mocks.articles.findArticleByUrl).not.toHaveBeenCalled();
  expect(mocks.articles.loadArticleByUrl).toHaveBeenCalledOnce();
});
it("unsupported downloads are not displayed or externally opened", async () => {
  mocks.articles.loadArticleByUrl.mockResolvedValue({
    blockedDownload: true,
    article: null,
  });
  await act(() => hook.resolve("https://example.org/archive"));
  expect(hook.blockedUrl).toBe("https://example.org/archive");
  expect(URL.createObjectURL).not.toHaveBeenCalled();
  expect(mocks.platform.openStoredPdf).not.toHaveBeenCalled();
});
it("cancellation after a cached read does not open or render its result", async () => {
  mocks.platform.readStoredPdf.mockImplementation(async () => {
    controller.current.abort();
    return { pdfBase64: "JVBERi0=" };
  });
  await act(() => hook.resolve("https://example.org/paper"));
  expect(URL.createObjectURL).not.toHaveBeenCalled();
  expect(mocks.platform.openStoredPdf).not.toHaveBeenCalled();
});

it("ingests a saved entry without article content through URL resolution", async () => {
  mocks.articles.findArticleByUrl.mockResolvedValue({
    id: "saved",
    content: "",
  });
  await act(() => hook.resolve("https://example.org/paper"));
  expect(mocks.articles.loadArticleByUrl).toHaveBeenCalled();
  expect(mocks.router.replaceMimeType).toHaveBeenCalledWith("application/pdf");
});
it("uses saved Markdown without network loading", async () => {
  const article = { id: "saved", content: "saved body" };
  mocks.articles.findArticleByUrl.mockResolvedValue(article);
  await act(() => hook.resolve("https://example.org/article"));
  expect(mocks.articles.loadArticleByUrl).not.toHaveBeenCalled();
  expect(mocks.router.replaceEntry).toHaveBeenCalledWith(article);
});

it.each(["failure", "unsupported"])(
  "retains the displayed PDF after refresh %s",
  async (outcome) => {
    mocks.platform.readStoredPdf.mockResolvedValue({ pdfBase64: "JVBERi0=" });
    await act(() => hook.resolve("https://example.org/paper"));
    const blob = hook.pdfBlobUrl;
    if (outcome === "failure")
      mocks.articles.loadArticleByUrl.mockRejectedValue(new Error("failed"));
    else
      mocks.articles.loadArticleByUrl.mockResolvedValue({
        blockedDownload: true,
        article: null,
      });
    let result: unknown;
    await act(async () => {
      result = await hook.resolve("https://example.org/paper", true);
    });
    expect(result).toBe(false);
    expect(hook.pdfBlobUrl).toBe(blob);
    expect(hook.blockedUrl).toBeNull();
    expect(URL.revokeObjectURL).not.toHaveBeenCalled();
  }
);
