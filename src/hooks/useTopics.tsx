import { useCallback, useContext, type RefObject } from "react";
import { usePlatform } from "../platform/Platform";
import TopicsContext from "../providers/TopicsContext";
import type { Annotation, Entry, Filter, Topic, TopicSort } from "../types";
import sortByUnreadCount from "../utils/sortByUnreadCount";
import { useSidebar } from "./useSidebar";

export function useTopics(abortControllerRef: RefObject<AbortController>) {
  const ctx = useContext(TopicsContext);
  if (!ctx) throw new Error("useTopics must be used inside TopicsProvider");

  const { topics, pinnedTopicIds, setTopics, setPinnedTopicIds } = ctx;

  const platform = usePlatform();
  const { listTopics } = useSidebar(abortControllerRef);

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

  const listTopicItems = useCallback(
    async (id: string, filter: Filter, sort: TopicSort) => {
      const signal = abortControllerRef.current.signal;
      try {
        return await platform.listTopicItems(signal, id, filter, sort);
      } catch {
        return undefined;
      }
    },
    [abortControllerRef, platform]
  );

  const addTopicItem = useCallback(
    async (topicId: string, item: Entry | Annotation) => {
      const signal = abortControllerRef.current.signal;
      try {
        return await platform.addTopicItem(signal, topicId, item);
      } catch {
        // surfaced via dialog
      }
    },
    [abortControllerRef, platform]
  );

  const removeTopicItem = useCallback(
    async (topicId: string, itemId: string) => {
      const signal = abortControllerRef.current.signal;
      try {
        return await platform.removeTopicItem(signal, topicId, itemId);
      } catch {
        // surfaced via dialog
      }
    },
    [abortControllerRef, platform]
  );

  const listEntryTopics = useCallback(
    async (entryId: string) => {
      const signal = abortControllerRef.current.signal;
      try {
        return await platform.listEntryTopics(signal, entryId);
      } catch {
        return undefined;
      }
    },
    [abortControllerRef, platform]
  );

  const rankTopicsRelatedToItem = useCallback(
    async (itemId: string, minSimilarity?: number) => {
      const signal = abortControllerRef.current.signal;
      try {
        return await platform.rankTopicsRelatedToItem(
          signal,
          itemId,
          minSimilarity
        );
      } catch {
        return undefined;
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
    listTopicItems,
    addTopicItem,
    removeTopicItem,
    listEntryTopics,
    rankTopicsRelatedToItem,
  };
}
