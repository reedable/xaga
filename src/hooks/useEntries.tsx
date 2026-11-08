import { useCallback, useState, type RefObject } from "react";
import { useErrorSink } from "./useErrorSink";
import { usePlatform } from "../platform/Platform";
import type { Entry, Feed, Filter } from "../types";

export default function useEntries(
  abortControllerRef: RefObject<AbortController>
) {
  const platform = usePlatform();
  const { pushError } = useErrorSink();
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<unknown>(null);

  const listEntries = useCallback(
    async (feed: Feed, filter: Filter = "unread") => {
      const signal = abortControllerRef.current.signal;
      setError(null);
      setLoading(true);

      try {
        return await platform.listEntries(signal, feed.url, filter);
      } catch (e) {
        setError(e);
      } finally {
        setLoading(false);
      }
    },
    [abortControllerRef, platform]
  );

  const listAllEntities = useCallback(
    async (filter: Filter = "unread") => {
      const signal = abortControllerRef.current.signal;
      setError(null);
      setLoading(true);

      try {
        return await platform.listAllEntities(signal, filter);
      } catch (e) {
        setError(e);
      } finally {
        setLoading(false);
      }
    },
    [abortControllerRef, platform]
  );

  const markEntryRead = useCallback(
    async (entryId: Entry["id"]) => {
      const signal = abortControllerRef.current.signal;
      setError(null);
      setLoading(true);

      try {
        return await platform.markEntryRead(signal, entryId);
      } catch (e) {
        setError(e);
        pushError(e);
      } finally {
        setLoading(false);
      }
    },
    [abortControllerRef, platform, pushError]
  );

  const markEntryUnread = useCallback(
    async (entryId: Entry["id"]) => {
      const signal = abortControllerRef.current.signal;
      setError(null);
      setLoading(true);

      try {
        return await platform.markEntryUnread(signal, entryId);
      } catch (e) {
        setError(e);
        pushError(e);
      } finally {
        setLoading(false);
      }
    },
    [abortControllerRef, platform, pushError]
  );

  return {
    loading,
    error,
    listEntries,
    listAllEntities,
    markEntryRead,
    markEntryUnread,
  };
}
