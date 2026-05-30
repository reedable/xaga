import {
  Feed as FeedIcon,
  FolderOutlined,
  Newspaper,
  RssFeed,
} from "@mui/icons-material";
import { useCallback } from "react";
import { useSearchParams } from "react-router-dom";
import useAbortController from "../../hooks/useAbortController";
import { useFeeds } from "../../hooks/useFeeds";
import useRouter from "../../hooks/useRouter";
import { ALL_ITEMS, SHORTCUTS } from "../../Shortcuts";
import { isFeed, isFilter, isTopic, type Collection } from "../../types";
import ExpandableList from "./ExpandableList";
import css from "./ShortcutList.module.css";
import SidebarItem from "./SidebarItem";
import UnreadCount from "./UnreadCount";

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
      const defaultFilter = isFilter(filterParam) ? filterParam : "unread";
      if (SHORTCUTS.some((s) => s.id === collection.id)) {
        navigate(`/?filter=${defaultFilter}`, {
          state: { shortcutId: collection.id },
        });
      } else if (isFeed(collection)) {
        navigate(`/?filter=${defaultFilter}`, {
          state: { feedId: collection.id },
        });
      } else if (isTopic(collection)) {
        navigate(`/?filter=${defaultFilter}`, {
          state: { topicId: collection.id },
        });
      } else {
        const filter = isFilter(filterParam) ? filterParam : "all";
        navigate(`/?filter=${filter}`, {
          state: { collectionId: collection.id },
        });
      }
    },
    [filterParam, navigate]
  );

  return (
    <ExpandableList
      className={css.ShortcutList}
      label="shortcuts"
      isExpanded={isExpanded}
      onClickHeader={onClickHeader}
    >
      <SidebarItem
        icon={Newspaper}
        collection={ALL_ITEMS}
        loading={loadingFeedIds.size > 0}
        selected={shortcutId === ALL_ITEMS.id}
        onSelectCollection={handleSelectShortcut}
      />

      {shortcuts.map((shortcut) => (
        <SidebarItem
          collection={shortcut}
          key={shortcut.id}
          icon={
            isFeed(shortcut)
              ? RssFeed
              : isTopic(shortcut)
                ? FeedIcon
                : FolderOutlined
          }
          loading={
            isFeed(shortcut)
              ? loadingFeedIds.has(shortcut.id)
              : loadingFeedIds.size > 0
          }
          selected={
            isFeed(shortcut)
              ? feedId === shortcut.id
              : isTopic(shortcut)
                ? topicId === shortcut.id
                : false
          }
          onSelectCollection={handleSelectShortcut}
          renderSecondaryAction={() => (
            <UnreadCount count={shortcut.unreadCount} />
          )}
        />
      ))}
    </ExpandableList>
  );
}
