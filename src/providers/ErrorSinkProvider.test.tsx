import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useErrorSink } from "../hooks/useErrorSink";
import { PlatformContext, type Platform } from "../platform/Platform";
import type { ArticleDiagnostic } from "../types";
import ArticleLoadError from "../utils/ArticleLoadError";
import ErrorSinkProvider from "./ErrorSinkProvider";

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
