import { Refresh } from "@mui/icons-material";
import { IconButton, Tooltip } from "@mui/material";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
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

type SidebarProps = {
  className?: string;
  onAddFeed: () => void;
  onAddTopic: () => void;
  onAddCollection: () => void;
};

type ListType = "SHORTCUTS" | "COLLECTIONS" | "TOPICS" | "FEEDS";

export function Sidebar({
  className,
  onAddFeed,
  onAddTopic,
  onAddCollection,
}: SidebarProps) {
  const [, abortControllerRef] = useAbortController();

  const { topics, pinnedTopicIds } = useTopics(abortControllerRef);

  const { loading, feeds, pinnedFeedIds, refreshAllFeeds } =
    useFeeds(abortControllerRef);

  const { collections, pinnedCollectionIds } =
    useCollections(abortControllerRef);

  const pinnedCollections = useMemo(
    () =>
      [
        ...pinnedTopicIds.flatMap((id) => topics.filter((t) => t.id === id)),
        ...pinnedFeedIds.flatMap((id) => feeds.filter((f) => f.id === id)),
        ...pinnedCollectionIds.flatMap((id) =>
          collections.filter((c) => c.id === id)
        ),
      ].sort(sortByUnreadCount),
    [
      feeds,
      pinnedFeedIds,
      pinnedTopicIds,
      topics,
      collections,
      pinnedCollectionIds,
    ]
  );

  const [expandedList, setExpandedList] = useLocalStorage<ListType | null>(
    "sidebar.expander",
    "SHORTCUTS"
  );

  const handleExpandedList = useCallback(
    (listType: ListType) => {
      setExpandedList((prev) => (prev === listType ? null : listType));
    },
    [setExpandedList]
  );

  const stopRequestedRef = useRef(false);
  const iidRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const [spin, setSpin] = useState(false);

  const handleRefreshAllFeeds = () => {
    refreshAllFeeds();
    setSpin(true);

    iidRef.current = setInterval(() => {
      console.log(">>>", Date.now(), stopRequestedRef.current);
      if (stopRequestedRef.current) {
        setSpin(false);
        clearInterval(iidRef.current);
      }
    }, 1000);
  };

  useEffect(() => {
    return () => clearTimeout(iidRef.current);
  }, []);

  useEffect(() => {
    if (!loading) {
      stopRequestedRef.current = true;
    }
  }, [loading]);

  return (
    <div className={cx(className, css.Sidebar)}>
      <Tooltip title="Refresh">
        <IconButton
          className={cx(css.RefreshButton, spin && css.Loading)}
          size="small"
          onClick={() => handleRefreshAllFeeds()}
        >
          <Refresh fontSize="small" />
        </IconButton>
      </Tooltip>

      <ShortcutList
        shortcuts={pinnedCollections}
        onClickHeader={() => handleExpandedList("SHORTCUTS")}
        isExpanded={expandedList === "SHORTCUTS"}
      />

      <TopicList
        topics={topics}
        onAddTopic={onAddTopic}
        onClickHeader={() => handleExpandedList("TOPICS")}
        isExpanded={expandedList === "TOPICS"}
      />

      <CollectionList
        collections={collections}
        onAddCollection={onAddCollection}
        onClickHeader={() => handleExpandedList("COLLECTIONS")}
        isExpanded={expandedList === "COLLECTIONS"}
      />

      <FeedList
        feeds={feeds}
        onAddFeed={onAddFeed}
        onClickHeader={() => handleExpandedList("FEEDS")}
        isExpanded={expandedList === "FEEDS"}
      />
    </div>
  );
}
