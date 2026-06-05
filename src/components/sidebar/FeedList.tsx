import { Add, RssFeed } from "@mui/icons-material";
import { IconButton, Tooltip } from "@mui/material";
import { useCallback } from "react";
import { useSearchParams } from "react-router-dom";
import useAbortController from "../../hooks/useAbortController";
import { useFeeds } from "../../hooks/useFeeds";
import useRouter from "../../hooks/useRouter";
import { isFilter, type Feed } from "../../types";
import ExpandableList from "./ExpandableList";
import css from "./FeedList.module.css";
import SidebarItem from "./SidebarItem";
import UnreadCount from "./UnreadCount";

type FeedListProps = {
  isExpanded?: boolean;
  feeds: Feed[];
  onAddFeed: () => void;
  onClickHeader: () => void;
};

export default function FeedList({
  isExpanded = false,
  feeds,
  onAddFeed,
  onClickHeader,
}: FeedListProps) {
  const { navigate, state } = useRouter();
  const { feedId } = state;
  const [searchParams] = useSearchParams();
  const filterParam = searchParams.get("filter");

  const [, abortControllerRef] = useAbortController();
  const { loadingFeedIds } = useFeeds(abortControllerRef);

  const handleSelectFeed = useCallback(
    (feed: Feed) => {
      const filter = isFilter(filterParam) ? filterParam : "unread";
      navigate(`/?filter=${filter}`, { state: { feedId: feed.id } });
    },
    [filterParam, navigate]
  );

  return (
    <ExpandableList
      className={css.FeedList}
      label="Feeds"
      isExpanded={isExpanded}
      onClickHeader={onClickHeader}
      renderHeaderAction={() => (
        <Tooltip title="Add feed">
          <IconButton size="small" onClick={onAddFeed} aria-label="Add feed">
            <Add className={css.AddIcon} />
          </IconButton>
        </Tooltip>
      )}
    >
      {feeds.map((feed) => (
        <SidebarItem
          collection={feed}
          key={feed.id}
          icon={RssFeed}
          loading={loadingFeedIds.has(feed.id)}
          selected={feedId === feed.id}
          onSelectCollection={handleSelectFeed}
          renderSecondaryAction={() => <UnreadCount count={feed.unreadCount} />}
        />
      ))}
    </ExpandableList>
  );
}
