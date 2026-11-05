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
import { ALL_ENTITIES } from "../../Shortcuts";
import { useAllEntitiesCounts } from "../../hooks/useAllEntitiesCounts";
import {
  isEntry,
  isFeed,
  isTopic,
  isSite,
  isShortcut,
  type Collection,
  type Countable,
  type Entry,
  type Shortcut,
} from "../../types";
import sortSidebarCollections from "../../utils/sortSidebarCollections";
import ExpandableList from "./ExpandableList";
import css from "./ShortcutList.module.css";
import SidebarEntity from "./SidebarEntity";
import SidebarSortControl from "./SidebarSortControl";
import UnreadCount from "./UnreadCount";

import type { PinTarget } from "../../pins";
import { pinKey } from "../../pins";
import MissingShortcut from "./MissingShortcut";

type ShortcutListProps = {
  missing?: PinTarget[];
  isExpanded?: boolean;
  shortcuts: (Collection | (Entry & Countable))[];
  onClickHeader: () => void;
};

export default function ShortcutList({
  missing = [],
  isExpanded = false,
  shortcuts,
  onClickHeader,
}: ShortcutListProps) {
  const { selectArticleScope, selectCollection, selectShortcut, state } =
    useRouter();
  const { feedId, topicId, shortcutId, articleScopeId, siteHostname } = state;
  const [, abortControllerRef] = useAbortController();
  const { loadingFeedIds } = useFeeds(abortControllerRef);
  const [sortMode, setSortMode] = useSidebarSort("shortcuts");
  const counts = useAllEntitiesCounts();
  const allEntities = useMemo(
    () => (counts ? { ...ALL_ENTITIES, ...counts } : ALL_ENTITIES),
    [counts]
  );
  const sortedShortcuts = useMemo(
    () => sortSidebarCollections(shortcuts, sortMode),
    [shortcuts, sortMode]
  );

  const handleSelectShortcut = useCallback(
    (target: Collection | (Entry & Countable) | Shortcut) =>
      isEntry(target)
        ? selectArticleScope(target)
        : isShortcut(target)
          ? selectShortcut(target)
          : selectCollection(target),
    [selectArticleScope, selectCollection, selectShortcut]
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

      <SidebarEntity
        icon={Newspaper}
        collection={allEntities}
        loading={loadingFeedIds.size > 0}
        selected={!articleScopeId && shortcutId === ALL_ENTITIES.id}
        onSelectCollection={handleSelectShortcut}
        renderSecondaryAction={
          counts ? () => <UnreadCount count={counts.unreadCount} /> : undefined
        }
      />

      {sortedShortcuts.map((shortcut) => (
        <SidebarEntity
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
              ? siteHostname === shortcut.id
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
