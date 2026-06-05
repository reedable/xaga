import { useCallback, useRef, useState } from "react";
import { SEARCH_THRESHOLD } from "../consts";
import { usePlatform } from "../platform/Platform";
import { isAbortError, type Embedded, type Item } from "../types";

export default function useSearch() {
  const platform = usePlatform();
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<unknown>(null);
  const abortRef = useRef(new AbortController());

  const search = useCallback(
    async (text: string): Promise<(Embedded & Item)[] | undefined> => {
      abortRef.current.abort();
      abortRef.current = new AbortController();
      const signal = abortRef.current.signal;

      setError(null);
      setLoading(true);

      try {
        return await platform.queryByText(
          signal,
          text,
          "all",
          SEARCH_THRESHOLD,
          null,
          100
        );
      } catch (e) {
        if (isAbortError(e)) return;
        setError(e);
      } finally {
        setLoading(false);
      }
    },
    [platform]
  );

  return { loading, error, search };
}
