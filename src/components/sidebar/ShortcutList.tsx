import {
  Feed as FeedIcon,
  FolderOutlined,
  Newspaper,
  RssFeed,
} from "@mui/icons-material";
import { useCallback } from "react";
import useAbortController from "../../hooks/useAbortController";
import { useFeeds } from "../../hooks/useFeeds";
import useRouter from "../../hooks/useRouter";
import { ALL_ITEMS } from "../../Shortcuts";
import { isFeed, isTopic, type Collection } from "../../types";
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
  const { selectCollection, state } = useRouter();
  const { feedId, topicId, shortcutId } = state;
  const [, abortControllerRef] = useAbortController();
  const { loadingFeedIds } = useFeeds(abortControllerRef);

  const handleSelectShortcut = useCallback(
    (collection: Collection) => selectCollection(collection),
    [selectCollection]
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
