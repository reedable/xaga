import { useCallback, useState, type RefObject } from "react";
import { usePlatform } from "../platform/Platform";
import type { AnnotationSummary } from "../types";

export function useAnnotationsList(
  abortControllerRef: RefObject<AbortController>,
) {
  const platform = usePlatform();
  const [annotations, setAnnotations] = useState<AnnotationSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<unknown>(null);

  const load = useCallback(async () => {
    const signal = abortControllerRef.current.signal;
    setError(null);
    setLoading(true);
    try {
      const result = await platform.listAnnotations(signal);
      setAnnotations(result);
    } catch (e) {
      setError(e);
    } finally {
      setLoading(false);
    }
  }, [abortControllerRef, platform]);

  return { annotations, loading, error, load };
}
