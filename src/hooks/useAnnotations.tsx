import { useCallback, useState, type RefObject } from "react";
import { usePlatform } from "../platform/Platform";
import type { Annotation, Article } from "../types";

export function useAnnotations(
  abortControllerRef: RefObject<AbortController>,
  articleId: string | null,
) {
  const platform = usePlatform();
  const [annotations, setAnnotations] = useState<Annotation[]>([]);

  /** Called after readArticle() resolves to hydrate state from disk. */
  const setAnnotationsFromArticle = useCallback((article: Article) => {
    setAnnotations(article.annotations ?? []);
  }, []);

  /** Creates or updates an annotation. The caller owns the id. createdAt is set by the backend. */
  const saveAnnotation = useCallback(
    async (annotation: Omit<Annotation, "createdAt">): Promise<void> => {
      if (!articleId) return;
      const signal = abortControllerRef.current.signal;
      const updatedArticle = await platform.addAnnotation(
        signal,
        articleId,
        annotation,
      );
      setAnnotations(updatedArticle.annotations);

      // TODO: trigger annotation embedding (annotation.text + annotation.note) for Search feature
      // once LanceDB semantic search is implemented. Each annotation should be indexed separately,
      // keyed by annotation.id, combining the highlighted text and the user note into one embedding.
    },
    [abortControllerRef, articleId, platform],
  );

  /** Removes an annotation by id. */
  const removeAnnotation = useCallback(
    async (annotationId: string): Promise<void> => {
      if (!articleId) return;
      const signal = abortControllerRef.current.signal;
      const updatedArticle = await platform.removeAnnotation(
        signal,
        articleId,
        annotationId,
      );
      setAnnotations(updatedArticle.annotations);
    },
    [abortControllerRef, articleId, platform],
  );

  return {
    annotations,
    setAnnotationsFromArticle,
    saveAnnotation,
    removeAnnotation,
  };
}
