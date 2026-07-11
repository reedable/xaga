import { Refresh } from "@mui/icons-material";
import { IconButton, Tooltip } from "@mui/material";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import useAbortController from "../../hooks/useAbortController";
import { useFeeds } from "../../hooks/useFeeds";
import { useLibraryReadOnly } from "../../hooks/useLibraryReadOnly";
import { useSidebar } from "../../hooks/useSidebar";
import { useTopics } from "../../hooks/useTopics";
import cx from "../../utils/cx";
import sortByUnreadCount from "../../utils/sortByUnreadCount";
import FeedList from "./FeedList";
import ShortcutList from "./ShortcutList";
import css from "./Sidebar.module.css";
import TopicList from "./TopicList";

type SidebarProps = {
  className?: string;
  onAddFeed: () => void;
  onAddTopic: () => void;
};

export function Sidebar({ className, onAddFeed, onAddTopic }: SidebarProps) {
  const [, abortControllerRef] = useAbortController();
  const isLibraryReadOnly = useLibraryReadOnly();

  const { topics, pinnedTopicIds } = useTopics(abortControllerRef);

  const { loading, feeds, pinnedFeedIds, refreshAllFeeds } =
    useFeeds(abortControllerRef);

  const { expandedList, setExpandedList } = useSidebar(abortControllerRef);

  const pinnedCollections = useMemo(
    () =>
      [
        ...pinnedTopicIds.flatMap((id) => topics.filter((t) => t.id === id)),
        ...pinnedFeedIds.flatMap((id) => feeds.filter((f) => f.id === id)),
      ].sort(sortByUnreadCount),
    [feeds, pinnedFeedIds, pinnedTopicIds, topics]
  );

  const handleExpandedList = useCallback(
    (listType: typeof expandedList) => {
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
      <div className={css.Buttons}>
        {isLibraryReadOnly && (
          <Tooltip title="Another xaga instance holds this library — this window is read-only">
            <span className={css.ReadOnlyBadge}>Read-only</span>
          </Tooltip>
        )}
        <Tooltip title="Refresh">
          {/* span keeps the Tooltip working when the button is disabled */}
          <span className={css.RefreshWrap}>
            <IconButton
              className={cx(css.RefreshButton, spin && css.Loading)}
              size="small"
              disabled={isLibraryReadOnly}
              onClick={() => handleRefreshAllFeeds()}
            >
              <Refresh fontSize="small" />
            </IconButton>
          </span>
        </Tooltip>
      </div>

      <ShortcutList
        shortcuts={pinnedCollections}
        onClickHeader={() => handleExpandedList("SHORTCUTS")}
        isExpanded={expandedList === "SHORTCUTS"}
      />

      <TopicList
        topics={topics}
        addDisabled={isLibraryReadOnly}
        onAddTopic={onAddTopic}
        onClickHeader={() => handleExpandedList("TOPICS")}
        isExpanded={expandedList === "TOPICS"}
      />

      <FeedList
        feeds={feeds}
        addDisabled={isLibraryReadOnly}
        onAddFeed={onAddFeed}
        onClickHeader={() => handleExpandedList("FEEDS")}
        isExpanded={expandedList === "FEEDS"}
      />
    </div>
  );
}
