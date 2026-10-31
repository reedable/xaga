import { act, useEffect } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useErrorSink } from "../hooks/useErrorSink";
import { PlatformContext, type Platform } from "../platform/Platform";
import type { ArticleDiagnostic } from "../types";
import ArticleLoadError from "../utils/ArticleLoadError";
import ErrorSinkProvider from "./ErrorSinkProvider";
import FaviconLoadError from "../utils/FaviconLoadError";

vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
const mounted: ReturnType<typeof createRoot>[] = [];
afterEach(async () => {
  for (const root of mounted.splice(0)) await act(() => root.unmount());
  document.body.innerHTML = "";
});

function Trigger() {
  const { pushError } = useErrorSink();
  return (
    <button
      onClick={() =>
        pushError(
          new ArticleLoadError("test-id", "fetch", {
            code: "app:error",
            message: "Something went wrong.",
            detail: "private exception",
          })
        )
      }
    >
      Fail article
    </button>
  );
}

describe("article error details", () => {
  it("opens the correct attempt and copies details without rendering technical text", async () => {
    const report: ArticleDiagnostic = {
      id: "test-id",
      mode: "load",
      startedAtMs: 0,
      outcome: "failed",
      droppedEvents: 0,
      events: [
        {
          elapsedMs: 100,
          source: "native",
          step: "capture_wait",
          outcome: "failed",
          code: "article:timeout",
          detail: "private technical detail",
        },
      ],
    };
    const read = vi.fn().mockResolvedValue(report);
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });
    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);
    mounted.push(root);
    await act(() =>
      root.render(
        <PlatformContext.Provider
          value={{ getArticleDiagnostic: read } as unknown as Platform}
        >
          <ErrorSinkProvider>
            <Trigger />
          </ErrorSinkProvider>
        </PlatformContext.Provider>
      )
    );
    const click = async (label: string) => {
      const button = [...document.querySelectorAll("button")].find(
        (b) => b.textContent === label
      );
      expect(button, label).toBeDefined();
      await act(async () => {
        button!.click();
      });
    };
    await click("Fail article");
    await click("Details");
    expect(read).toHaveBeenCalledWith("test-id");
    expect(document.body.textContent).toContain("article:timeout");
    expect(document.body.textContent).not.toContain("private technical detail");
    expect(document.body.textContent).not.toContain("private exception");
    await click("Copy diagnostics");
    expect(JSON.parse(writeText.mock.calls[0][0])).toEqual(report);
    expect(document.body.textContent).toContain("Diagnostics copied.");
  });
});

it.each([false, true])(
  "shows favicon warning without claiming article failure (remedy=%s)",
  async (retry) => {
    let push: (error: unknown) => void = () => {};
    function Capture() {
      const nextPush = useErrorSink().pushError;
      useEffect(() => {
        push = nextPush;
      }, [nextPush]);
      return null;
    }
    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);
    mounted.push(root);
    await act(() =>
      root.render(
        <ErrorSinkProvider>
          <Capture />
        </ErrorSinkProvider>
      )
    );
    await act(() =>
      push(new FaviconLoadError("https://example.org/article", retry))
    );
    const alert = document.querySelector('[role="alert"]');
    expect(alert?.textContent).toContain("The site icon couldn’t be loaded.");
    expect(alert?.className).toContain("MuiAlert-colorWarning");
    expect(alert?.className).toContain("MuiAlert-filled");
    expect(alert?.textContent).not.toContain("Article loading failed");
    expect(
      alert?.textContent?.includes(
        "Reloading the article will also retry loading its icon."
      )
    ).toBe(retry);
    // A second notification for the same article must not replace the first.
    await act(() =>
      push(new FaviconLoadError("https://example.org/article", !retry))
    );
    expect(
      alert?.textContent?.includes(
        "Reloading the article will also retry loading its icon."
      )
    ).toBe(retry);
  }
);

it("shows native network errors through the existing sink and diagnostic action", async () => {
  let deliver: (error: unknown) => void = () => {};
  const stop = vi.fn();
  const subscribe = vi.fn(async (callback: (error: unknown) => void) => {
    deliver = callback;
    return stop;
  });
  const container = document.createElement("div");
  document.body.appendChild(container);
  const root = createRoot(container);
  await act(() =>
    root.render(
      <ErrorSinkProvider subscribeErrors={subscribe}>{null}</ErrorSinkProvider>
    )
  );
  await act(() =>
    deliver({
      code: "proxy:blocked",
      message: "Remote content was blocked.",
      detail: "main; private evidence",
      diagnosticId: "network-1",
    })
  );
  expect(document.querySelector('[role="alert"]')?.textContent).toBe(
    "Remote content was blocked.Details"
  );
  expect(document.body.textContent).not.toContain("private evidence");
  await act(() => root.unmount());
  expect(stop).toHaveBeenCalledOnce();
});

it("shows the image-load message and diagnostics through the existing AppError contract", async () => {
  let push: (error: unknown) => void = () => {};
  function CaptureImageError() {
    const nextPush = useErrorSink().pushError;
    useEffect(() => {
      push = nextPush;
    }, [nextPush]);
    return null;
  }
  const container = document.createElement("div");
  document.body.appendChild(container);
  const root = createRoot(container);
  mounted.push(root);
  await act(() =>
    root.render(
      <ErrorSinkProvider>
        <CaptureImageError />
      </ErrorSinkProvider>
    )
  );
  const log = vi.spyOn(console, "error").mockImplementation(() => {});
  try {
    await act(() =>
      push({
        code: "app:error",
        message:
          "An image couldn’t be loaded. Reloading the article will retry loading its images.",
        detail: "One or more local article images could not be resolved.",
        diagnosticId: "image-attempt",
      })
    );
    expect(document.body.textContent).toContain("An image couldn’t be loaded.");
    expect(document.body.textContent).toContain("Details");
    expect(document.body.textContent).not.toContain("Something went wrong.");
    expect(document.body.textContent).not.toContain("One or more local");
  } finally {
    log.mockRestore();
  }
});

it("queues concurrent errors, retains diagnostic identity through Details, and times each plain toast separately", async () => {
  vi.useFakeTimers();
  const log = vi.spyOn(console, "error").mockImplementation(() => {});
  let push: (error: unknown) => void = () => {};
  function Capture() {
    const nextPush = useErrorSink().pushError;
    useEffect(() => {
      push = nextPush;
    }, [nextPush]);
    return null;
  }
  const read = vi.fn(async (id: string) => ({ id, events: [] }));
  const container = document.createElement("div");
  document.body.appendChild(container);
  const root = createRoot(container);
  mounted.push(root);
  const alert = () => document.querySelector('[role="alert"]')!;
  try {
    await act(() =>
      root.render(
        <PlatformContext.Provider
          value={{ getArticleDiagnostic: read } as unknown as Platform}
        >
          <ErrorSinkProvider>
            <Capture />
          </ErrorSinkProvider>
        </PlatformContext.Provider>
      )
    );
    await act(() => {
      push({
        code: "app:error",
        message: "First failure",
        detail: "",
        diagnosticId: "first",
      });
      push(new FaviconLoadError("https://example.org", false, "second"));
    });
    await act(() => vi.advanceTimersByTimeAsync(10000));
    expect(alert().textContent).toContain("First failure");
    await act(() =>
      [...alert().querySelectorAll("button")]
        .find((b) => b.textContent === "Details")!
        .click()
    );
    expect(read).toHaveBeenLastCalledWith("first");
    await act(() =>
      push({ code: "app:error", message: "Third failure", detail: "" })
    );
    await act(() =>
      [...document.querySelectorAll("button")]
        .find((b) => b.textContent === "Close")!
        .click()
    );
    expect(alert().textContent).toContain("First failure");
    await act(() =>
      alert()
        .querySelector<HTMLButtonElement>('button[aria-label="Close"]')!
        .click()
    );
    expect(alert().textContent).toContain("site icon");
    await act(() =>
      [...alert().querySelectorAll("button")]
        .find((b) => b.textContent === "Details")!
        .click()
    );
    expect(read).toHaveBeenLastCalledWith("second");
    await act(() =>
      [...document.querySelectorAll("button")]
        .find((b) => b.textContent === "Close")!
        .click()
    );
    await act(() =>
      alert()
        .querySelector<HTMLButtonElement>('button[aria-label="Close"]')!
        .click()
    );
    expect(alert().textContent).toContain("Third failure");
    await act(() =>
      push({ code: "app:error", message: "Fourth failure", detail: "" })
    );
    await act(() => vi.advanceTimersByTimeAsync(5999));
    expect(alert().textContent).toContain("Third failure");
    await act(() => vi.advanceTimersByTimeAsync(1));
    expect(alert().textContent).toContain("Fourth failure");
    await act(() => vi.advanceTimersByTimeAsync(5999));
    expect(alert().textContent).toContain("Fourth failure");
    await act(() => vi.advanceTimersByTimeAsync(1));
    expect(document.querySelector('[role="alert"]')).toBeNull();
  } finally {
    vi.useRealTimers();
    log.mockRestore();
  }
});
