import { act, useEffect } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { useArticles } from "./useArticles";

const mocks = vi.hoisted(() => ({
  platform: {
    loadArticle: vi.fn(), findArticle: vi.fn(), createArticle: vi.fn(),
    readArticle: vi.fn(), getArticleDiagnostic: vi.fn(),
  },
  sink: { pushError: vi.fn() },
  processEntry: vi.fn(),
}));
vi.mock("../platform/Platform", () => ({ usePlatform: () => mocks.platform }));
vi.mock("./useErrorSink", () => ({ useErrorSink: () => mocks.sink }));
vi.mock("../text/entryWorker", () => ({ processEntry: mocks.processEntry }));
vi.mock("../utils/ArticleLoadAttempt", () => ({
  default: { create: async () => ({
    runStep: async (_: string, fn: () => unknown) => fn(),
    complete: async () => {},
  }) },
}));
vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
let controller: { current: AbortController };
let root: ReturnType<typeof createRoot>;
let hook: ReturnType<typeof useArticles>;
beforeEach(async () => {
  vi.resetAllMocks();
  mocks.platform.loadArticle.mockResolvedValue({ html: "<p>Body</p>", mimeType: "text/html" });
  mocks.processEntry.mockResolvedValue({ title: "Title", markdown: "Body", favicon: null, imageData: {} });
  mocks.platform.createArticle.mockImplementation(async (_signal, article) => ({ ...article, id: "entry" }));
  mocks.platform.readArticle.mockResolvedValue({ id: "entry" });
  controller = { current: new AbortController() };
  function Host() {
    const articles = useArticles(controller);
    useEffect(() => { hook = articles; }, [articles]);
    return null;
  }
  root = createRoot(document.createElement("div"));
  await act(() => root.render(<Host />));
});
afterEach(async () => { await act(() => root.unmount()); });

it("downloads without reading and resubmitting saved metadata", async () => {
  mocks.platform.findArticle.mockResolvedValue({
    id: "entry", description: "Stale summary", publishedAt: "old", updatedAt: "old",
  });
  await act(() => hook.loadArticleByUrl("https://example.org/article"));
  expect(mocks.platform.findArticle).not.toHaveBeenCalled();
  expect(mocks.platform.createArticle).toHaveBeenCalledOnce();
  const submitted = mocks.platform.createArticle.mock.calls[0][1];
  expect(submitted).toMatchObject({ title: "Title", content: "Body", url: "https://example.org/article" });
  for (const field of ["description", "publishedAt", "updatedAt"]) {
    expect(submitted).not.toHaveProperty(field);
  }
  expect(mocks.sink.pushError).not.toHaveBeenCalled();
});

it("returns the saved article from readback after downloading", async () => {
  const saved = { id: "entry", content: "Body", description: "Current saved summary" };
  mocks.platform.readArticle.mockResolvedValue(saved);
  let result;
  await act(async () => { result = await hook.loadArticleByUrl("https://example.org/new"); });
  expect(result).toMatchObject({ article: saved });
  expect(mocks.sink.pushError).not.toHaveBeenCalled();
});


it("does not emit recovered warnings when navigation cancels a pending diagnostic lookup", async () => {
  let finish!: (value: unknown) => void;
  mocks.platform.getArticleDiagnostic.mockImplementation(() => new Promise(resolve => { finish = resolve; }));
  const parent = {
    id: "attempt",
    runStep: async (_step: string, fn: () => unknown) => fn(),
    complete: async () => {},
  } as unknown as NonNullable<Parameters<typeof hook.loadArticleByUrl>[2]>;
  let result: unknown;
  let pending!: Promise<unknown>;
  await act(() => { pending = hook.loadArticleByUrl("https://example.org/article", false, parent).catch(error => { result = error; }); });
  controller.current.abort();
  await act(async () => {
    finish({ events: [{ step: "favicon", outcome: "warning" }] });
    await pending;
  });
  expect(result).toHaveProperty("name", "AbortError");
  expect(mocks.sink.pushError).not.toHaveBeenCalled();
  expect(mocks.processEntry).not.toHaveBeenCalled();
});
