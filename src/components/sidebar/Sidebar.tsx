import { useMemo } from "react";
import useAbortController from "../../hooks/useAbortController";
import { useFeeds } from "../../hooks/useFeeds";
import { useLocalStorage } from "../../hooks/useLocalStorage";
import { useTopics } from "../../hooks/useTopics";
import { type Feed, type Topic } from "../../types";
import cx from "../../utils/cx";
import sortByUnreadCount from "../../utils/sortByUnreadCount";
import css from "./_.module.css";
import FeedList from "./FeedList";
import ShortcutList from "./ShortcutList";
import TopicList from "./TopicList";

export const SIDEBAR_WIDTH = 240;

type SidebarProps = {
  className?: string;
  onAddFeed: () => void;
  onAddTopic: () => void;
};

export type ListType = "SHORTCUTS" | "TOPICS" | "FEEDS";

export function Sidebar({ className, onAddFeed, onAddTopic }: SidebarProps) {
  const [, abortControllerRef] = useAbortController();
  const { topics, pinnedTopicIds } = useTopics(abortControllerRef);
  const { feeds, pinnedFeedIds } = useFeeds(abortControllerRef);

  const pinnedCollections = useMemo(
    () =>
      [...pinnedTopicIds, ...pinnedFeedIds]
        .map((id) => [...topics, ...feeds].find((c) => c.id === id))
        .filter((c): c is Topic | Feed => c !== undefined)
        .sort(sortByUnreadCount),
    [feeds, pinnedFeedIds, pinnedTopicIds, topics]
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
