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
  const [topics, setTopics] = useState<Topic[]>([]);

  // Pinned state lives in each topic's `.md` tags (surfaced as `isPinned`); derive the id-set.
  const pinnedTopicIds = useMemo(
    () => topics.filter((t) => t.isPinned).map((t) => t.id),
    [topics]
  );

  // Optimistically flip the topic's `isPinned`, then persist to the `.md` (fire-and-forget).
  const setTopicPinned = useCallback(
    (id: string, pinned: boolean) => {
      setTopics((prev) =>
        prev.map((t) => (t.id === id ? { ...t, isPinned: pinned } : t))
      );
      // Optimistic; a refused persist (e.g. read-only library) surfaces to the
      // user rather than being swallowed to the console.
      platform.setTopicPinned(id, pinned).catch(pushError);
    },
    [platform, pushError]
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
