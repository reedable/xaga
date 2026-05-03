import { useCallback, useState, type RefObject } from "react";
import { usePlatform } from "../platform/Platform";
import type { Entry, Feed, Filter } from "../types";

export default function useEntries(
  abortControllerRef: RefObject<AbortController>
) {
  const platform = usePlatform();
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

  const listAllEntries = useCallback(
    async (filter: Filter = "unread") => {
      const signal = abortControllerRef.current.signal;
      setError(null);
      setLoading(true);

      try {
        return await platform.listAllEntries(signal, filter);
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
      } finally {
        setLoading(false);
      }
    },
    [abortControllerRef, platform]
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
      } finally {
        setLoading(false);
      }
    },
    [abortControllerRef, platform]
  );

  return {
    loading,
    error,
    listEntries,
    listAllEntries,
    markEntryRead,
    markEntryUnread,
  };
}
