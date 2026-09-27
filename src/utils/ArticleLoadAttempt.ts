import type { Platform } from "../platform/Platform";
import { errorCode } from "../platform/errors";
import {
  isAbortError,
  type ArticleDiagnosticOutcome,
  type ArticleDiagnosticStep,
} from "../types";
import ArticleLoadError from "./ArticleLoadError";

/** A single frontend operation, with native events joined by the same ID.
 * Diagnostic transport failures must never prevent reading an article.
 */
export default class ArticleLoadAttempt {
  private pending: Promise<void> = Promise.resolve();
  private step: ArticleDiagnosticStep = "attempt";
  private readonly platform: Platform;
  readonly id: string;
  private readonly signal?: AbortSignal;

  private constructor(platform: Platform, id: string, signal?: AbortSignal) {
    this.platform = platform;
    this.id = id;
    this.signal = signal;
  }

  static async create(platform: Platform, mode: string, signal?: AbortSignal) {
    const id = await platform.createArticleDiagnostic(mode).catch(() => "");
    return new ArticleLoadAttempt(platform, id, signal);
  }

  addEvent(
    step: ArticleDiagnosticStep,
    outcome: ArticleDiagnosticOutcome,
    code = ""
  ) {
    if (!this.id) return;
    this.pending = this.pending
      .then(() =>
        this.platform.addArticleDiagnosticEvent(this.id, step, outcome, code)
      )
      .catch(() => {});
  }

  /** Called inside extraction so its precise failing phase survives the catch. */
  setStep = (step: ArticleDiagnosticStep) => {
    if (this.step !== "attempt" && this.step !== "extract")
      this.addEvent(this.step, "succeeded");
    this.step = step;
    this.addEvent(step, "started");
  };

  async runStep<T>(
    step: ArticleDiagnosticStep,
    fn: () => T | Promise<T>
  ): Promise<T> {
    this.step = step;
    this.addEvent(step, "started");
    try {
      if (this.signal?.aborted)
        throw new DOMException("Cancelled", "AbortError");
      await this.pending;
      if (this.signal?.aborted)
        throw new DOMException("Cancelled", "AbortError");
      const value = await fn();
      if (this.signal?.aborted)
        throw new DOMException("Cancelled", "AbortError");
      this.addEvent(this.step, "succeeded");
      if (this.step !== step) this.addEvent(step, "succeeded");
      this.step = "attempt";
      return value;
    } catch (error) {
      if (error instanceof ArticleLoadError) throw error;
      const cancelled = isAbortError(error);
      this.addEvent(
        this.step,
        cancelled ? "cancelled" : "failed",
        getErrorCode(error, this.step)
      );
      if (cancelled) throw error;
      throw new ArticleLoadError(this.id, this.step, error);
    }
  }

  async complete(outcome: ArticleDiagnosticOutcome = "succeeded") {
    this.addEvent("attempt", outcome);
    await this.pending;
  }
}

function getErrorCode(error: unknown, step: ArticleDiagnosticStep): string {
  if (isAbortError(error)) return "AbortError";

  const code = errorCode(error);
  if (code) return code;

  // Exception messages may quote article content. Only this known extraction
  // failure and standard error names are included in the diagnostic payload.
  if (error instanceof Error) {
    if (
      step === "readability" &&
      error.message === "Readability could not extract an entry"
    ) {
      return "article:no-readable-content";
    }
    if (["TypeError", "RangeError", "SyntaxError"].includes(error.name)) {
      return error.name;
    }
  }
  return "article:unknown";
}
