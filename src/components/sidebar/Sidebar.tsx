import { useMemo } from "react";
import useAbortController from "../../hooks/useAbortController";
import { useCollections } from "../../hooks/useCollections";
import { useFeeds } from "../../hooks/useFeeds";
import { useLocalStorage } from "../../hooks/useLocalStorage";
import { useTopics } from "../../hooks/useTopics";
import cx from "../../utils/cx";
import sortByUnreadCount from "../../utils/sortByUnreadCount";
import CollectionList from "./CollectionList";
import FeedList from "./FeedList";
import ShortcutList from "./ShortcutList";
import css from "./Sidebar.module.css";
import TopicList from "./TopicList";

export const SIDEBAR_WIDTH = 240;

type SidebarProps = {
  className?: string;
  onAddFeed: () => void;
  onAddTopic: () => void;
  onAddCollection: () => void;
};

export type ListType = "SHORTCUTS" | "COLLECTIONS" | "TOPICS" | "FEEDS";

export function Sidebar({
  className,
  onAddFeed,
  onAddTopic,
  onAddCollection,
}: SidebarProps) {
  const [, abortControllerRef] = useAbortController();
  const { topics, pinnedTopicIds } = useTopics(abortControllerRef);
  const { feeds, pinnedFeedIds } = useFeeds(abortControllerRef);
  const { collections, pinnedCollectionIds } = useCollections(abortControllerRef);

  const pinnedCollections = useMemo(
    () => [
      ...pinnedTopicIds.flatMap((id) => topics.filter((t) => t.id === id)),
      ...pinnedFeedIds.flatMap((id) => feeds.filter((f) => f.id === id)),
      ...pinnedCollectionIds.flatMap((id) => collections.filter((c) => c.id === id)),
    ].sort(sortByUnreadCount),
    [feeds, pinnedFeedIds, pinnedTopicIds, topics, collections, pinnedCollectionIds]
  );

  const [expandedList, setExpandedList] = useLocalStorage<ListType | null>(
    "sidebar.expander",
    "SHORTCUTS"
  );

  return (
    <div className={cx(className, css.Sidebar)}>
      <ShortcutList
        shortcuts={pinnedCollections}
        onClickHeader={() => setExpandedList("SHORTCUTS")}
        isExpanded={expandedList === "SHORTCUTS"}
      />

      <CollectionList
        collections={collections}
        onAddCollection={onAddCollection}
        onClickHeader={() => setExpandedList("COLLECTIONS")}
        isExpanded={expandedList === "COLLECTIONS"}
      />

      <TopicList
        topics={topics}
        onAddTopic={onAddTopic}
        onClickHeader={() => setExpandedList("TOPICS")}
        isExpanded={expandedList === "TOPICS"}
      />

      <FeedList
        feeds={feeds}
        onAddFeed={onAddFeed}
        onClickHeader={() => setExpandedList("FEEDS")}
        isExpanded={expandedList === "FEEDS"}
      />
    </div>
  );
}
