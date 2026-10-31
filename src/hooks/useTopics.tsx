import { useCallback, useContext, type RefObject } from "react";
import { usePlatform } from "../platform/Platform";
import type { CreateTopicInput } from "../platform/Platform";
import TopicsContext from "../providers/TopicsContext";
import type { Annotation, Entry, Filter, Topic, TopicSort } from "../types";
import sortByUnreadCount from "../utils/sortByUnreadCount";
import { emitTopicItemAdded } from "../events/topicMembership";
import { useErrorSink } from "./useErrorSink";
import { useSidebar } from "./useSidebar";

export function useTopics(abortControllerRef: RefObject<AbortController>) {
  const ctx = useContext(TopicsContext);
  if (!ctx) throw new Error("useTopics must be used inside TopicsProvider");

  const {
    topics,
    pinnedTopicIds,
    setTopics,
    setTopicPinned,
    setMembership,
    getMemberships,
  } = ctx;

  const platform = usePlatform();
  const { pushError } = useErrorSink();
  const { listTopics } = useSidebar(abortControllerRef);

  const createTopic = useCallback(
    async (topic: CreateTopicInput) => {
      const signal = abortControllerRef.current.signal;
      try {
        const created = await platform.createTopic(signal, topic);
        setTopics((prev) => [...prev, created].sort(sortByUnreadCount));
        return created;
      } catch (e) {
        pushError(e);
      }
    },
    [abortControllerRef, platform, pushError, setTopics]
  );

  const createTopicWithMember = useCallback(
    async (topicInput: CreateTopicInput, member: Entry | Annotation) => {
      const signal = abortControllerRef.current.signal;
      try {
        const topic = await platform.createTopicWithMember(
          signal,
          topicInput,
          member
        );
        setMembership(topic.id, member.id, true);
        setTopics((prev) => [...prev, topic].sort(sortByUnreadCount));
        emitTopicItemAdded({ topicId: topic.id, itemId: member.id });
        return topic;
      } catch (e) {
        pushError(e);
      }
    },
    [abortControllerRef, platform, pushError, setTopics, setMembership]
  );

  const saveTopic = useCallback(
    async (topic: Topic) => {
      const signal = abortControllerRef.current.signal;
      try {
        return await platform.saveTopic(signal, topic);
      } catch (e) {
        pushError(e);
      }
    },
    [abortControllerRef, platform, pushError]
  );

  const deleteTopic = useCallback(
    async (id: string) => {
      const signal = abortControllerRef.current.signal;
      try {
        await platform.deleteTopic(signal, id);
      } catch (e) {
        pushError(e);
      }
    },
    [abortControllerRef, platform, pushError]
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
        const topic = await platform.addTopicItem(signal, topicId, item);
        setMembership(topicId, item.id, true);
        setTopics((prev) =>
          prev
            .map((value) => (value.id === topic.id ? topic : value))
            .sort(sortByUnreadCount)
        );
        emitTopicItemAdded({ topicId, itemId: item.id });
        return topic;
      } catch (e) {
        pushError(e);
      }
    },
    [abortControllerRef, platform, pushError, setTopics, setMembership]
  );

  const removeTopicItem = useCallback(
    async (topicId: string, itemId: string) => {
      const signal = abortControllerRef.current.signal;
      try {
        const topic = await platform.removeTopicItem(signal, topicId, itemId);
        setMembership(topicId, itemId, false);
        setTopics((prev) =>
          prev
            .map((value) => (value.id === topic.id ? topic : value))
            .sort(sortByUnreadCount)
        );
        // Keep an open Topic list stable; removal is reflected on its next natural reload.
        return topic;
      } catch (e) {
        pushError(e);
      }
    },
    [abortControllerRef, platform, pushError, setTopics, setMembership]
  );

  const listEntryTopics = useCallback(
    async (entryId: string) => {
      const signal = abortControllerRef.current.signal;
      try {
        return getMemberships(
          entryId,
          await platform.listEntryTopics(signal, entryId)
        );
      } catch {
        return undefined;
      }
    },
    [abortControllerRef, platform, getMemberships]
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
    setTopicPinned,
    requestTopics,
    createTopic,
    createTopicWithMember,
    saveTopic,
    deleteTopic,
    listTopicItems,
    addTopicItem,
    removeTopicItem,
    listEntryTopics,
    rankTopicsRelatedToItem,
  };
}
