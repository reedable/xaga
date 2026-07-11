import { Add, Feed as FeedIcon } from "@mui/icons-material";
import { IconButton, Tooltip } from "@mui/material";
import { useCallback } from "react";
import useAbortController from "../../hooks/useAbortController";
import { useFeeds } from "../../hooks/useFeeds";
import useRouter from "../../hooks/useRouter";
import { type Topic } from "../../types";
import ExpandableList from "./ExpandableList";
import SidebarItem from "./SidebarItem";
import css from "./TopicList.module.css";
import UnreadCount from "./UnreadCount";

type TopicListProps = {
  isExpanded?: boolean;
  topics: Topic[];
  /** Disables the add affordance (read-only instance). */
  addDisabled?: boolean;
  onAddTopic: () => void;
  onClickHeader: () => void;
};

export default function TopicList({
  isExpanded = false,
  topics,
  addDisabled = false,
  onAddTopic,
  onClickHeader,
}: TopicListProps) {
  const { selectTopic, state } = useRouter();
  const { topicId } = state;
  const [, abortControllerRef] = useAbortController();
  const { loadingFeedIds } = useFeeds(abortControllerRef);

  const handleSelectTopic = useCallback(
    (topic: Topic) => selectTopic(topic),
    [selectTopic]
  );

  return (
    <ExpandableList
      className={css.TopicList}
      label="Topics"
      isExpanded={isExpanded}
      onClickHeader={onClickHeader}
      renderHeaderAction={() => (
        <Tooltip title="Add topic">
          {/* span keeps the Tooltip working when the button is disabled */}
          <span>
            <IconButton
              size="small"
              disabled={addDisabled}
              onClick={onAddTopic}
              aria-label="Add topic"
            >
              <Add className={css.AddIcon} />
            </IconButton>
          </span>
        </Tooltip>
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
          renderSecondaryAction={() => (
            <UnreadCount count={topic.unreadCount} />
          )}
        />
      ))}
    </ExpandableList>
  );
}
