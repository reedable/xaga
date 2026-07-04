import { useCallback, useState, type RefObject } from "react";
import { usePlatform } from "../platform/Platform";
import type { Annotation, Article } from "../types";

export function useAnnotations(abortControllerRef: RefObject<AbortController>) {
  const platform = usePlatform();
  const [annotations, setAnnotations] = useState<Annotation[]>([]);

  /** Called after readArticle() resolves to hydrate state from disk. */
  const setAnnotationsFromArticle = useCallback((article: Article) => {
    setAnnotations(article.annotations ?? []);
  }, []);

  /** Creates or updates an annotation. The caller owns the id. createdAt is set by the backend. */
  const saveAnnotation = useCallback(
    async (
      annotation: Omit<Annotation, "createdAt" | "updatedAt" | "__kind">
    ): Promise<void> => {
      const signal = abortControllerRef.current.signal;
      const updatedArticle = await platform.addAnnotation(signal, annotation);
      setAnnotations(updatedArticle.annotations);
    },
    [abortControllerRef, platform]
  );

  /** Replaces an annotation under a new id — the color-change path (delete/create).
   * The caller owns both ids; `annotation.id` is the new one. */
  const replaceAnnotation = useCallback(
    async (
      oldId: string,
      annotation: Omit<Annotation, "createdAt" | "updatedAt" | "__kind">
    ): Promise<void> => {
      const signal = abortControllerRef.current.signal;
      const updatedArticle = await platform.replaceAnnotation(
        signal,
        oldId,
        annotation
      );
      setAnnotations(updatedArticle.annotations);
    },
    [abortControllerRef, platform]
  );

  /** Removes an annotation by id. */
  const removeAnnotation = useCallback(
    async (annotation: Pick<Annotation, "id" | "reference">): Promise<void> => {
      const signal = abortControllerRef.current.signal;
      const updatedArticle = await platform.removeAnnotation(
        signal,
        annotation
      );
      setAnnotations(updatedArticle.annotations);
    },
    [abortControllerRef, platform]
  );

  return {
    annotations,
    setAnnotationsFromArticle,
    saveAnnotation,
    replaceAnnotation,
    removeAnnotation,
  };
}
