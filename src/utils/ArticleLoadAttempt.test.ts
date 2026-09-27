import { describe, expect, it, vi } from "vitest";
import type { Platform } from "../platform/Platform";
import { processEntry } from "../text/entryWorker";
import ArticleLoadAttempt from "./ArticleLoadAttempt";
import ArticleLoadError from "./ArticleLoadError";

function harness() {
  let id = 0;
  const record = vi.fn().mockResolvedValue(undefined);
  const platform = {
    createArticleDiagnostic: vi.fn(async () => `attempt-${++id}`),
    addArticleDiagnosticEvent: record,
  } as unknown as Platform;
  return { platform, record };
}

describe("article diagnostics", () => {
  it("records the actual extraction failure without HTML or URL contents", async () => {
    const { platform, record } = harness();
    const attempt = await ArticleLoadAttempt.create(platform, "load");
    const error = await attempt
      .runStep("extract", () =>
        processEntry(
          "https://user:secret@example.com/?token=secret",
          "<html></html>",
          null,
          attempt.setStep
        )
      )
      .catch((e) => e);
    expect(error).toBeInstanceOf(ArticleLoadError);
    expect(error.step).toBe("readability");
    await attempt.complete("failed");
    expect(record).toHaveBeenCalledWith(
      "attempt-1",
      "readability",
      "failed",
      "article:no-readable-content"
    );
    expect(JSON.stringify(record.mock.calls)).not.toMatch(
      /secret|<html>|https:/
    );
  });

  it("keeps interleaved attempts separate and preserves typed storage failures", async () => {
    const { platform, record } = harness();
    const a = await ArticleLoadAttempt.create(platform, "load");
    const b = await ArticleLoadAttempt.create(platform, "refresh");
    await Promise.all([
      a
        .runStep("save", () => {
          throw {
            code: "entry:io",
            message: "Could not save.",
            detail: "secret body",
          };
        })
        .catch(() => {}),
      b.runStep("save", () => 42),
    ]);
    await Promise.all([a.complete("failed"), b.complete()]);
    expect(record).toHaveBeenCalledWith(a.id, "save", "failed", "entry:io");
    expect(record).toHaveBeenCalledWith(b.id, "save", "succeeded", "");
    expect(JSON.stringify(record.mock.calls)).not.toContain("secret body");
  });

  it("records cancellation separately and never starts cancelled work", async () => {
    const { platform, record } = harness();
    const controller = new AbortController();
    const attempt = await ArticleLoadAttempt.create(
      platform,
      "load",
      controller.signal
    );
    controller.abort();
    const work = vi.fn();
    await expect(attempt.runStep("fetch", work)).rejects.toMatchObject({
      name: "AbortError",
    });
    await attempt.complete("cancelled");
    expect(work).not.toHaveBeenCalled();
    expect(record).toHaveBeenCalledWith(
      attempt.id,
      "fetch",
      "cancelled",
      "AbortError"
    );
    expect(record.mock.calls.some((call) => call[2] === "failed")).toBe(false);
  });

  it("does not turn unavailable diagnostic transport into an article failure", async () => {
    const { platform, record } = harness();
    record.mockRejectedValue(new Error("IPC unavailable"));
    const attempt = await ArticleLoadAttempt.create(platform, "load");
    await expect(attempt.runStep("fetch", () => 42)).resolves.toBe(42);
    await attempt.complete();
    vi.mocked(platform.createArticleDiagnostic).mockRejectedValue(
      new Error("unavailable")
    );
    const fallback = await ArticleLoadAttempt.create(platform, "load");
    await expect(fallback.runStep("read", () => 42)).resolves.toBe(42);
  });
});
