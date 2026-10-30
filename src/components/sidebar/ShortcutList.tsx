import {
  Feed as FeedIcon,
  FolderOutlined,
  Newspaper,
  RssFeed,
} from "@mui/icons-material";
import { useCallback, useMemo } from "react";
import useAbortController from "../../hooks/useAbortController";
import { useFeeds } from "../../hooks/useFeeds";
import { useSidebarSort } from "../../hooks/useSidebarSort";
import useRouter from "../../hooks/useRouter";
import { ALL_ITEMS } from "../../Shortcuts";
import { useAllItemsCounts } from "../../hooks/useAllItemsCounts";
import { isEntry, isFeed, isTopic, type Countable } from "../../types";
import sortSidebarCollections from "../../utils/sortSidebarCollections";
import ExpandableList from "./ExpandableList";
import css from "./ShortcutList.module.css";
import SidebarItem from "./SidebarItem";
import SidebarSortControl from "./SidebarSortControl";
import UnreadCount from "./UnreadCount";

type ShortcutListProps = {
  isExpanded?: boolean;
  shortcuts: Countable[];
  onClickHeader: () => void;
};

export default function ShortcutList({
  isExpanded = false,
  shortcuts,
  onClickHeader,
}: ShortcutListProps) {
  const { selectArticleScope, selectCollection, state } = useRouter();
  const { feedId, topicId, shortcutId, articleScopeId } = state;
  const [, abortControllerRef] = useAbortController();
  const { loadingFeedIds } = useFeeds(abortControllerRef);
  const [sortMode, setSortMode] = useSidebarSort("shortcuts");
  const counts = useAllItemsCounts();
  const allItems = useMemo(
    () => (counts ? { ...ALL_ITEMS, ...counts } : ALL_ITEMS),
    [counts]
  );
  const sortedShortcuts = useMemo(
    () => sortSidebarCollections(shortcuts, sortMode),
    [shortcuts, sortMode]
  );

  const handleSelectShortcut = useCallback(
    (collection: Countable) =>
      isEntry(collection)
        ? selectArticleScope(collection)
        : selectCollection(collection),
    [selectArticleScope, selectCollection]
  );

  return (
    <ExpandableList
      className={css.ShortcutList}
      count={shortcuts.length}
      label="Shortcuts"
      isExpanded={isExpanded}
      onClickHeader={onClickHeader}
    >
      {isExpanded && (
        <SidebarSortControl
          label="Shortcuts"
          sortMode={sortMode}
          onChange={setSortMode}
        />
      )}

      <SidebarItem
        icon={Newspaper}
        collection={allItems}
        loading={loadingFeedIds.size > 0}
        selected={!articleScopeId && shortcutId === ALL_ITEMS.id}
        onSelectCollection={handleSelectShortcut}
        renderSecondaryAction={
          counts ? () => <UnreadCount count={counts.unreadCount} /> : undefined
        }
      />

      {sortedShortcuts.map((shortcut) => (
        <SidebarItem
          collection={shortcut}
          key={shortcut.id}
          icon={
            isFeed(shortcut)
              ? RssFeed
              : isTopic(shortcut)
                ? FeedIcon
                : isEntry(shortcut)
                  ? Newspaper
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
                : isEntry(shortcut) && articleScopeId === shortcut.id
          }
          onSelectCollection={handleSelectShortcut}
          renderSecondaryAction={
            isEntry(shortcut)
              ? undefined
              : () => <UnreadCount count={shortcut.unreadCount} />
          }
        />
      ))}
    </ExpandableList>
  );
}
