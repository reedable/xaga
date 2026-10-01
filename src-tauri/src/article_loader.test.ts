// @vitest-environment node

import { readFileSync } from "node:fs";
import { createContext, runInContext } from "node:vm";
import { afterEach, describe, expect, it, vi } from "vitest";

const script = readFileSync("src-tauri/src/article_loader.js", "utf8");
function loader() {
  const invoke = vi.fn().mockResolvedValue(undefined);
  const listeners: Record<string, () => void> = {};
  let mutate = () => {};
  const context = createContext({
    window: {
      __TAURI_INTERNALS__: { invoke },
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
