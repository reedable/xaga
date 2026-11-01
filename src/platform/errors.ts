// The error codes the rust layer returns from commands:
//   ERROR_CODE ↔ DomainError (crates/xaga-core/src/error.rs)
//              + AppError    (src-tauri/src/error.rs)
// A rejected command rejects with a serialized `AppError`:
//   { code: ErrorCode; message: string; detail: string }
// `message` is the only user-safe field; `detail` is the full technical text,
// for the console/log only — never render it to a user.
//
// Hand-maintained against the rust enums. The rust tests pin each code list
// and this file pins the same set, so a code added on one side without the
// other fails a test rather than reaching the UI as an unhandled string.

export const ERROR_CODE = [
  "tags:invalid-store",
  "tags:invalid-name",
  "tags:deleting",
  "pins:invalid-store",
  // DomainError — EntryError
  "entry:io",
  "entry:invalid-url",
  "entry:invalid-id",
  "entry:not-found",
  "entry:malformed",
  "entry:too-large",
  "entry:color-change-requires-replace",
  "entry:invalid-color",
  // DomainError — FeedError
  "feed:invalid-url",
  "feed:http",
  "feed:io",
  "feed:atom-parse",
  "feed:parse",
  "feed:not-found",
  "feed:missing-self-link",
  "feed:body-too-large",
  "feed:http-blocked",
  "feed:writer",
  // DomainError — EmbeddingsError
  "embeddings:io",
  "embeddings:model-download",
  "embeddings:model",
  "embeddings:tokenize",
  "embeddings:lancedb",
  "embeddings:arrow",
  "embeddings:json",
  "embeddings:parse",
  // DomainError — TopicsError
  "topic:io",
  "topic:not-found",
  "topic:malformed",
  "topic:too-large",
  // DomainError — BookmarksError
  "bookmark:io",
  "bookmark:invalid-url",
  // DomainError — ItemError (own)
  "item:io",
  // DomainError — WriterError (own)
  "writer:shutting-down",
  "writer:closed",
  // DomainError — ProxyError
  "proxy:bad-request",
  "proxy:dns",
  "proxy:blocked",
  // AppError (src-tauri)
  "library:read-only",
  "command:unauthorized",
  "article:unsupported-download",
  "settings:stale-view",
  "settings:busy",
  "import:too-large",
  "settings:invalid-import-limit",
  "app:error", // transitional catch-all for not-yet-typed string errors
] as const;
export type ErrorCode = (typeof ERROR_CODE)[number];

/** The serialized shape a rejected command rejects with. */
export interface AppError {
  diagnosticId?: string;
  code: ErrorCode;
  /** User-safe; the only field to show a user. */
  message: string;
  /** Full technical text — console/log only, never rendered to a user. */
  detail: string;
}

const CODES = new Set<string>(ERROR_CODE);

/** Narrows an unknown rejection value to a serialized `AppError`. */
export function isAppError(e: unknown): e is AppError {
  return (
    typeof e === "object" &&
    e !== null &&
    typeof (e as { code?: unknown }).code === "string" &&
    CODES.has((e as { code: string }).code)
  );
}

/** The error's code, or `undefined` if it isn't a recognized `AppError`. */
export function errorCode(e: unknown): ErrorCode | undefined {
  return isAppError(e) ? e.code : undefined;
}

/**
 * A user-safe message for any rejection value. Prefers the `AppError.message`;
 * falls back to a generic sentence rather than leaking a raw string/object.
 */
export function errorMessage(e: unknown): string {
  if (isAppError(e)) return e.message;
  return "Something went wrong.";
}
