import { useCallback, useState, type RefObject } from "react";
import { usePlatform } from "../platform/Platform";
import { errorCode } from "../platform/errors";
import { processEntry, type LoaderFavicon } from "../text/entryWorker";
import { isAbortError, type Entry } from "../types";
import ArticleLoadAttempt from "../utils/ArticleLoadAttempt";
import ArticleLoadError from "../utils/ArticleLoadError";
import { useErrorSink } from "./useErrorSink";

/**
 * Pairs the loader's favicon URL with its bytes, or null when the loader did not
 * come back with both. Only a complete pair is usable: the URL is what the
 * backend hashes, and the bytes are what it writes.
 */
export function toLoaderFavicon(
  url?: string | null,
  dataUri?: string | null
): LoaderFavicon | null {
  return url && dataUri ? { url, dataUri } : null;
}

/** Decode base64 (PDF bytes from the Rust inline-PDF path) into a byte array. */
function base64ToBytes(base64: string): Uint8Array<ArrayBuffer> {
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
  return bytes;
}

export function useArticles(abortControllerRef: RefObject<AbortController>) {
  const platform = usePlatform();
  const { pushError } = useErrorSink();
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<unknown>(null);

  const reportRecoveredTimeout = useCallback(
    async (attempt: ArticleLoadAttempt) => {
      if (!attempt.id) return;
      const diagnostic = await platform
        .getArticleDiagnostic(attempt.id)
        .catch(() => null);
      if (
        diagnostic?.events.some((event) => event.code === "article:timeout")
      ) {
        pushError(
          new ArticleLoadError(
            attempt.id,
            "fetch",
            new Error(
              "The page did not finish loading in time. Processing the available content; some content may be missing."
            )
          )
        );
      }
    },
    [platform, pushError]
  );

  /**
   * Loads and saves a remote content identified by URL.
   *
   * WARNING: Do not use this method if you already have an Entry or Article
   * object. To refresh an existing article, call `loadArticle` instead.
   */
  const loadArticleByUrl = useCallback(
    async (url: string, keepOpen = false, parent?: ArticleLoadAttempt) => {
      const signal = abortControllerRef.current.signal;
      setError(null);
      setLoading(true);

      const attempt =
        parent ??
        (await ArticleLoadAttempt.create(
          platform,
          keepOpen ? "visible" : "load",
          signal
        ));
      let outcome: "succeeded" | "failed" | "cancelled" = "succeeded";
      try {
        const {
          mimeType,
          html,
          pdfBase64,
          favicon: faviconUrl,
          faviconData,
        } = await attempt.runStep("fetch", () =>
          platform.loadArticle(signal, url, keepOpen, attempt.id)
        );

        await reportRecoveredTimeout(attempt);
        if (mimeType === "application/pdf") {
          const pdfBytes = pdfBase64
            ? await attempt.runStep("pdf_decode", () => base64ToBytes(pdfBase64))
            : null;
          return { article: null, mimeType, pdfBytes };
        }

        const { title, markdown, favicon, imageData } = await attempt.runStep(
          "extract",
          () =>
            processEntry(
              url,
              html,
              toLoaderFavicon(faviconUrl, faviconData),
              attempt.setStep
            )
        );

        const article = await attempt.runStep("create", () =>
          platform.createArticle(
            signal,
            {
              title,
              content: markdown,
              url,
              annotations: [],
              favicon: favicon ?? undefined,
            },
            imageData,
            attempt.id
          )
        );

        return {
          article: await attempt.runStep("readback", () =>
            platform.readArticle(signal, article.id, attempt.id)
          ),
          mimeType,
        };
      } catch (e) {
        outcome =
          isAbortError(e)
            ? "cancelled"
            : "failed";
        // The inline-PDF probe rejected the URL as a spoof (Content-Type claimed
        // PDF, body isn't). Surface it so the reader pane can show a block
        // message rather than swallowing it as a generic failure.
        if (
          errorCode(e instanceof ArticleLoadError ? e.cause : e) ===
          "article:spoofed-pdf"
        ) {
          pushError(e);
          attempt.addEvent("attempt", "failed", "article:spoofed-pdf");
          return { article: null, blockedSpoof: true };
        }
        if (parent) throw e;
        pushError(e);
      } finally {
        if (!parent) await attempt.complete(outcome);
        setLoading(false);
      }
    },
    [abortControllerRef, platform, pushError, reportRecoveredTimeout]
  );

  /**
   * Loads the remote content over the network, and saves the fresh content to
   * the disc.
   */
  const loadArticle = useCallback(
    async (entry: Entry, keepOpen = false, parent?: ArticleLoadAttempt) => {
      const signal = abortControllerRef.current.signal;
      setError(null);
      setLoading(true);

      const attempt =
        parent ??
        (await ArticleLoadAttempt.create(
          platform,
          keepOpen ? "visible" : "refresh",
          signal
        ));
      let outcome: "succeeded" | "failed" | "cancelled" = "succeeded";
      try {
        const {
          html,
          favicon: faviconUrl,
          faviconData,
        } = await attempt.runStep("fetch", () =>
          platform.loadArticle(signal, entry.url, keepOpen, attempt.id)
        );
        await reportRecoveredTimeout(attempt);
        const { title, markdown, favicon, imageData } = await attempt.runStep(
          "extract",
          () =>
            processEntry(
              entry.url,
              html,
              toLoaderFavicon(faviconUrl, faviconData),
              attempt.setStep
            )
        );

        const article = await attempt.runStep("readback", () =>
          platform.readArticle(signal, entry.id, attempt.id)
        );

        await attempt.runStep("save", () =>
          platform.saveArticle(
            signal,
            {
              ...article,
              title,
              content: markdown,
              favicon: favicon ?? undefined,
            },
            imageData,
            attempt.id
          )
        );

        return await attempt.runStep("readback", () =>
          platform.readArticle(signal, article.id, attempt.id)
        );
      } catch (e) {
        outcome =
          isAbortError(e)
            ? "cancelled"
            : "failed";
        if (parent) throw e;
        pushError(e);
      } finally {
        if (!parent) await attempt.complete(outcome);
        setLoading(false);
      }
    },
    [abortControllerRef, platform, pushError, reportRecoveredTimeout]
  );

  /**
   * Reads the entry from disk. Fast, no loading state.
   */
  const readEntry = useCallback(
    async (entryId: Entry["id"]) => {
      const signal = abortControllerRef.current.signal;
      setError(null);
      const attempt = await ArticleLoadAttempt.create(platform, "read", signal);
      let outcome: "succeeded" | "failed" | "cancelled" = "succeeded";
      try {
        return await attempt.runStep("read", () =>
          platform.readEntry(signal, entryId)
        );
      } catch (e) {
        outcome =
          isAbortError(e)
            ? "cancelled"
            : "failed";
        pushError(e);
      } finally {
        await attempt.complete(outcome);
      }
    },
    [abortControllerRef, platform, pushError]
  );

  /**
   * Reads the locally saved article and returns it if content is present.
   * Otherwise loads from the network and returns the fresh content.
   * Call `loadArticle` instead if you need the fresh content every time.
   *
   * WARNING: `loadArticle` implicitly calls `save_article`, which moves the
   * md file from unread/ to read/ directory, i.e. marks the item read.
   */
  const readArticle = useCallback(
    async (entryId: Entry["id"]) => {
      const signal = abortControllerRef.current.signal;
      setError(null);

      const attempt = await ArticleLoadAttempt.create(platform, "read", signal);
      let outcome: "succeeded" | "failed" | "cancelled" = "succeeded";
      try {
        const article = await attempt.runStep("readback", () =>
          platform.readArticle(signal, entryId, attempt.id)
        );

        if (article.content) {
          return article;
        }

        // Content missing — fetch from network. loadArticle manages loading state.
        return await loadArticle(article, false, attempt);
      } catch (e) {
        outcome =
          isAbortError(e)
            ? "cancelled"
            : "failed";
        pushError(e);
      } finally {
        await attempt.complete(outcome);
      }
    },
    [abortControllerRef, loadArticle, platform, pushError]
  );

  const extractArticleFromHtml = useCallback(
    async (url: string, html: string, loaderFavicon?: LoaderFavicon | null) => {
      const attempt = await ArticleLoadAttempt.create(platform, "extract");
      try {
        const result = await attempt.runStep("extract", () =>
          processEntry(url, html, loaderFavicon, attempt.setStep)
        );
        if (!result.markdown) {
          attempt.addEvent("extract", "warning", "article:empty");
          await attempt.complete("warning");
          return null;
        }
        await attempt.complete();
        return result;
      } catch (error) {
        await attempt.complete("failed");
        pushError(error);
        return null;
      }
    },
    [platform, pushError]
  );

  const saveExtractedArticle = useCallback(
    async (
      entry: Entry,
      extracted: {
        title: string;
        markdown: string;
        favicon: string | null;
        imageData?: Record<string, string>;
      }
    ) => {
      const signal = abortControllerRef.current.signal;
      setError(null);
      setLoading(true);

      const attempt = await ArticleLoadAttempt.create(platform, "refresh", signal);
      let outcome: "succeeded" | "failed" | "cancelled" = "succeeded";
      try {
        const article = await attempt.runStep("readback", () =>
          platform.readArticle(signal, entry.id, attempt.id)
        );

        await attempt.runStep("save", () =>
          platform.saveArticle(
            signal,
            {
              ...article,
              title: extracted.title,
              content: extracted.markdown,
              favicon: extracted.favicon ?? undefined,
            },
            extracted.imageData ?? {},
            attempt.id
          )
        );

        return await attempt.runStep("readback", () =>
          platform.readArticle(signal, article.id, attempt.id)
        );
      } catch (e) {
        outcome =
          isAbortError(e)
            ? "cancelled"
            : "failed";
        pushError(e);
      } finally {
        await attempt.complete(outcome);
        setLoading(false);
      }
    },
    [abortControllerRef, platform, pushError]
  );

  const findArticleByUrl = useCallback(
    async (url: string, parent?: ArticleLoadAttempt) => {
      const signal = abortControllerRef.current.signal;
      setError(null);
      setLoading(true);

      const attempt =
        parent ?? (await ArticleLoadAttempt.create(platform, "read", signal));
      let outcome: "succeeded" | "failed" | "cancelled" = "succeeded";
      try {
        return await attempt.runStep("lookup", () =>
          platform.findArticle(signal, url, attempt.id)
        );
      } catch (e) {
        outcome =
          isAbortError(e)
            ? "cancelled"
            : "failed";
        if (parent) throw e;
        pushError(e);
      } finally {
        if (!parent) await attempt.complete(outcome);
        setLoading(false);
      }
    },
    [abortControllerRef, platform, pushError]
  );

  const createArticleFromHtml = useCallback(
    async (url: string, html: string, loaderFavicon?: LoaderFavicon | null) => {
      const signal = abortControllerRef.current.signal;
      setError(null);
      setLoading(true);

      const attempt = await ArticleLoadAttempt.create(platform, "extract", signal);
      let outcome: "succeeded" | "failed" | "cancelled" = "succeeded";
      try {
        const { title, markdown, favicon, imageData } = await attempt.runStep(
          "extract",
          () => processEntry(url, html, loaderFavicon, attempt.setStep)
        );
        const article = await attempt.runStep("create", () =>
          platform.createArticle(
            signal,
            {
              title,
              content: markdown,
              url,
              annotations: [],
              favicon: favicon ?? undefined,
            },
            imageData,
            attempt.id
          )
        );
        return await attempt.runStep("readback", () =>
          platform.readArticle(signal, article.id, attempt.id)
        );
      } catch (e) {
        outcome =
          isAbortError(e)
            ? "cancelled"
            : "failed";
        pushError(e);
      } finally {
        await attempt.complete(outcome);
        setLoading(false);
      }
    },
    [abortControllerRef, platform, pushError]
  );

  return {
    error,
    loading,
    readEntry,
    readArticle,
    loadArticle,
    loadArticleByUrl,
    findArticleByUrl,
    createArticleFromHtml,
    extractArticleFromHtml,
    saveExtractedArticle,
  };
}
