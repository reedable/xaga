import { useCallback, useEffect, useMemo, useState, type ReactNode } from "react";
import { usePreference } from "../hooks/usePreference";
import { usePlatform } from "../platform/Platform";
import type { Topic } from "../types";
import { debounced } from "../utils/Decorator";
import sortByUnreadCount from "../utils/sortByUnreadCount";
import { subscribe } from "../hooks/useEvents";
import TopicsContext, { type TopicsContextType } from "./TopicsContext";

export function TopicsProvider({ children }: { children: ReactNode }) {
  const platform = usePlatform();
  const [topics, setTopics] = useState<Topic[]>([]);
  const [pinnedTopicIds, setPinnedTopicIds] = usePreference<string[]>(
    "sidebar.pinnedTopics",
    [],
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
    () => ({ topics, pinnedTopicIds, setTopics, setPinnedTopicIds }),
    [topics, pinnedTopicIds, setPinnedTopicIds],
  );

  return (
    <TopicsContext.Provider value={value}>
      {children}
    </TopicsContext.Provider>
  );
}
