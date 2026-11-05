import { useCallback, useContext, useMemo } from "react";
import FeedsContext from "../providers/FeedsContext";
import TopicsContext from "../providers/TopicsContext";
import SitesContext from "../providers/SitesContext";
import { usePinnedEntries } from "./usePinnedEntries";
import { useMissingPins } from "./useMissingPins";

/** The same resolved targets serve sidebar and native menu presentation. */
export function useShortcutTargets() {
  const entries = usePinnedEntries();
  const feeds = useContext(FeedsContext);
  const topics = useContext(TopicsContext);
  const sites = useContext(SitesContext);
  if (!feeds || !topics || !sites)
    throw new Error("Shortcut targets require collection providers");
  const feedItems = feeds.feeds;
  const topicItems = topics.topics;
  const missing = useMissingPins(entries, feedItems, topicItems);
  const targets = useMemo(
    () => [
      ...(sites.pinnedSites ?? []),
      // Article shortcuts show no counts and contribute zero to unread sorting.
      ...entries.map((entry) => ({ ...entry, entryCount: 0, unreadCount: 0 })),
      ...topics.pinnedTopicIds.flatMap((id) =>
        topicItems.filter((topic) => topic.id === id)
      ),
      ...feeds.pinnedFeedIds.flatMap((id) =>
        feedItems.filter((feed) => feed.id === id)
      ),
    ],
    [
      entries,
      feedItems,
      feeds.pinnedFeedIds,
      topicItems,
      topics.pinnedTopicIds,
      sites.pinnedSites,
    ]
  );
  const pending = useCallback(
    (target: (typeof missing)[number]) =>
      target.type === "feed"
        ? feeds.hasLoaded === false
        : target.type === "topic"
          ? topics.hasLoaded === false
          : target.type === "site" && sites.hasLoaded === false,
    [feeds.hasLoaded, topics.hasLoaded, sites.hasLoaded]
  );
  return { targets, missing, pending };
}
