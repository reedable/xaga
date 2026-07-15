import { useCallback, useState, type RefObject } from "react";
import { errorCode } from "../platform/errors";
import { usePlatform } from "../platform/Platform";
import { processEntry } from "../text/entryWorker";
import { type Entry } from "../types";

/** Decode base64 (PDF bytes from the Rust inline-PDF path) into a byte array. */
function base64ToBytes(base64: string): Uint8Array<ArrayBuffer> {
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
  return bytes;
}

export function useArticles(abortControllerRef: RefObject<AbortController>) {
  const platform = usePlatform();
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<unknown>(null);

  /**
   * Loads and saves a remote content identified by URL.
   *
   * WARNING: Do not use this method if you already have an Entry or Article
   * object. To refresh an existing article, call `loadArticle` instead.
   */
  const loadArticleByUrl = useCallback(
    async (url: string, keepOpen = false) => {
      const signal = abortControllerRef.current.signal;
      setError(null);
      setLoading(true);

      try {
        const { mimeType, html, pdfBase64 } = await platform.loadArticle(
          signal,
          url,
          keepOpen
        );

        if (mimeType === "application/pdf") {
          const pdfBytes = pdfBase64 ? base64ToBytes(pdfBase64) : null;
          return { article: null, mimeType, pdfBytes };
        }

        const { title, markdown, favicon } = await processEntry(url, html);

        const article = await platform.createArticle(signal, {
          title,
          content: markdown,
          url,
          annotations: [],
          favicon: favicon ?? undefined,
        });

        return {
          article: await platform.readArticle(signal, article.id),
          mimeType,
        };
      } catch (e) {
        // The inline-PDF probe rejected the URL as a spoof (Content-Type claimed
        // PDF, body isn't). Surface it so the reader pane can show a block
        // message rather than swallowing it as a generic failure.
        if (errorCode(e) === "article:spoofed-pdf") {
          return { article: null, blockedSpoof: true };
        }
        setError(e);
      } finally {
        setLoading(false);
      }
    },
    [abortControllerRef, platform]
  );

  /**
   * Loads the remote content over the network, and saves the fresh content to
   * the disc.
   */
  const loadArticle = useCallback(
    async (entry: Entry, keepOpen = false) => {
      const signal = abortControllerRef.current.signal;
      setError(null);
      setLoading(true);

      try {
        const { html } = await platform.loadArticle(
          signal,
          entry.url,
          keepOpen
        );
        const { title, markdown, favicon } = await processEntry(
          entry.url,
          html
        );

        const article = await platform.readArticle(signal, entry.id);

        await platform.saveArticle(signal, {
          ...article,
          title,
          content: markdown,
          favicon: favicon ?? undefined,
        });

        return await platform.readArticle(signal, article.id);
      } catch (e) {
        setError(e);
      } finally {
        setLoading(false);
      }
    },
    [abortControllerRef, platform]
  );

  /**
   * Reads the entry from disk. Fast, no loading state.
   */
  const readEntry = useCallback(
    async (entryId: Entry["id"]) => {
      const signal = abortControllerRef.current.signal;
      setError(null);
      try {
        return await platform.readEntry(signal, entryId);
      } catch (e) {
        setError(e);
      }
    },
    [abortControllerRef, platform]
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

      try {
        const article = await platform.readArticle(signal, entryId);

        if (article.content) {
          return article;
        }

        // Content missing — fetch from network. loadArticle manages loading state.
        return await loadArticle(article);
      } catch (e) {
        setError(e);
      }
    },
    [abortControllerRef, loadArticle, platform]
  );

  const extractArticleFromHtml = useCallback(
    async (url: string, html: string) => {
      try {
        const result = await processEntry(url, html);
        if (!result.markdown) return null;
        return result;
      } catch {
        return null;
      }
    },
    []
  );

  const saveExtractedArticle = useCallback(
    async (
      entry: Entry,
      extracted: { title: string; markdown: string; favicon: string | null }
    ) => {
      const signal = abortControllerRef.current.signal;
      setError(null);
      setLoading(true);

      try {
        const article = await platform.readArticle(signal, entry.id);

        await platform.saveArticle(signal, {
          ...article,
          title: extracted.title,
          content: extracted.markdown,
          favicon: extracted.favicon ?? undefined,
        });

        return await platform.readArticle(signal, article.id);
      } catch (e) {
        setError(e);
      } finally {
        setLoading(false);
      }
    },
    [abortControllerRef, platform]
  );

  const findArticleByUrl = useCallback(
    async (url: string) => {
      const signal = abortControllerRef.current.signal;
      setError(null);
      setLoading(true);

      try {
        return await platform.findArticle(signal, url);
      } catch (e) {
        setError(e);
      } finally {
        setLoading(false);
      }
    },
    [abortControllerRef, platform]
  );

  const createArticleFromHtml = useCallback(
    async (url: string, html: string) => {
      const signal = abortControllerRef.current.signal;
      setError(null);
      setLoading(true);

      try {
        const { title, markdown, favicon } = await processEntry(url, html);
        const article = await platform.createArticle(signal, {
          title,
          content: markdown,
          url,
          annotations: [],
          favicon: favicon ?? undefined,
        });
        return await platform.readArticle(signal, article.id);
      } catch (e) {
        setError(e);
      } finally {
        setLoading(false);
      }
    },
    [abortControllerRef, platform]
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
