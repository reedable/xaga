import { isAppError, type AppError, type ErrorCode } from "../platform/errors";
import type { FeedRefreshFailure } from "../types";

/** A refresh report retains structured failures for persistent, readable presentation. */
export class FeedRefreshError extends Error implements AppError {
  readonly code: ErrorCode;
  readonly detail: string;
  readonly failures: readonly FeedRefreshFailure[];

  constructor(
    message: string,
    code: ErrorCode,
    detail: string,
    failures: readonly FeedRefreshFailure[] = []
  ) {
    super(message);
    this.name = "FeedRefreshError";
    this.code = code;
    this.detail = detail;
    this.failures = failures.map((failure) => ({ ...failure }));
  }
}

export function feedRefreshError(
  failures: FeedRefreshFailure[]
): FeedRefreshError {
  return new FeedRefreshError(
    `${failures.length} ${failures.length === 1 ? "feed couldn’t" : "feeds couldn’t"} be refreshed.`,
    "app:error",
    failures.map((failure) => `${failure.id}: ${failure.code}`).join("; "),
    failures
  );
}

export function feedRefreshRequestError(error: unknown): FeedRefreshError {
  return new FeedRefreshError(
    isAppError(error) ? error.message : "Couldn’t refresh feeds.",
    isAppError(error) ? error.code : "app:error",
    isAppError(error) ? error.detail : String(error)
  );
}
