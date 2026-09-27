// @vitest-environment node

import { readFileSync } from "node:fs";
import { createContext, runInContext } from "node:vm";
import { afterEach, describe, expect, it, vi } from "vitest";

const script = readFileSync("src-tauri/src/article_loader.js", "utf8");
function loader() {
  const invoke = vi.fn().mockResolvedValue(undefined);
  const context = createContext({
    window: { __TAURI_INTERNALS__: { invoke }, addEventListener: () => {} },
    document: { contentType: "text/html" },
    setTimeout,
    clearTimeout,
  });
  runInContext(script, context);
  return { context, invoke };
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
