import { isAppError, type AppError } from "../platform/errors";
import type { FeedRefreshFailure } from "../types";

export function feedRefreshError(failures: FeedRefreshFailure[]): AppError {
  return {
    code: "app:error",
    message: `Couldn't refresh ${failures.length === 1 ? "this feed" : "these feeds"}. ${failures.map((failure) => `${failure.title}: ${failure.message}`).join(" ")} Use Refresh to retry.`,
    detail: failures
      .map((failure) => `${failure.id}: ${failure.code}`)
      .join("; "),
  };
}

export function feedRefreshRequestError(error: unknown): AppError {
  return {
    code: isAppError(error) ? error.code : "app:error",
    message: `${isAppError(error) ? error.message : "Couldn't refresh feeds."} Use Refresh to retry.`,
    detail: isAppError(error) ? error.detail : String(error),
  };
}
