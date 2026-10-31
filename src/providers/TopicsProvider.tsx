import { usePins } from "./PinsContext";
import {
  useCallback,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
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
  const [rawTopics, setTopics] = useState<Topic[]>([]);
  const { pins, setPin } = usePins();
  const topics = useMemo(
    () =>
      rawTopics.map((value) => ({
        ...value,
        isPinned: pins.some(
          (pin) => pin.type === "topic" && pin.id === value.id
        ),
      })),
    [rawTopics, pins]
  );

  const pinnedTopicIds = useMemo(
    () => pins.filter((pin) => pin.type === "topic").map((pin) => pin.id),
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
  }, [platform]);

  useEffect(() => {
    const refresh = debounced(refreshTopics, { wait: 1000 });
    return subscribe(() => platform.onUnreadChanged(refresh));
  }, [platform, refreshTopics]);

  useEffect(() => {
    const refresh = debounced(refreshTopics, { wait: 1000 });
    return subscribe(() => platform.onEmbeddingsChanged(refresh));
  }, [platform, refreshTopics]);

  const value = useMemo<TopicsContextType>(
    () => ({ topics, pinnedTopicIds, setTopics, setTopicPinned }),
    [topics, pinnedTopicIds, setTopicPinned]
  );

  return (
    <TopicsContext.Provider value={value}>{children}</TopicsContext.Provider>
  );
}
