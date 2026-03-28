import { useCallback, useEffect, useRef, type RefObject } from "react";

export default function useAbortController(): [
  () => void,
  RefObject<AbortController>,
] {
  const abortControllerRef = useRef(new AbortController());

  const abort = useCallback(() => {
    const abortController = new AbortController();
    abortControllerRef.current.abort();
    abortControllerRef.current = abortController;
  }, []);

  useEffect(() => {
    abortControllerRef.current = new AbortController();
    return () => abortControllerRef.current.abort();
  }, []);

  return [abort, abortControllerRef];
}
