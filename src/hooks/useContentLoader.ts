import { useCallback, useState, type RefObject } from "react";
import { usePlatform } from "../platform/Platform";
import { useArticles } from "./useArticles";
import useRouter from "./useRouter";
import { useSettings } from "./useSettings";

/**
 * Resolves a bare URL — one a link click or the URL bar pushed with no
 * entryId/mimeType yet — into a concrete presentation, then writes the result
 * back onto the current route entry:
 *
 *  - PDF (inline enabled, bytes)   → exposes the bytes for inline rendering.
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
  const { inlinePdfReader } = useSettings();
  const { replaceEntry, replaceMimeType } = useRouter();
  const { findArticleByUrl, loadArticleByUrl } = useArticles(abortControllerRef);

  const [pdfBytes, setPdfBytes] = useState<Uint8Array<ArrayBuffer> | null>(
    null
  );
  // The URL whose inline-PDF probe was blocked as a spoof, so ItemView can show
  // a block message for exactly that URL.
  const [blockedUrl, setBlockedUrl] = useState<string | null>(null);

  const resolve = useCallback(
    async (url: string) => {
      const { signal } = abortControllerRef.current;
      setPdfBytes(null);
      setBlockedUrl(null);

      // Already saved? Point the route at the entry without re-fetching.
      const existing = await findArticleByUrl(url);
      if (signal.aborted) return;

      if (existing) {
        replaceEntry(existing);
        return;
      }

      const result = await loadArticleByUrl(url);
      if (signal.aborted || !result) return;

      // Blocked as a spoof — show a message for this URL.
      if ("blockedSpoof" in result) {
        setBlockedUrl(url);
        replaceMimeType("text/html"); // clear the loading placeholder
        return;
      }

      if (result.mimeType === "application/pdf") {
        const bytes = "pdfBytes" in result ? result.pdfBytes : null;
        if (bytes) {
          setPdfBytes(bytes);
          replaceMimeType("application/pdf");
        } else if (inlinePdfReader) {
          // reqwest was blocked but the loader webview found a PDF and is now
          // showing it in its own window — nothing to render inline.
          replaceMimeType("text/html");
        } else {
          // Inline PDF disabled — open externally and mark resolved.
          await platform.openUrl(url);
          replaceMimeType("text/html");
        }
        return;
      }

      if (result.article) {
        replaceEntry(result.article);
      }
    },
    [
      abortControllerRef,
      findArticleByUrl,
      loadArticleByUrl,
      inlinePdfReader,
      platform,
      replaceEntry,
      replaceMimeType,
    ]
  );

  return { pdfBytes, blockedUrl, resolve };
}
