import { useCallback, useEffect, useState, type RefObject } from "react";
import { usePlatform } from "../platform/Platform";
import { isAbortError } from "../types";
import ArticleLoadAttempt from "../utils/ArticleLoadAttempt";
import { useArticles } from "./useArticles";
import { useErrorSink } from "./useErrorSink";
import useRouter from "./useRouter";
import { useSettings } from "./useSettings";

/**
 * Resolves a bare URL — one a link click or the URL bar pushed with no
 * entryId/mimeType yet — into a concrete presentation, then writes the result
 * back onto the current route entry:
 *
 *  - PDF (inline enabled, bytes)   → offloads the bytes to a Blob and exposes
 *    its URL for inline rendering.
 *  - PDF (reqwest blocked)         → the loader webview shows it in its own
 *    window; nothing renders inline.
 *  - PDF (inline disabled)         → open it externally.
 *  - HTML                          → the article is persisted; set entryId so
 *    the reader renders it.
 *  - spoof (Content-Type claimed PDF, body isn't) → record the blocked URL so
 *    ItemView can show a block message.
 *
 * Entries and annotations selected from lists already carry an entryId and are
 * rendered by ArticleReader directly, so the owner only invokes `resolve` for
 * bare URLs. The owner (ItemView) holds the AbortController and drives
 * cancellation: `abort(); resolve(url); return abort;`.
 */
export function useContentLoader(
  abortControllerRef: RefObject<AbortController>
) {
  const platform = usePlatform();
  const { pushError } = useErrorSink();
  const { inlinePdfReader } = useSettings();
  const { replaceEntry, replaceMimeType } = useRouter();
  const { findArticleByUrl, loadArticleByUrl } =
    useArticles(abortControllerRef);

  // Blob URL for an inline PDF. The hook owns the blob's lifecycle: the bytes
  // are offloaded to blob storage immediately (never retained in state) and the
  // URL is revoked on replacement and on unmount.
  const [pdfBlobUrl, setPdfBlobUrl] = useState<string | null>(null);

  // Revoking the previous URL while an iframe still displays it is safe — an
  // already-loaded document keeps its blob alive. If StrictMode is ever
  // re-enabled, the remount cleanup revokes a URL that cannot be recreated,
  // since the bytes are deliberately not retained.
  useEffect(() => {
    if (!pdfBlobUrl) return;
    return () => URL.revokeObjectURL(pdfBlobUrl);
  }, [pdfBlobUrl]);
  // The URL whose inline-PDF probe was blocked as a spoof, so ItemView can show
  // a block message for exactly that URL.
  const [blockedUrl, setBlockedUrl] = useState<string | null>(null);

  const resolve = useCallback(
    async (url: string) => {
      const { signal } = abortControllerRef.current;
      setPdfBlobUrl(null);
      setBlockedUrl(null);

      const attempt = await ArticleLoadAttempt.create(platform, "load", signal);
      let outcome: "succeeded" | "failed" | "cancelled" = "succeeded";
      try {
        // Already saved? Point the route at the entry without re-fetching.
        const existing = await findArticleByUrl(url, attempt);
        signal.throwIfAborted();

        if (existing) {
          replaceEntry(existing);
          return;
        }

        const result = await loadArticleByUrl(url, false, attempt);
        signal.throwIfAborted();
        if (!result) return;

        // Blocked as a spoof — show a message for this URL.
        if ("blockedSpoof" in result) {
          outcome = "failed";
          setBlockedUrl(url);
          replaceMimeType("text/html"); // clear the loading placeholder
          return;
        }

        if (result.mimeType === "application/pdf") {
          const bytes = "pdfBytes" in result ? result.pdfBytes : null;
          if (bytes) {
            setPdfBlobUrl(
              await attempt.runStep("render", () =>
                URL.createObjectURL(
                  new Blob([bytes], { type: "application/pdf" })
                )
              )
            );
            replaceMimeType("application/pdf");
          } else if (inlinePdfReader) {
            // reqwest was blocked but the loader webview found a PDF and is now
            // showing it in its own window — nothing to render inline.
            replaceMimeType("text/html");
          } else {
            // Inline PDF disabled — open externally and mark resolved.
            await attempt.runStep("render", () => platform.openUrl(url));
            replaceMimeType("text/html");
          }
          return;
        }

        if (result.article) {
          replaceEntry(result.article);
        }
      } catch (error) {
        outcome =
          isAbortError(error)
            ? "cancelled"
            : "failed";
        pushError(error);
      } finally {
        await attempt.complete(outcome);
      }
    },
    [
      abortControllerRef,
      findArticleByUrl,
      loadArticleByUrl,
      inlinePdfReader,
      platform,
      pushError,
      replaceEntry,
      replaceMimeType,
    ]
  );

  return { pdfBlobUrl, blockedUrl, resolve };
}
