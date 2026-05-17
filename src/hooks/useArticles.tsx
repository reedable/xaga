import { useCallback, useState, type RefObject } from "react";
import { useAppearance } from "./useAppearance";
import { usePlatform } from "../platform/Platform";
import { processEntry } from "../text/entryWorker";
import type { Entry } from "../types";

export function useArticles(abortControllerRef: RefObject<AbortController>) {
  const platform = usePlatform();
  const { showOriginalSite: keepOpen } = useAppearance();
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<unknown>(null);

  /**
   * Loads and saves a remote content identified by URL.
   *
   * WARNING: Do not use this method if you already have an Entry or Article
   * object. To refresh an existing article, call `loadArticle` instead.
   */
  const loadArticleByUrl = useCallback(
    async (url: string) => {
      const signal = abortControllerRef.current.signal;
      setError(null);
      setLoading(true);

      try {
        const html = await platform.loadArticle(signal, url, keepOpen);
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
    [abortControllerRef, keepOpen, platform]
  );

  /**
   * Loads the remote content over the network, and saves the fresh content to
   * the disc.
   */
  const loadArticle = useCallback(
    async (entry: Entry) => {
      const signal = abortControllerRef.current.signal;
      setError(null);
      setLoading(true);

      try {
        const article = await platform.readArticle(signal, entry.id);
        const html = await platform.loadArticle(signal, entry.url, keepOpen);
        const { title, markdown, favicon } = await processEntry(article.url, html);

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
    [abortControllerRef, keepOpen, platform]
  );

  /**
   * Reads the locally saved content and returns if found. Otherwise, it loads
   * the remote content and returns the fresh content. Call `loadArticle`
   * instead if you need the fresh content every time.
   *
   * WARNING: `loadArticle` implicitly calls `save_article`, which moves the
   * md file from unread/ to read/ directory, i.e. marks the item read.
   */
  const readArticle = useCallback(
    async (entryId: Entry["id"]) => {
      const signal = abortControllerRef.current.signal;
      setError(null);
      setLoading(true);

      try {
        const article = await platform.readArticle(signal, entryId);

        if (article.content && !keepOpen) {
          return article;
        }

        return await loadArticle(article);
      } catch (e) {
        setError(e);
      } finally {
        setLoading(false);
      }
    },
    [abortControllerRef, loadArticle, platform]
  );

  const updateArticleFromHtml = useCallback(
    async (entry: Entry, html: string) => {
      const signal = abortControllerRef.current.signal;
      setError(null);
      setLoading(true);

      try {
        const article = await platform.readArticle(signal, entry.id);
        const { title, markdown, favicon } = await processEntry(article.url, html);

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

  return {
    error,
    loading,
    readArticle,
    loadArticle,
    loadArticleByUrl,
    findArticleByUrl,
    updateArticleFromHtml,
  };
}
