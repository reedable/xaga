import { Add, Feed as FeedIcon } from "@mui/icons-material";
import { IconButton, Typography } from "@mui/material";
import { useCallback } from "react";
import { useSearchParams } from "react-router-dom";
import useAbortController from "../../hooks/useAbortController";
import { useFeeds } from "../../hooks/useFeeds";
import useRouter from "../../hooks/useRouter";
import { isFilter, type Topic } from "../../types";
import CollectionList from "./CollectionList";
import CollectionListItem from "./CollectionListItem";

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
    <CollectionList
      label="Topics"
      isExpanded={isExpanded}
      onClickHeader={onClickHeader}
      renderHeaderAction={() => (
        <IconButton size="small" onClick={onAddTopic}>
          <Add sx={{ fontSize: 16 }} />
        </IconButton>
      )}
    >
      {topics.map((topic) => (
        <CollectionListItem
          collection={topic}
          key={topic.id}
          icon={FeedIcon}
          loading={loadingFeedIds.size > 0}
          selected={topicId === topic.id}
          onSelectCollection={handleSelectTopic}
          renderSecondaryAction={() =>
            topic.unreadCount > 0 ? (
              <Typography
                variant="caption"
                color="text.secondary"
                sx={{ pr: 1, userSelect: "none" }}
              >
                {topic.unreadCount}
              </Typography>
            ) : null
          }
        />
      ))}
    </CollectionList>
  );
}
