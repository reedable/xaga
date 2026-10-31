import { Clear, Refresh } from "@mui/icons-material";
import { IconButton, InputAdornment, TextField, Tooltip } from "@mui/material";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import useAbortController from "../../hooks/useAbortController";
import { useFeeds } from "../../hooks/useFeeds";
import { useLibraryReadOnly } from "../../hooks/useLibraryReadOnly";
import { useSidebar } from "../../hooks/useSidebar";
import { useTopics } from "../../hooks/useTopics";
import { useSites } from "../../hooks/useSites";
import { useMissingPins } from "../../hooks/useMissingPins";
import { usePinnedEntries } from "../../hooks/usePinnedEntries";
import cx from "../../utils/cx";
import sortByUnreadCount from "../../utils/sortByUnreadCount";
import FeedList from "./FeedList";
import ShortcutList from "./ShortcutList";
import css from "./Sidebar.module.css";
import TopicList from "./TopicList";
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
  const [, abortControllerRef] = useAbortController();
  const isLibraryReadOnly = useLibraryReadOnly();

  const pinnedEntries = usePinnedEntries();
  const { topics, pinnedTopicIds } = useTopics(abortControllerRef);

  const { loading, feeds, pinnedFeedIds, refreshAllFeeds } =
    useFeeds(abortControllerRef);

  const missingPins = useMissingPins(pinnedEntries, feeds, topics);

  const { expandedList, setExpandedList } = useSidebar(abortControllerRef);
  const { sites, pinnedSites = [] } = useSites(abortControllerRef);

  const pinnedCollections = useMemo(
    () =>
      [
        ...pinnedSites,
        // Counts here are only the Shortcuts sorting projection; articles show none.
        ...pinnedEntries.map((entry) => ({
          ...entry,
          entryCount: 0,
          unreadCount: 0,
        })),
        ...pinnedTopicIds.flatMap((id) => topics.filter((t) => t.id === id)),
        ...pinnedFeedIds.flatMap((id) => feeds.filter((f) => f.id === id)),
      ].sort(sortByUnreadCount),
    [feeds, pinnedFeedIds, pinnedTopicIds, topics, pinnedEntries, pinnedSites]
  );
  const normalizedFilter = filter.trim().toLocaleLowerCase();
  const matchesFilter = useCallback(
    ({ title }: { title: string }) =>
      title.toLocaleLowerCase().includes(normalizedFilter),
    [normalizedFilter]
  );
  const filteredPinnedCollections = useMemo(
    () => pinnedCollections.filter(matchesFilter),
    [matchesFilter, pinnedCollections]
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

  const stopRequestedRef = useRef(false);
  const iidRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const [spin, setSpin] = useState(false);

  const handleRefreshAllFeeds = () => {
    refreshAllFeeds();
    setSpin(true);

    iidRef.current = setInterval(() => {
      if (stopRequestedRef.current) {
        setSpin(false);
        clearInterval(iidRef.current);
      }
    }, 1000);
  };

  useEffect(() => {
    return () => clearTimeout(iidRef.current);
  }, []);

  useEffect(() => {
    if (!loading) {
      stopRequestedRef.current = true;
    }
  }, [loading]);

  return (
    <div className={cx(className, css.Sidebar)}>
      <div className={css.Buttons}>
        {isLibraryReadOnly && (
          <Tooltip title="This library is open in read-only mode">
            <span className={css.ReadOnlyBadge}>Read-only</span>
          </Tooltip>
        )}
        <Tooltip title="Refresh">
          {/* span keeps the Tooltip working when the button is disabled */}
          <span className={css.RefreshWrap}>
            <IconButton
              className={cx(css.RefreshButton, spin && css.Loading)}
              size="small"
              disabled={isLibraryReadOnly}
              onClick={() => handleRefreshAllFeeds()}
            >
              <Refresh fontSize="small" />
            </IconButton>
          </span>
        </Tooltip>
      </div>

      <ShortcutList
        shortcuts={filteredPinnedCollections}
        missing={missingPins.filter((pin) =>
          `${pin.type}: ${pin.type === "site" ? pin.hostname : pin.id}`
            .toLocaleLowerCase()
            .includes(normalizedFilter)
        )}
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

      <TextField
        className={css.FilterInput}
        fullWidth
        placeholder="Filter collections…"
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
      />
    </div>
  );
}
