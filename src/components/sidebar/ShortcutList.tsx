import {
  Feed as FeedIcon,
  FolderOutlined,
  Newspaper,
  Language,
  RssFeed,
} from "@mui/icons-material";
import { useCallback, useMemo } from "react";
import useAbortController from "../../hooks/useAbortController";
import { useFeeds } from "../../hooks/useFeeds";
import { useSidebarSort } from "../../hooks/useSidebarSort";
import useRouter from "../../hooks/useRouter";
import { ALL_ITEMS } from "../../Shortcuts";
import { useAllItemsCounts } from "../../hooks/useAllItemsCounts";
import {
  isEntry,
  isFeed,
  isTopic,
  isSite,
  type Site,
  type Countable,
} from "../../types";
import sortSidebarCollections from "../../utils/sortSidebarCollections";
import ExpandableList from "./ExpandableList";
import css from "./ShortcutList.module.css";
import SidebarItem from "./SidebarItem";
import SidebarSortControl from "./SidebarSortControl";
import UnreadCount from "./UnreadCount";

import type { PinTarget } from "../../pins";
import { pinKey } from "../../pins";
import MissingShortcut from "./MissingShortcut";

type ShortcutListProps = {
  missing?: PinTarget[];
  isExpanded?: boolean;
  shortcuts: (Countable | Site)[];
  onClickHeader: () => void;
};

export default function ShortcutList({
  missing = [],
  isExpanded = false,
  shortcuts,
  onClickHeader,
}: ShortcutListProps) {
  const { selectArticleScope, selectCollection, selectSite, state } =
    useRouter();
  const { feedId, topicId, shortcutId, articleScopeId, siteHostname } = state;
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
    (collection: Countable | Site) =>
      isSite(collection)
        ? selectSite(collection)
        : isEntry(collection)
          ? selectArticleScope(collection)
          : selectCollection(collection),
    [selectArticleScope, selectCollection, selectSite]
  );

  return (
    <ExpandableList
      className={css.ShortcutList}
      count={shortcuts.length + missing.length}
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
          key={`${isEntry(shortcut) ? "entry" : isFeed(shortcut) ? "feed" : isSite(shortcut) ? "site" : "topic"}:${shortcut.id}`}
          icon={
            isSite(shortcut)
              ? Language
              : isFeed(shortcut)
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
            isSite(shortcut)
              ? siteHostname === shortcut.hostname
              : isFeed(shortcut)
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
      {missing.map((target) => (
        <MissingShortcut key={pinKey(target)} target={target} />
      ))}
    </ExpandableList>
  );
}
