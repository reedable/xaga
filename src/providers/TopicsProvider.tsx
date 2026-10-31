import { usePins } from "./PinsContext";
import {
  useCallback,
  useEffect,
  useMemo,
  useState,
  useRef,
  type ReactNode,
  type SetStateAction,
} from "react";
import { useErrorSink } from "../hooks/useErrorSink";
import { subscribe } from "../hooks/useEvents";
import { usePlatform } from "../platform/Platform";
import type { Topic } from "../types";
import { debounced } from "../utils/Decorator";
import sortByUnreadCount from "../utils/sortByUnreadCount";
import TopicsContext, { type TopicsContextType } from "./TopicsContext";

export function TopicsProvider({ children }: { children: ReactNode }) {
  const platform = usePlatform();
  const { pushError } = useErrorSink();
  const [rawTopics, setRawTopics] = useState<Topic[] | undefined>();
  const setTopics = useCallback((update: SetStateAction<Topic[]>) => {
    setRawTopics((previous) =>
      typeof update === "function" ? update(previous ?? []) : update
    );
  }, []);
  // File-committed membership intent wins over the asynchronous reverse index.
  // This projection lasts for the library session, including picker remounts.
  const membershipChanges = useRef(new Map<string, Map<string, boolean>>());
  const setMembership = useCallback(
    (topicId: string, itemId: string, present: boolean) => {
      const changes =
        membershipChanges.current.get(itemId) ?? new Map<string, boolean>();
      changes.set(topicId, present);
      membershipChanges.current.set(itemId, changes);
    },
    []
  );
  const getMemberships = useCallback((itemId: string, indexedIds: string[]) => {
    const ids = new Set(indexedIds);
    for (const [topicId, present] of membershipChanges.current.get(itemId) ??
      []) {
      if (present) ids.add(topicId);
      else ids.delete(topicId);
    }
    return [...ids];
  }, []);
  const { pins, setPin } = usePins();
  const topics = useMemo(
    () =>
      (rawTopics ?? []).map((value) => ({
        ...value,
        isPinned: pins.some(
          (pin) => pin.type === "topic" && pin.id === value.id
        ),
      })),
    [rawTopics, pins]
  );

  const pinnedTopicIds = useMemo(
    () => pins.flatMap((pin) => (pin.type === "topic" ? [pin.id] : [])),
    [pins]
  );
  const setTopicPinned = useCallback(
    (id: string, pinned: boolean) => {
      void setPin({ type: "topic", id }, pinned).catch(pushError);
    },
    [setPin, pushError]
  );

  const refreshTopics = useCallback(async () => {
    const results = await platform.listTopics(new AbortController().signal);
    if (results) setTopics([...results].sort(sortByUnreadCount));
  }, [platform, setTopics]);

  useEffect(() => {
    const refresh = debounced(refreshTopics, { wait: 1000 });
    return subscribe(() => platform.onUnreadChanged(refresh));
  }, [platform, refreshTopics]);

  useEffect(() => {
    const refresh = debounced(refreshTopics, { wait: 1000 });
    return subscribe(() => platform.onEmbeddingsChanged(refresh));
  }, [platform, refreshTopics]);

  const value = useMemo<TopicsContextType>(
    () => ({
      topics,
      hasLoaded: rawTopics !== undefined,
      pinnedTopicIds,
      setTopics,
      setTopicPinned,
      setMembership,
      getMemberships,
    }),
    [
      topics,
      rawTopics,
      pinnedTopicIds,
      setTopics,
      setTopicPinned,
      setMembership,
      getMemberships,
    ]
  );

  return (
    <TopicsContext.Provider value={value}>{children}</TopicsContext.Provider>
  );
}
