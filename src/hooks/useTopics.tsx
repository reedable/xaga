import { useCallback, useContext, type RefObject } from "react";
import { usePlatform } from "../platform/Platform";
import TopicsContext from "../providers/TopicsContext";
import type { Topic } from "../types";
import sortByUnreadCount from "../utils/sortByUnreadCount";
import { useCollections } from "./useCollections";

export function useTopics(abortControllerRef: RefObject<AbortController>) {
  const ctx = useContext(TopicsContext);
  if (!ctx) throw new Error("useTopics must be used inside TopicsProvider");

  const { topics, pinnedTopicIds, setTopics, setPinnedTopicIds } = ctx;

  const platform = usePlatform();
  const { listTopics } = useCollections(abortControllerRef);

  const createTopic = useCallback(
    async (
      topic: Omit<Topic, "id" | "createdAt" | "unreadCount" | "entryCount">
    ) => {
      const signal = abortControllerRef.current.signal;
      try {
        return await platform.createTopic(signal, topic);
      } catch {
        // surfaced via dialog
      }
    },
    [abortControllerRef, platform]
  );

  const saveTopic = useCallback(
    async (topic: Topic) => {
      const signal = abortControllerRef.current.signal;
      try {
        return await platform.saveTopic(signal, topic);
      } catch {
        // surfaced via dialog
      }
    },
    [abortControllerRef, platform]
  );

  const deleteTopic = useCallback(
    async (id: string) => {
      const signal = abortControllerRef.current.signal;
      try {
        await platform.deleteTopic(signal, id);
      } catch {
        // surfaced via dialog
      }
    },
    [abortControllerRef, platform]
  );

  const requestTopics = useCallback(async () => {
    const results = await listTopics(sortByUnreadCount);
    if (results) setTopics(results);
  }, [listTopics, setTopics]);

  return {
    topics,
    pinnedTopicIds,
    setPinnedTopicIds,
    requestTopics,
    createTopic,
    saveTopic,
    deleteTopic,
  };
}
