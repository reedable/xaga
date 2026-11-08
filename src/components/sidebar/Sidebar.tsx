import SidebarSortControl from "./SidebarSortControl";
import { Clear, Newspaper, Refresh } from "@mui/icons-material";
import {
  IconButton,
  InputAdornment,
  List,
  TextField,
  Tooltip,
} from "@mui/material";
import { useCallback, useMemo, useRef, useState } from "react";
import { ALL_ENTITIES } from "../../Shortcuts";
import { useAllEntitiesCounts } from "../../hooks/useAllEntitiesCounts";
import useRouter from "../../hooks/useRouter";
import SidebarEntity from "./SidebarEntity";
import UnreadCount from "./UnreadCount";
import useAbortController from "../../hooks/useAbortController";
import { useFeeds } from "../../hooks/useFeeds";
import { useLibraryReadOnly } from "../../hooks/useLibraryReadOnly";
import { useSidebar } from "../../hooks/useSidebar";
import { useTopics } from "../../hooks/useTopics";
import { useSites } from "../../hooks/useSites";
import { useShortcutTargets } from "../../hooks/useShortcutTargets";
import cx from "../../utils/cx";
import FeedList from "./FeedList";
import ShortcutList from "./ShortcutList";
import css from "./Sidebar.module.css";
import TopicList from "./TopicList";
import TagList from "./TagList";
import { useTags } from "../../providers/TagsContext";
import SiteList from "./SiteList";

type SidebarProps = {
  className?: string;
  filter: string;
  onFilterChange: (filter: string) => void;
  onAddFeed: () => void;
  onAddTopic: () => void;
};

export function Sidebar({
  className,
  filter,
  onFilterChange,
  onAddFeed,
  onAddTopic,
}: SidebarProps) {
  const toolbarRef = useRef<HTMLDivElement>(null);
  const [, abortControllerRef] = useAbortController();
  const isLibraryReadOnly = useLibraryReadOnly();

  const { tags } = useTags();
  const {
    targets: pinnedTargets,
    missing: missingPins,
    hierarchy,
  } = useShortcutTargets();
  const { topics } = useTopics(abortControllerRef);

  const { loading, loadingFeedIds, feeds, refreshAllFeeds } =
    useFeeds(abortControllerRef);
  const { selectShortcutTarget, state } = useRouter();
  const counts = useAllEntitiesCounts();
  const allEntities = useMemo(
    () => (counts ? { ...ALL_ENTITIES, ...counts } : ALL_ENTITIES),
    [counts]
  );

  const { expandedList, setExpandedList } = useSidebar(abortControllerRef);
  const { sites } = useSites(abortControllerRef);
  const normalizedFilter = filter.trim().toLocaleLowerCase();
  const matchesFilter = useCallback(
    ({ title }: { title: string }) =>
      title.toLocaleLowerCase().includes(normalizedFilter),
    [normalizedFilter]
  );
  const filteredTopics = useMemo(
    () => topics.filter(matchesFilter),
    [matchesFilter, topics]
  );
  const filteredFeeds = useMemo(
    () => feeds.filter(matchesFilter),
    [feeds, matchesFilter]
  );
  const filteredSites = useMemo(
    () => sites.filter(matchesFilter),
    [matchesFilter, sites]
  );

  const handleExpandedList = useCallback(
    (listType: typeof expandedList) => {
      setExpandedList((prev) => (prev === listType ? null : listType));
    },
    [setExpandedList]
  );

  const [spin, setSpin] = useState(false);

  const handleRefreshAllFeeds = async () => {
    if (spin || loading) return;
    setSpin(true);
    try {
      await refreshAllFeeds();
    } finally {
      setSpin(false);
    }
  };

  return (
    <div className={cx(className, css.Sidebar)}>
      <div ref={toolbarRef} className={css.Buttons}>
        {isLibraryReadOnly && (
          <Tooltip title="This library is open in read-only mode">
            <span className={css.ReadOnlyBadge}>Read-only</span>
          </Tooltip>
        )}
        <SidebarSortControl
          className={css.ToolbarButton}
          menuAnchor={() => toolbarRef.current}
        />
        <Tooltip
          title={
            isLibraryReadOnly
              ? "This library is read-only"
              : spin || loading
                ? "Refresh in progress…"
                : "Refresh"
          }
        >
          {/* span keeps the Tooltip working when the button is disabled */}
          <span className={css.RefreshWrap}>
            <IconButton
              className={cx(css.ToolbarButton, spin && css.Loading)}
              size="small"
              aria-label="Refresh"
              disabled={isLibraryReadOnly || spin || loading}
              onClick={() => handleRefreshAllFeeds()}
            >
              <Refresh fontSize="small" />
            </IconButton>
          </span>
        </Tooltip>
      </div>

      <List dense disablePadding className={css.AllItems}>
        <SidebarEntity
          icon={Newspaper}
          collection={allEntities}
          loading={loadingFeedIds.size > 0}
          selected={
            !state.articleScopeId && state.shortcutId === ALL_ENTITIES.id
          }
          onSelectCollection={selectShortcutTarget}
          renderSecondaryAction={
            counts
              ? () => <UnreadCount count={counts.unreadCount} />
              : undefined
          }
        />
      </List>

      <TextField
        className={css.FilterInput}
        placeholder="Filter collections…"
        fullWidth
        size="small"
        value={filter}
        variant="standard"
        onChange={(event) => onFilterChange(event.target.value)}
        slotProps={{
          htmlInput: { "aria-label": "Filter collections" },
          input: {
            endAdornment: filter ? (
              <InputAdornment position="end">
                <Tooltip title="Clear filter">
                  <IconButton
                    aria-label="Clear collection filter"
                    edge="end"
                    size="small"
                    onClick={() => onFilterChange("")}
                  >
                    <Clear fontSize="small" />
                  </IconButton>
                </Tooltip>
              </InputAdornment>
            ) : null,
          },
        }}
        sx={{
          "& .MuiInputBase-input": {
            paddingLeft: "0px !important",
          },
        }}
      />

      <ShortcutList
        shortcuts={pinnedTargets}
        hierarchy={hierarchy}
        filter={filter}
        missing={missingPins}
        onClickHeader={() => handleExpandedList("SHORTCUTS")}
        isExpanded={expandedList === "SHORTCUTS"}
      />

      <TopicList
        topics={filteredTopics}
        addDisabled={isLibraryReadOnly}
        onAddTopic={onAddTopic}
        onClickHeader={() => handleExpandedList("TOPICS")}
        isExpanded={expandedList === "TOPICS"}
      />

      <FeedList
        feeds={filteredFeeds}
        addDisabled={isLibraryReadOnly}
        onAddFeed={onAddFeed}
        onClickHeader={() => handleExpandedList("FEEDS")}
        isExpanded={expandedList === "FEEDS"}
      />

      <SiteList
        sites={filteredSites}
        onClickHeader={() => handleExpandedList("SITES")}
        isExpanded={expandedList === "SITES"}
      />

      <TagList
        tags={tags.filter((tag) =>
          tag.name.toLowerCase().includes(normalizedFilter)
        )}
        isExpanded={expandedList === "TAGS"}
        onClickHeader={() => handleExpandedList("TAGS")}
      />

      {/*
        TODO Consider placing a filter button next to refresh at the top to
             reveal filter input. More user interaction, but more intuitive.
      */}
      {/* <TextField
        className={css.FilterInput}
        placeholder="Filter collections…"
        fullWidth
        size="small"
        value={filter}
        variant="standard"
        onChange={(event) => onFilterChange(event.target.value)}
        slotProps={{
          htmlInput: { "aria-label": "Filter collections" },
          input: {
            endAdornment: filter ? (
              <InputAdornment position="end">
                <Tooltip title="Clear filter">
                  <IconButton
                    aria-label="Clear collection filter"
                    edge="end"
                    size="small"
                    onClick={() => onFilterChange("")}
                  >
                    <Clear fontSize="small" />
                  </IconButton>
                </Tooltip>
              </InputAdornment>
            ) : null,
          },
        }}
        sx={{
          "& .MuiInputBase-input": {
            paddingLeft: "0px !important",
          },
        }}
      /> */}
    </div>
  );
}
