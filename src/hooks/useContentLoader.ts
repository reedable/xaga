import { useCallback, useEffect, useState, type RefObject } from "react";
import { usePlatform } from "../platform/Platform";
import { isAbortError } from "../types";
import ArticleLoadAttempt from "../utils/ArticleLoadAttempt";
import { base64ToBytes, useArticles } from "./useArticles";
import { useErrorSink } from "./useErrorSink";
import useRouter from "./useRouter";
import { useSettings } from "./useSettings";

/** Resolve a URL from local storage or ingestion, then present validated content. */
export function useContentLoader(
  abortControllerRef: RefObject<AbortController>
) {
  const platform = usePlatform();
  const { inlinePdfReader } = useSettings();
  const { pushError } = useErrorSink();
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
  // The URL whose downloaded content was rejected.
  const [blockedUrl, setBlockedUrl] = useState<string | null>(null);

  const resolve = useCallback(
    async (url: string, refresh = false) => {
      const { signal } = abortControllerRef.current;
      setPdfBlobUrl(null);
      setBlockedUrl(null);

      const attempt = await ArticleLoadAttempt.create(
        platform,
        refresh ? "refresh" : "load",
        signal
      );
      let outcome: "succeeded" | "failed" | "cancelled" = "succeeded";
      try {
        if (!refresh) {
          const stored = await attempt.runStep("lookup", () =>
            platform.readStoredPdf(signal, url)
          );
          signal.throwIfAborted();
          if (stored) {
            if (!inlinePdfReader) {
              await platform.openStoredPdf(url);
              signal.throwIfAborted();
            } else if (stored.pdfBase64) {
              setPdfBlobUrl(
                URL.createObjectURL(
                  new Blob([base64ToBytes(stored.pdfBase64)], {
                    type: "application/pdf",
                  })
                )
              );
            }
            replaceMimeType(
              inlinePdfReader && stored.pdfBase64
                ? "application/pdf"
                : "text/html"
            );
            return;
          }
          const existing = await findArticleByUrl(url, attempt);
          signal.throwIfAborted();
          if (existing?.content) {
            replaceEntry(existing);
            return;
          }
        }

        const result = await loadArticleByUrl(url, false, attempt);
        signal.throwIfAborted();
        if (!result) return;

        // Show a neutral rejection message for this URL.
        if ("blockedDownload" in result) {
          outcome = "failed";
          setBlockedUrl(url);
          replaceMimeType("text/html"); // clear the loading placeholder
          return;
        }

        if (result.mimeType === "application/pdf") {
          const bytes = "pdfBytes" in result ? result.pdfBytes : null;
          if (inlinePdfReader && bytes) {
            setPdfBlobUrl(
              await attempt.runStep("render", () =>
                URL.createObjectURL(
                  new Blob([bytes], { type: "application/pdf" })
                )
              )
            );
            replaceMimeType("application/pdf");
          } else {
            // Open only the native URL-derived, validated local PDF.
            await platform.openStoredPdf(url);
            signal.throwIfAborted();
            replaceMimeType("text/html");
          }
          return;
        }

        if (result.article) {
          replaceEntry(result.article);
        }
      } catch (error) {
        outcome = isAbortError(error) ? "cancelled" : "failed";
        pushError(error);
      } finally {
        await attempt.complete(outcome);
      }
    },
    [
      abortControllerRef,
      inlinePdfReader,
      findArticleByUrl,
      loadArticleByUrl,
      platform,
      pushError,
      replaceEntry,
      replaceMimeType,
    ]
  );

  return { pdfBlobUrl, blockedUrl, resolve };
}
