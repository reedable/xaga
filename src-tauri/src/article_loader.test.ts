// @vitest-environment node

import { readFileSync } from "node:fs";
import { createContext, runInContext } from "node:vm";
import { afterEach, describe, expect, it, vi } from "vitest";

const script = readFileSync("src-tauri/src/article_loader.js", "utf8");
function loader(enabled = true) {
  const invoke = vi.fn().mockResolvedValue(undefined);
  const listeners: Record<string, () => void> = {};
  let mutate = () => {};
  const context = createContext({
    window: {
      __TAURI_INTERNALS__: { invoke },
      __LOAD_REMOTE_IMAGE__: enabled,
      addEventListener: (name: string, fn: () => void) => {
        listeners[name] = fn;
      },
    },
    document: {
      contentType: "text/html",
      documentElement: { outerHTML: "<html>Available article</html>" },
      images: [],
    },
    MutationObserver: class {
      constructor(fn: () => void) {
        mutate = fn;
      }
      observe() {}
      disconnect() {}
    },
    setTimeout,
    clearTimeout,
  });
  runInContext(script, context);
  return { context, invoke, listeners, mutate: () => mutate() };
}

afterEach(() => vi.useRealTimers());
describe("loader diagnostics", () => {
  it("does not report a timeout after a successful image fetch", async () => {
    vi.useFakeTimers();
    const { context, invoke } = loader();
    await runInContext(
      "__xagaWithTimeout(Promise.resolve('ok'), 100, null)",
      context
    );
    await vi.advanceTimersByTimeAsync(200);
    expect(
      invoke.mock.calls.some((call) => call[1].event === "image_timeout")
    ).toBe(false);
  });
  it("records a timeout when an image never returns", async () => {
    vi.useFakeTimers();
    const { context, invoke } = loader();
    const result = runInContext(
      "__xagaWithTimeout(new Promise(function() {}), 100, null).then(__xagaReportImageCounts)",
      context
    );
    await vi.advanceTimersByTimeAsync(100);
    await result;
    expect(invoke).toHaveBeenCalledWith("add_loader_diagnostic_event", {
      event: "image_timeout",
      count: 1,
    });
  });
  it("a failed diagnostic callback does not recursively report itself", async () => {
    const { context, invoke } = loader();
    invoke.mockClear().mockRejectedValue(new Error("transport unavailable"));
    await runInContext("__xagaDiagnostic('script_started')", context);
    expect(invoke).toHaveBeenCalledTimes(1);
  });
});

describe("favicon selection", () => {
  it("prefers 32x32 over 16x16 regardless of declaration order", () => {
    const { context } = loader();
    const link = (href: string, sizes: string) => ({
      getAttribute: (name: string) => ({ href, sizes })[name as "href" | "sizes"],
    });
    context.URL = URL;
    context.document.baseURI = "https://example.org/article";
    context.document.querySelectorAll = () => [
      link("/favicon-16.png", "16x16"),
      link("/favicon-32.png", "32x32"),
    ];

    expect(runInContext("__xagaFaviconUrl()", context)).toBe(
      "https://example.org/favicon-32.png"
    );
  });
});

describe("deadline snapshot", () => {
  it("keeps normal settled-page capture working", async () => {
    vi.useFakeTimers();
    const { context, invoke, listeners } = loader();
    runInContext("__xagaFaviconUrl = function() { return null; }", context);
    listeners.load();
    await vi.advanceTimersByTimeAsync(300);
    expect(
      invoke.mock.calls.filter(([name]) => name === "receive_html")
    ).toHaveLength(1);
  });
  it("does not send again when an in-flight image fetch finishes after timeout", async () => {
    vi.useFakeTimers();
    const { context, invoke, listeners } = loader();
    runInContext(
      "var finishImages; __xagaFetchImages = function() { return new Promise(function(resolve) { finishImages = resolve; }); }",
      context
    );
    listeners.load();
    await vi.advanceTimersByTimeAsync(300);
    runInContext("window.__xagaCaptureOnTimeout(); finishImages()", context);
    await vi.advanceTimersByTimeAsync(0);
    expect(
      invoke.mock.calls.filter(([name]) => name === "receive_html")
    ).toHaveLength(1);
  });

  it("delivers available HTML when continuous mutations prevent settling", async () => {
    vi.useFakeTimers();
    const { context, invoke, listeners, mutate } = loader();
    listeners.load();
    for (let i = 0; i < 50; i++) {
      await vi.advanceTimersByTimeAsync(200);
      mutate();
    }
    expect(
      invoke.mock.calls.filter(([name]) => name === "receive_html")
    ).toHaveLength(0);
    runInContext("window.__xagaCaptureOnTimeout()", context);
    await vi.advanceTimersByTimeAsync(1000);
    const captures = invoke.mock.calls.filter(
      ([name]) => name === "receive_html"
    );
    expect(captures).toHaveLength(1);
    expect(captures[0][1]).toMatchObject({
      html: "<html>Available article</html>",
      favicon: null,
    });
    expect(
      invoke.mock.calls.some(([, args]) => args.event === "images_started")
    ).toBe(false);
  });
  it("can snapshot before load and does not deliver twice", () => {
    const { context, invoke } = loader();
    runInContext(
      "window.__xagaCaptureOnTimeout(); window.__xagaCaptureOnTimeout()",
      context
    );
    expect(
      invoke.mock.calls.filter(([name]) => name === "receive_html")
    ).toHaveLength(1);
  });
});


describe("visible loader close", () => {
  it("captures the current page again after initial capture completes", async () => {
    vi.useFakeTimers();
    const { context, invoke, listeners } = loader();
    runInContext("__xagaFaviconUrl = function() { return null; }", context);
    listeners.load();
    await vi.advanceTimersByTimeAsync(300);
    expect(invoke.mock.calls.filter(([name]) => name === "receive_html")).toHaveLength(1);
    runInContext('document.documentElement.outerHTML = "<html>Updated article</html>"; window.__captureAndClose()', context);
    await vi.advanceTimersByTimeAsync(0);
    expect(invoke).toHaveBeenCalledWith("capture_and_close_loader", {
      html: "<html>Updated article</html>", mimeType: "text/html", favicon: null, faviconData: null,
    });
    expect(invoke.mock.calls.filter(([name]) => name === "capture_and_close_loader")).toHaveLength(1);
  });
});

describe("remote image preference", () => {
  it.each([false, true])("preserves local images with capture %s", async (enabled) => {
    const { context } = loader(enabled);
    const fetch = vi.fn();
    context.fetch = fetch;
    runInContext("document.images = [{ src: 'data:image/png;base64,YQ==' }]", context);
    await runInContext("__xagaFetchImages()", context);
    expect(fetch).not.toHaveBeenCalled();
    expect(runInContext("document.images[0].src", context)).toBe("data:image/png;base64,YQ==");
  });
  it("keeps remote references when opted-in capture fails", async () => {
    const { context } = loader(true);
    const fetch = vi.fn().mockRejectedValue(new Error("unavailable"));
    context.fetch = fetch;
    runInContext("document.images = [{ src: 'https://example.org/image.png' }]", context);
    await runInContext("__xagaFetchImages()", context);
    expect(fetch).toHaveBeenCalledTimes(1);
    expect(runInContext("document.images[0].src", context)).toBe("https://example.org/image.png");
  });
  it("does not fetch article images when closing a visible loader with capture off", async () => {
    const { context, invoke } = loader(false);
    const fetch = vi.fn();
    context.fetch = fetch;
    runInContext("document.images = [{ src: 'https://example.org/image.png' }]; __xagaFaviconUrl = function() { return null; }; window.__captureAndClose()", context);
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(fetch).not.toHaveBeenCalled();
    expect(invoke.mock.calls.some(([name]) => name === "capture_and_close_loader")).toBe(true);
  });
  it("does not make additional capture fetches when disabled", async () => {
    const { context } = loader(false);
    const fetch = vi.fn();
    context.fetch = fetch;
    runInContext(" document.images = [{ src: 'https://example.org/image.png' }]", context);
    await runInContext("__xagaFetchImages()", context);
    expect(fetch).not.toHaveBeenCalled();
  });
});

describe("shared image acquisition", () => {
  it.each([false, true])("sends original SVG bytes for image purpose %s", async (imageOnly) => {
    const { context, invoke } = loader();
    const svg = '<svg xmlns="http://www.w3.org/2000/svg"><rect width="16" height="16"/></svg>';
    context.Blob = Blob;
    context.FileReader = class {
      result = "";
      onload = () => {};
      async readAsDataURL(blob: Blob) {
        this.result = `data:${blob.type};base64,${Buffer.from(await blob.arrayBuffer()).toString("base64")}`;
        this.onload();
      }
    };
    const fetch = vi.fn().mockResolvedValue(new Response(svg, { headers: { "content-type": "image/svg+xml" } }));
    context.fetch = fetch;
    context.imageOnly = imageOnly;
    runInContext("document.contentType='image/svg+xml'; window.location={href:'https://example.org/image.svg'}; window.__LOADER_ID__='test'; window.__LOADER_IMAGE_ONLY__=imageOnly", context);
    await runInContext(imageOnly ? "__xagaCaptureImage()" : "__xagaCaptureFavicon()", context);
    expect(fetch).toHaveBeenCalledWith("https://example.org/image.svg", { credentials: "omit", cache: "force-cache" });
    expect(invoke).toHaveBeenCalledWith(imageOnly ? "receive_image" : "receive_favicon", {
      id: "test", dataUri: `data:image/svg+xml;base64,${Buffer.from(svg).toString("base64")}`, reason: null,
    });
    await runInContext("__xagaCaptureImage()", context);
    expect(fetch).toHaveBeenCalledTimes(1);
  });
});


describe("image capture decisions", () => {
  it.each([
    [false, [], "images_skipped_disabled"],
    [true, [], "images_skipped_no_candidates"],
    [true, [{ src: "data:image/png;base64,YQ==" }], "images_skipped_no_candidates"],
    [true, [{ src: "https://example.org/image.png" }], "images_attempted"],
  ] as const)("reports enabled=%s candidates=%j as %s", async (enabled, images, event) => {
    const { context, invoke } = loader(enabled);
    context.document.images = images;
    context.fetch = vi.fn().mockRejectedValue(new Error("fetch failed"));
    await runInContext("__xagaFetchImages()", context);
    expect(invoke).toHaveBeenCalledWith("add_loader_diagnostic_event", { event, count: 1 });
    expect(context.fetch).toHaveBeenCalledTimes(event === "images_attempted" ? 1 : 0);
    if (event === "images_attempted") {
      expect(invoke).toHaveBeenCalledWith("add_loader_diagnostic_event", { event: "image_unreadable", count: 1 });
    }
  });
  it.each([false, true])("reports capture skipped at deadline with enabled=%s", (enabled) => {
    const { context, invoke } = loader(enabled);
    runInContext("window.__xagaCaptureOnTimeout()", context);
    expect(invoke).toHaveBeenCalledWith("add_loader_diagnostic_event", {
      event: enabled ? "images_skipped_deadline" : "images_skipped_disabled", count: 1,
    });
  });
  it("distinguishes a deadline after fetches began", async () => {
    const { context, invoke } = loader();
    context.document.images = [{ src: "https://example.org/image.png" }];
    context.fetch = vi.fn().mockRejectedValue(new Error("fetch failed"));
    await runInContext("__xagaFetchImages()", context);
    runInContext("window.__xagaCaptureOnTimeout()", context);
    expect(invoke).toHaveBeenCalledWith("add_loader_diagnostic_event", { event: "images_interrupted_deadline", count: 1 });
  });
});
