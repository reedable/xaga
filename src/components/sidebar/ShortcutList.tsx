import {
  Add,
  Feed as FeedIcon,
  FolderOutlined,
  Newspaper,
  Language,
  RssFeed,
} from "@mui/icons-material";
import { IconButton, Tooltip } from "@mui/material";
import { useMemo, useState } from "react";
import { useLibraryReadOnly } from "../../hooks/useLibraryReadOnly";
import ShortcutTree from "./ShortcutTree";
import {
  projectPinTree,
  shortcutReference,
  type ShortcutNode,
} from "../../utils/projectPinTree";
import useAbortController from "../../hooks/useAbortController";
import { useFeeds } from "../../hooks/useFeeds";
import { useSidebarSort } from "../../hooks/useSidebarSort";
import useRouter from "../../hooks/useRouter";
import {
  isEntry,
  isFeed,
  isTopic,
  isSite,
  type Collection,
  type Countable,
  type Entry,
} from "../../types";
import sortSidebarCollections from "../../utils/sortSidebarCollections";
import ExpandableList from "./ExpandableList";
import css from "./ShortcutList.module.css";
import SidebarEntity from "./SidebarEntity";

import type { PinTarget } from "../../pins";
import MissingShortcut from "./MissingShortcut";

type ShortcutListProps = {
  missing?: PinTarget[];
  hierarchy?: ShortcutNode[];
  filter?: string;
  isExpanded?: boolean;
  shortcuts: (Collection | (Entry & Countable))[];
  onClickHeader: () => void;
};

export default function ShortcutList({
  missing = [],
  hierarchy,
  filter = "",
  isExpanded = false,
  shortcuts,
  onClickHeader,
}: ShortcutListProps) {
  const [createRequest, setCreateRequest] = useState(0);
  const readOnly = useLibraryReadOnly();
  const { selectShortcutTarget, state } = useRouter();
  const { feedId, topicId, articleScopeId, siteHostname } = state;
  const [, abortControllerRef] = useAbortController();
  const { loadingFeedIds } = useFeeds(abortControllerRef);
  const [sortMode] = useSidebarSort();
  const sortedShortcuts = useMemo(
    () => sortSidebarCollections(shortcuts, sortMode),
    [shortcuts, sortMode]
  );

  return (
    <ExpandableList
      section="shortcuts"
      className={css.ShortcutList}
      count={shortcuts.length + missing.length}
      label="Shortcuts"
      isExpanded={isExpanded}
      onClickHeader={onClickHeader}
      renderHeaderAction={() => (
        <Tooltip title="Add group">
          <span>
            <IconButton
              data-testid="add-shortcut-group"
              aria-label="Add shortcut group"
              size="small"
              disabled={readOnly}
              onClick={() => {
                setCreateRequest((value) => value + 1);
                if (!isExpanded) onClickHeader();
              }}
            >
              <Add fontSize="small" />
            </IconButton>
          </span>
        </Tooltip>
      )}
    >
      <ShortcutTree
        nodes={
          hierarchy ??
          projectPinTree(
            [...sortedShortcuts.map(shortcutReference), ...missing],
            sortedShortcuts,
            sortMode,
            () => false
          )
        }
        filter={filter}
        createRequest={createRequest}
        renderTarget={(node, unreadCount) => {
          const shortcut = node.target;
          return shortcut ? (
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
              onSelectCollection={selectShortcutTarget}
              renderSecondaryAction={
                !isEntry(shortcut) && shortcut.unreadCount > 0
                  ? () => unreadCount
                  : undefined
              }
            />
          ) : node.reference.type !== "group" ? (
            <MissingShortcut target={node.reference} />
          ) : null;
        }}
      />
    </ExpandableList>
  );
}
