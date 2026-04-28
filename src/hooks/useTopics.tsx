import { useCallback, useState, type RefObject } from "react";
import { usePlatform } from "../platform/Platform";
import type { Topic } from "../types";

export function useTopics(abortControllerRef: RefObject<AbortController>) {
  const platform = usePlatform();
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<unknown>(null);

  const createTopic = useCallback(
    async (
      topic: Omit<Topic, "id" | "createdAt" | "unreadCount" | "entryCount">,
    ): Promise<Topic | undefined> => {
      const signal = abortControllerRef.current.signal;
      setLoading(true);
      setError(null);

      try {
        return await platform.createTopic(signal, topic);
      } catch (e) {
        setError(e);
      } finally {
        setLoading(false);
      }
    },
    [abortControllerRef, platform],
  );

  const saveTopic = useCallback(
    async (topic: Topic): Promise<Topic | undefined> => {
      const signal = abortControllerRef.current.signal;
      setLoading(true);
      setError(null);

      try {
        return await platform.saveTopic(signal, topic);
      } catch (e) {
        setError(e);
      } finally {
        setLoading(false);
      }
    },
    [abortControllerRef, platform],
  );

  const deleteTopic = useCallback(
    async (id: string): Promise<void> => {
      const signal = abortControllerRef.current.signal;
      setLoading(true);
      setError(null);

      try {
        await platform.deleteTopic(signal, id);
      } catch (e) {
        setError(e);
      } finally {
        setLoading(false);
      }
    },
    [abortControllerRef, platform],
  );

  return { error, loading, createTopic, deleteTopic, saveTopic };
}
