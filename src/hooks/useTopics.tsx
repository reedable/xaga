import { useCallback, useContext, type RefObject } from "react";
import { usePlatform } from "../platform/Platform";
import type { CreateTopicInput } from "../platform/Platform";
import TopicsContext from "../providers/TopicsContext";
import type { Annotation, Entry, Filter, Topic, TopicSort } from "../types";
import sortByUnreadCount from "../utils/sortByUnreadCount";
import { emitTopicEntityAdded } from "../events/topicMembership";
import { useErrorSink } from "./useErrorSink";
import { useSidebar } from "./useSidebar";

export function useTopics(abortControllerRef: RefObject<AbortController>) {
  const ctx = useContext(TopicsContext);
  if (!ctx) throw new Error("useTopics must be used inside TopicsProvider");

  const {
    topics,
    hasLoaded = false,
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
        emitTopicEntityAdded({ topicId: topic.id, entityId: member.id });
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

  const listTopicEntities = useCallback(
    async (id: string, filter: Filter, sort: TopicSort) => {
      const signal = abortControllerRef.current.signal;
      try {
        return await platform.listTopicEntities(signal, id, filter, sort);
      } catch {
        return undefined;
      }
    },
    [abortControllerRef, platform]
  );

  const addTopicEntity = useCallback(
    async (topicId: string, entity: Entry | Annotation) => {
      const signal = abortControllerRef.current.signal;
      try {
        const topic = await platform.addTopicEntity(signal, topicId, entity);
        setMembership(topicId, entity.id, true);
        setTopics((prev) =>
          prev
            .map((value) => (value.id === topic.id ? topic : value))
            .sort(sortByUnreadCount)
        );
        emitTopicEntityAdded({ topicId, entityId: entity.id });
        return topic;
      } catch (e) {
        pushError(e);
      }
    },
    [abortControllerRef, platform, pushError, setTopics, setMembership]
  );

  const removeTopicEntity = useCallback(
    async (topicId: string, entityId: string) => {
      const signal = abortControllerRef.current.signal;
      try {
        const topic = await platform.removeTopicEntity(signal, topicId, entityId);
        setMembership(topicId, entityId, false);
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

  const rankTopicsRelatedToEntity = useCallback(
    async (entityId: string, minSimilarity?: number) => {
      const signal = abortControllerRef.current.signal;
      try {
        return await platform.rankTopicsRelatedToEntity(
          signal,
          entityId,
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
    hasLoaded,
    pinnedTopicIds,
    setTopicPinned,
    requestTopics,
    createTopic,
    createTopicWithMember,
    saveTopic,
    deleteTopic,
    listTopicEntities,
    addTopicEntity,
    removeTopicEntity,
    listEntryTopics,
    rankTopicsRelatedToEntity,
  };
}
