import type { AppError } from "../platform/errors";

export function importFailure(
  kind: "feeds" | "bookmarks",
  outcome: { error: AppError | null; result: { failed: number } | null }
): AppError | null {
  if (outcome.error) return outcome.error;
  const failed = outcome.result?.failed ?? 0;
  return failed > 0
    ? {
        code: "app:error",
        message: `${failed} ${kind.slice(0, -1)}${failed === 1 ? "" : "s"} could not be imported. Successfully imported items have been kept.`,
        detail: "",
      }
    : null;
}
