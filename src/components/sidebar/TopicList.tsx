import { Add, Feed as FeedIcon } from "@mui/icons-material";
import { IconButton } from "@mui/material";
import { useCallback } from "react";
import { useSearchParams } from "react-router-dom";
import useAbortController from "../../hooks/useAbortController";
import { useFeeds } from "../../hooks/useFeeds";
import useRouter from "../../hooks/useRouter";
import { isFilter, type Topic } from "../../types";
import ExpandableList from "./ExpandableList";
import SidebarItem from "./SidebarItem";
import css from "./TopicList.module.css";
import UnreadCount from "./UnreadCount";

type TopicListProps = {
  isExpanded?: boolean;
  topics: Topic[];
  onAddTopic: () => void;
  onClickHeader: () => void;
};

export default function TopicList({
  isExpanded = false,
  topics,
  onAddTopic,
  onClickHeader,
}: TopicListProps) {
  const { navigate, state } = useRouter();
  const { topicId } = state;
  const [searchParams] = useSearchParams();
  const filterParam = searchParams.get("filter");

  const [, abortControllerRef] = useAbortController();
  const { loadingFeedIds } = useFeeds(abortControllerRef);

  const handleSelectTopic = useCallback(
    (topic: Topic) => {
      const filter = isFilter(filterParam) ? filterParam : "unread";
      navigate(`/?filter=${filter}`, { state: { topicId: topic.id } });
    },
    [filterParam, navigate]
  );

  return (
    <ExpandableList
      label="Topics"
      isExpanded={isExpanded}
      onClickHeader={onClickHeader}
      renderHeaderAction={() => (
        <IconButton size="small" onClick={onAddTopic}>
          <Add className={css.AddIcon} />
        </IconButton>
      )}
    >
      {topics.map((topic) => (
        <SidebarItem
          collection={topic}
          key={topic.id}
          icon={FeedIcon}
          loading={loadingFeedIds.size > 0}
          selected={topicId === topic.id}
          onSelectCollection={handleSelectTopic}
          renderSecondaryAction={() => <UnreadCount count={topic.unreadCount} />}
        />
      ))}
    </ExpandableList>
  );
}
