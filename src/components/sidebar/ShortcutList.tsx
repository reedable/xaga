import {
  Feed as FeedIcon,
  Newspaper,
  RssFeed,
  Star,
} from "@mui/icons-material";
import { Typography } from "@mui/material";
import { useCallback } from "react";
import { useSearchParams } from "react-router-dom";
import useAbortController from "../../hooks/useAbortController";
import { useFeeds } from "../../hooks/useFeeds";
import useRouter from "../../hooks/useRouter";
import { ALL_ITEMS, SAVED_ITEMS, SHORTCUTS } from "../../Shortcuts";
import { isFeed, isFilter, type Collection } from "../../types";
import CollectionList from "./CollectionList";
import CollectionListItem from "./CollectionListItem";

type ShortcutListProps = {
  isExpanded?: boolean;
  shortcuts: Collection[];
  onClickHeader: () => void;
};

export default function ShortcutList({
  isExpanded = false,
  shortcuts,
  onClickHeader,
}: ShortcutListProps) {
  const { navigate, state } = useRouter();
  const { feedId, topicId, shortcutId } = state;
  const [searchParams] = useSearchParams();
  const filterParam = searchParams.get("filter");

  const [, abortControllerRef] = useAbortController();
  const { loadingFeedIds } = useFeeds(abortControllerRef);

  const handleSelectShortcut = useCallback(
    (collection: Collection) => {
      const filter = isFilter(filterParam) ? filterParam : "unread";
      const url = `/?filter=${filter}`;
      if (SHORTCUTS.some((s) => s.id === collection.id)) {
        navigate(url, { state: { shortcutId: collection.id } });
      } else if (isFeed(collection)) {
        navigate(url, { state: { feedId: collection.id } });
      } else {
        navigate(url, { state: { topicId: collection.id } });
      }
    },
    [filterParam, navigate]
  );

  return (
    <CollectionList
      label="shortcuts"
      isExpanded={isExpanded}
      onClickHeader={onClickHeader}
    >
      <CollectionListItem
        icon={Newspaper}
        collection={ALL_ITEMS}
        loading={loadingFeedIds.size > 0}
        selected={shortcutId === ALL_ITEMS.id}
        onSelectCollection={handleSelectShortcut}
      />

      <CollectionListItem
        icon={Star}
        collection={SAVED_ITEMS}
        loading={loadingFeedIds.size > 0}
        selected={shortcutId === SAVED_ITEMS.id}
        onSelectCollection={handleSelectShortcut}
      />

      {shortcuts.map((shortcut) => (
        <CollectionListItem
          collection={shortcut}
          key={shortcut.id}
          icon={isFeed(shortcut) ? RssFeed : FeedIcon}
          loading={
            isFeed(shortcut)
              ? loadingFeedIds.has(shortcut.id)
              : loadingFeedIds.size > 0
          }
          selected={
            isFeed(shortcut) ? feedId === shortcut.id : topicId === shortcut.id
          }
          onSelectCollection={handleSelectShortcut}
          renderSecondaryAction={() =>
            shortcut.unreadCount > 0 ? (
              <Typography
                variant="caption"
                color="text.secondary"
                sx={{ pr: 1, userSelect: "none" }}
              >
                {shortcut.unreadCount}
              </Typography>
            ) : null
          }
        />
      ))}
    </CollectionList>
  );
}
