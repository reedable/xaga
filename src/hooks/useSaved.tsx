import { useCallback, useState, type RefObject } from "react";
import { usePlatform } from "../platform/Platform";
import type { SavedItem } from "../types";

export function useSaved(abortControllerRef: RefObject<AbortController>) {
  const platform = usePlatform();
  const [saved, setSaved] = useState<SavedItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<unknown>(null);

  const load = useCallback(async () => {
    const signal = abortControllerRef.current.signal;
    setError(null);
    setLoading(true);
    try {
      const result = await platform.listSaved(signal);
      setSaved(result);
    } catch (e) {
      setError(e);
    } finally {
      setLoading(false);
    }
  }, [abortControllerRef, platform]);

  return { saved, loading, error, load };
}
